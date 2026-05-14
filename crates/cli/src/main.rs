use std::path::PathBuf;

use anyhow::Result;
use clap::{ArgAction, Parser, Subcommand};
use pastedev_core::{SnippetType, Visibility};

mod client;
mod commands;
mod credentials;
mod mcp;
mod output;

use output::Format;

#[derive(Parser, Debug)]
#[command(
    name = "pastedev-cli",
    version,
    about = "terminal client for paste.dev.su",
    arg_required_else_help = true,
)]
struct Cli {
    #[arg(long, global = true, help = "machine-readable output", action = ArgAction::SetTrue)]
    json: bool,

    #[arg(long, global = true, help = "override the server URL for this invocation")]
    base_url: Option<String>,

    #[arg(long, global = true, help = "override key from environment / config")]
    token: Option<String>,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Store an API key + base URL.
    Auth {
        /// Optional override; defaults to PASTEDEV_BASE_URL.
        #[arg(long)]
        base_url: Option<String>,
        token: String,
    },
    /// Print current identity.
    Whoami,
    /// Create a snippet from stdin or a file.
    #[command(alias = "put")]
    Publish {
        /// Path to read. If omitted, reads stdin.
        file: Option<PathBuf>,
        /// Force snippet type (overrides extension inference).
        #[arg(long = "type", value_enum)]
        kind: Option<KindArg>,
        /// Display filename.
        #[arg(long)]
        name: Option<String>,
        /// Snippet visibility — public (default) or private (auth required to view).
        #[arg(long, value_enum)]
        visibility: Option<VisibilityArg>,
        /// Lifetime from creation. Accepts `15m`, `2h`, `1d`, `1w`, or seconds.
        #[arg(long = "lifetime")]
        lifetime: Option<String>,
        /// Burn the snippet 15 min after the first non-owner view.
        #[arg(long)]
        burn_after_read: bool,
    },
    /// List your snippets.
    #[command(alias = "ls")]
    List {
        /// Filter by type.
        #[arg(long = "type", value_enum)]
        kind: Option<KindArg>,
        /// Page size (default 50, max 200).
        #[arg(long)]
        limit: Option<u32>,
    },
    /// Fetch a snippet body by slug. Body goes to stdout, metadata to stderr.
    #[command(alias = "cat")]
    Get {
        slug: String,
        /// Also print metadata block to stderr.
        #[arg(long)]
        meta: bool,
    },
    /// Remove a snippet.
    #[command(alias = "rm")]
    Delete {
        slug: String,
        /// Skip the confirmation prompt.
        #[arg(short, long)]
        yes: bool,
    },
    /// Update an existing snippet's sharing policy (visibility / expiry /
    /// burn-after-read). At least one option must be passed.
    Settings {
        slug: String,
        /// Set visibility.
        #[arg(long, value_enum)]
        visibility: Option<VisibilityArg>,
        /// Set lifetime from now. Accepts `15m`, `2h`, `1d`, `1w`, or seconds.
        #[arg(long = "lifetime", conflicts_with = "no_lifetime")]
        lifetime: Option<String>,
        /// Clear the lifetime — snippet never expires (until burn-after-read fires).
        #[arg(long = "no-lifetime")]
        no_lifetime: bool,
        /// Enable burn-after-read.
        #[arg(long = "burn-after-read", conflicts_with = "no_burn_after_read")]
        burn_after_read: bool,
        /// Disable burn-after-read and clear any armed timer.
        #[arg(long = "no-burn-after-read")]
        no_burn_after_read: bool,
    },
    /// Run as an MCP server over stdio.
    Mcp,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum KindArg {
    Code,
    Markdown,
    Html,
}

impl From<KindArg> for SnippetType {
    fn from(value: KindArg) -> Self {
        match value {
            KindArg::Code => SnippetType::Code,
            KindArg::Markdown => SnippetType::Markdown,
            KindArg::Html => SnippetType::Html,
        }
    }
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum VisibilityArg {
    Public,
    Private,
}

impl From<VisibilityArg> for Visibility {
    fn from(value: VisibilityArg) -> Self {
        match value {
            VisibilityArg::Public => Visibility::Public,
            VisibilityArg::Private => Visibility::Private,
        }
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let format = if cli.json { Format::Json } else { Format::Human };
    let result: Result<()> = match cli.cmd {
        Cmd::Auth { base_url, token } => {
            commands::auth::run(commands::auth::Args {
                token: &token,
                base_url_flag: base_url.as_deref().or(cli.base_url.as_deref()),
            })
            .await
        }
        Cmd::Whoami => {
            commands::whoami::run(commands::whoami::Args {
                format,
                token: cli.token.as_deref(),
                base_url: cli.base_url.as_deref(),
            })
            .await
        }
        Cmd::Publish {
            file,
            kind,
            name,
            visibility,
            lifetime,
            burn_after_read,
        } => {
            let lifetime_seconds = match lifetime.as_deref() {
                Some(s) => match commands::publish::parse_duration(s) {
                    Ok(n) => Some(n),
                    Err(e) => {
                        eprintln!("error: {e:#}");
                        std::process::exit(2);
                    }
                },
                None => None,
            };
            commands::publish::run(commands::publish::Args {
                format,
                token: cli.token.as_deref(),
                base_url: cli.base_url.as_deref(),
                kind: kind.map(Into::into),
                name,
                file,
                visibility: visibility.map(Into::into),
                lifetime_seconds,
                burn_after_read,
            })
            .await
        }
        Cmd::List { kind, limit } => {
            commands::list::run(commands::list::Args {
                format,
                token: cli.token.as_deref(),
                base_url: cli.base_url.as_deref(),
                kind: kind.map(Into::into),
                limit,
            })
            .await
        }
        Cmd::Get { slug, meta } => {
            commands::get::run(commands::get::Args {
                format,
                token: cli.token.as_deref(),
                base_url: cli.base_url.as_deref(),
                slug: &slug,
                meta,
            })
            .await
        }
        Cmd::Delete { slug, yes } => {
            commands::delete::run(commands::delete::Args {
                token: cli.token.as_deref(),
                base_url: cli.base_url.as_deref(),
                slug: &slug,
                yes,
            })
            .await
        }
        Cmd::Settings {
            slug,
            visibility,
            lifetime,
            no_lifetime,
            burn_after_read,
            no_burn_after_read,
        } => {
            // Collapse the `--lifetime` / `--no-lifetime` pair into the
            // three-state `Option<Option<i32>>` that matches the wire.
            // clap's `conflicts_with` rules out the "both" case.
            let lifetime_seconds = match (lifetime.as_deref(), no_lifetime) {
                (Some(s), false) => match commands::publish::parse_duration(s) {
                    Ok(n) => Some(Some(n)),
                    Err(e) => {
                        eprintln!("error: {e:#}");
                        std::process::exit(2);
                    }
                },
                (None, true) => Some(None),
                (None, false) => None,
                (Some(_), true) => unreachable!("clap enforces conflicts_with"),
            };
            let burn = match (burn_after_read, no_burn_after_read) {
                (true, false) => Some(true),
                (false, true) => Some(false),
                (false, false) => None,
                (true, true) => unreachable!("clap enforces conflicts_with"),
            };
            commands::settings::run(commands::settings::Args {
                format,
                token: cli.token.as_deref(),
                base_url: cli.base_url.as_deref(),
                slug: &slug,
                visibility: visibility.map(Into::into),
                lifetime_seconds,
                burn_after_read: burn,
            })
            .await
        }
        Cmd::Mcp => mcp::run().await,
    };
    if let Err(e) = result {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}
