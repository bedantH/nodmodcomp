use std::{
    error::Error,
    ffi::OsString,
    io::{Error as IoError, ErrorKind},
    path::Path,
};

fn hibernate_error(kind: ErrorKind, message: impl Into<String>) -> IoError {
    IoError::new(kind, format!("hibernation failed: {}", message.into()))
}

pub fn run(path: &OsString, dry_run: bool) -> Result<(), Box<dyn Error>> {
    let project = Path::new(path);

    if !project.try_exists()? {
        return Err(hibernate_error(
            ErrorKind::NotFound,
            format!("path not found: {}", project.display()),
        )
        .into());
    }

    if !project.is_dir() {
        return Err(hibernate_error(
            ErrorKind::NotADirectory,
            format!("path is not a directory: {}", project.display()),
        )
        .into());
    }

    let package_json = project.join("package.json");

    if !package_json.is_file() {
        return Err(hibernate_error(
            ErrorKind::NotFound,
            format!("package.json not found in {}", project.display()),
        )
        .into());
    }

    super::pack::pack_at(project, dry_run)
}
