use std::ffi::{OsStr, OsString};
use std::process::Command;

fn run_command(command: &[OsString]) -> Result<(), Box<dyn std::error::Error>> {
    let (program, arguments) = command.split_first().ok_or("no command was provided")?;

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        let error = Command::new(program).args(arguments).exec();
        Err(command_error(program, error).into())
    }
}

fn command_error(program: &OsStr, error: std::io::Error) -> String {
    format!("failed to execute `{}`: {error}", program.to_string_lossy())
}

pub async fn run(command: &[OsString]) -> Result<(), Box<dyn std::error::Error>> {
    let node_modules_exists = std::fs::exists("node_modules/")?;
    let archive_exists = std::fs::exists("node_modules.pack")?;

    if !node_modules_exists && archive_exists {
        println!("node_modules is packed; restoring it before running the command...");
        super::unpack::unpack().await?;
    }

    run_command(command)
}
