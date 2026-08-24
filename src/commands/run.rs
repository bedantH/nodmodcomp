use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::ExitStatus;

async fn run_command(command: &[OsString]) -> Result<ExitStatus, Box<dyn std::error::Error>> {
    let (program, arguments) = command.split_first().ok_or("no command was provided")?;
    let mut child = tokio::process::Command::new(program)
        .args(arguments)
        .spawn()
        .map_err(|error| command_error(program, error))?;

    tokio::select! {
        status = child.wait() => Ok(status?),
        interrupt = tokio::signal::ctrl_c() => {
            interrupt?;
            Ok(child.wait().await?)
        }
    }
}

fn command_error(program: &OsStr, error: std::io::Error) -> String {
    format!("failed to execute `{}`: {error}", program.to_string_lossy())
}

pub async fn run(command: &[OsString]) -> Result<ExitStatus, Box<dyn std::error::Error>> {
    let node_modules_exists = std::fs::exists("node_modules/")?;
    let archive_exists = std::fs::exists("node_modules.pack")?;
    let unpacked_for_command = !node_modules_exists && archive_exists;

    if unpacked_for_command {
        println!("node_modules is packed; restoring it before running the command...");
        super::unpack::unpack().await?;
    }

    let command_result = run_command(command).await;

    if !unpacked_for_command {
        if node_modules_exists {
            println!("node_modules was already unpacked; skipping repack.");
        } else {
            println!("node_modules was not restored by nodmodcomp; skipping repack.");
        }

        return command_result;
    }

    println!("command finished; packing node_modules...");
    let pack_result = super::pack::pack_at(Path::new("."), false);

    match (command_result, pack_result) {
        (Ok(status), Ok(())) => Ok(status),
        (Err(command_error), Ok(())) => Err(command_error),
        (Ok(_), Err(pack_error)) => Err(pack_error),
        (Err(command_error), Err(pack_error)) => Err(format!(
            "{command_error}; additionally failed to repack node_modules: {pack_error}"
        )
        .into()),
    }
}
