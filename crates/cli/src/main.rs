//! `paste-cli` — terminal client for paste.dev.su. Phase 6 fills in the
//! subcommands; for now the binary just prints the planned surface so the
//! workspace builds end-to-end.

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "paste-cli", version, about = "terminal client for paste.dev.su")]
struct Cli {}

fn main() {
    let _ = Cli::parse();
    eprintln!("paste-cli {} — subcommands land in phase 6.", env!("CARGO_PKG_VERSION"));
}
