use std::ffi::OsString;

use clap::{Parser, Subcommand, error::Result};

pub mod commands;
mod metadata;

#[derive(Parser)]
#[command(name = "nodmodcomp", version, author = "bedantH")]
pub struct NodModCompCLI {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[clap(name = "pack")]
    Pack,

    #[clap(name = "unpack")]
    Unpack,

    #[command(name = "run", trailing_var_arg = true)]
    Run {
        /// Command and arguments to run after restoring node_modules if needed
        #[arg(required = true, allow_hyphen_values = true)]
        command: Vec<OsString>,
    },

    #[command(name = "hibernate")]
    Hibernate {
        #[arg(required = true)]
        path: OsString,

        #[arg(long)]
        dry_run: bool,
    },

    /// Install nodmodcomp into ~/.local/bin
    #[command(name = "setup")]
    Setup {
        /// Replace an existing nodmodcomp installation
        #[arg(long)]
        force: bool,
    },
}

impl NodModCompCLI {
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        match &self.command {
            Commands::Pack => commands::pack::run().await,
            Commands::Unpack => commands::unpack::run().await,
            Commands::Run { command } => commands::run::run(command).await,
            Commands::Hibernate { path, dry_run } => commands::hibernate::run(path, *dry_run),
            Commands::Setup { force } => commands::setup::run(*force),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = NodModCompCLI::parse();
    cli.run().await?;

    Ok(())
}
