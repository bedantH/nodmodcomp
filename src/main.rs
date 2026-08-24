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
    pub async fn run(
        &self,
    ) -> Result<Option<std::process::ExitStatus>, Box<dyn std::error::Error>> {
        match &self.command {
            Commands::Pack => commands::pack::run().await.map(|()| None),
            Commands::Unpack => commands::unpack::run().await.map(|()| None),
            Commands::Run { command } => commands::run::run(command).await.map(Some),
            Commands::Hibernate { path, dry_run } => {
                commands::hibernate::run(path, *dry_run).map(|()| None)
            }
            Commands::Setup { force } => commands::setup::run(*force).map(|()| None),
        }
    }
}

fn exit_code(status: std::process::ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }

    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;

        status.signal().map_or(1, |signal| 128 + signal)
    }

    #[cfg(not(unix))]
    1
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = NodModCompCLI::parse();

    if let Some(status) = cli.run().await?
        && !status.success()
    {
        std::process::exit(exit_code(status));
    }

    Ok(())
}
