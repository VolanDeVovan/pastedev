use std::path::PathBuf;

use anyhow::Result;
use clap::{ArgAction, Parser, Subcommand};
use paste_core::SnippetType;

mod client;
mod commands;
mod credentials;
mod mcp;
mod output;

use output::Format;

#[derive(Parser, Debug)]
#[command(
    name = "paste-cli",
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
        /// Optional override; defaults to PASTE_BASE_URL.
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
        Cmd::Publish { file, kind, name } => {
            commands::publish::run(commands::publish::Args {
                format,
                token: cli.token.as_deref(),
                base_url: cli.base_url.as_deref(),
                kind: kind.map(Into::into),
                name,
                file,
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
        Cmd::Mcp => mcp::run().await,
    };
    if let Err(e) = result {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}
