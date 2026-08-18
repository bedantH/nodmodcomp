use std::{
    env,
    error::Error,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{self, ErrorKind},
    path::{Path, PathBuf},
};

fn install_directory() -> Result<PathBuf, Box<dyn Error>> {
    let home = env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "HOME is not set"))?;

    Ok(PathBuf::from(home).join(".local/bin"))
}

fn same_file(source: &Path, destination: &Path) -> io::Result<bool> {
    if !destination.try_exists()? {
        return Ok(false);
    }

    Ok(fs::canonicalize(source)? == fs::canonicalize(destination)?)
}

fn install_binary(source: &Path, destination: &Path, force: bool) -> Result<bool, Box<dyn Error>> {
    if same_file(source, destination)? {
        return Ok(false);
    }

    if destination.try_exists()? && !force {
        return Err(io::Error::new(
            ErrorKind::AlreadyExists,
            format!(
                "{} already exists; use `nodmodcomp setup --force` to replace it",
                destination.display()
            ),
        )
        .into());
    }

    let parent = destination.parent().ok_or_else(|| {
        io::Error::new(
            ErrorKind::InvalidInput,
            format!("invalid installation path: {}", destination.display()),
        )
    })?;
    fs::create_dir_all(parent)?;

    let temporary = parent.join(format!(".nodmodcomp.install.{}.tmp", std::process::id()));
    let copy_result = (|| -> Result<(), Box<dyn Error>> {
        let mut input = File::open(source)?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        io::copy(&mut input, &mut output)?;
        output.sync_all()?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&temporary, fs::Permissions::from_mode(0o755))?;
        }

        Ok(())
    })();

    if let Err(error) = copy_result {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }

    if let Err(error) = fs::rename(&temporary, destination) {
        let _ = fs::remove_file(&temporary);
        return Err(error.into());
    }

    Ok(true)
}

fn directory_is_in_path(directory: &Path) -> bool {
    env::var_os("PATH")
        .map(|path| env::split_paths(&path).any(|entry| entry == directory))
        .unwrap_or(false)
}

fn is_debug_build(executable: &Path) -> bool {
    executable
        .components()
        .collect::<Vec<_>>()
        .windows(2)
        .any(|components| {
            components[0].as_os_str() == "target" && components[1].as_os_str() == "debug"
        })
}

pub fn run(force: bool) -> Result<(), Box<dyn Error>> {
    let source = env::current_exe()?;
    let directory = install_directory()?;
    let destination = directory.join("nodmodcomp");
    let installed = install_binary(&source, &destination, force)?;

    if installed {
        println!("✔ installed nodmodcomp to {}", destination.display());
    } else {
        println!(
            "nodmodcomp is already installed at {}",
            destination.display()
        );
    }

    if !directory_is_in_path(&directory) {
        let home = env::var_os("HOME").unwrap_or_else(|| OsString::from("$HOME"));
        eprintln!(
            "warning: {} is not in PATH\n  add this to your shell configuration:\n  export PATH=\"{}/.local/bin:$PATH\"",
            directory.display(),
            PathBuf::from(home).display()
        );
    }

    if is_debug_build(&source) {
        eprintln!(
            "warning: this installed a debug build; for an optimized binary run:\n  cargo run --release -- setup --force"
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::install_binary;
    use std::{fs, process};

    #[test]
    fn installs_and_only_overwrites_with_force() {
        let directory = std::env::temp_dir().join(format!(
            "nodmodcomp-setup-test-{}-{:?}",
            process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&directory).unwrap();

        let source = directory.join("source");
        let destination = directory.join("bin/nodmodcomp");
        fs::write(&source, b"first").unwrap();

        assert!(install_binary(&source, &destination, false).unwrap());
        assert_eq!(fs::read(&destination).unwrap(), b"first");

        fs::write(&source, b"second").unwrap();
        assert!(install_binary(&source, &destination, false).is_err());
        assert!(install_binary(&source, &destination, true).unwrap());
        assert_eq!(fs::read(&destination).unwrap(), b"second");

        fs::remove_dir_all(directory).unwrap();
    }
}
