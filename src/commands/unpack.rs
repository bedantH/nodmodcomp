use indicatif::{ProgressBar, ProgressStyle};
use std::fs::File;

use crate::metadata::{ArchiveMetadata, METADATA_NAME, read_package_json};

fn warn_on_package_drift() -> Result<(), Box<dyn std::error::Error>> {
    let metadata_path = std::path::Path::new(METADATA_NAME);

    if !metadata_path.try_exists()? {
        eprintln!("warning: {METADATA_NAME} is missing; package.json drift cannot be checked");
        return Ok(());
    }

    let metadata = match ArchiveMetadata::read(metadata_path) {
        Ok(metadata) => metadata,
        Err(error) => {
            eprintln!(
                "warning: could not read {METADATA_NAME}: {error}; package.json drift cannot be checked"
            );
            return Ok(());
        }
    };
    let package_json = std::path::Path::new("package.json");

    if !package_json.is_file() {
        eprintln!(
            "warning: package.json is missing; restored node_modules may not match this project"
        );
        return Ok(());
    }

    let current = match read_package_json(std::path::Path::new(".")) {
        Ok(package_json) => package_json,
        Err(error) => {
            eprintln!(
                "warning: could not read package.json: {error}; package.json drift cannot be checked"
            );
            return Ok(());
        }
    };
    let changed_fields = metadata.changed_package_fields(&current);

    if !changed_fields.is_empty() {
        eprintln!(
            "warning: package.json changed since node_modules was packed\n  changed fields: {}\n  restored node_modules may not match the current dependencies",
            changed_fields.join(", ")
        );
    }

    Ok(())
}

pub(crate) async fn unpack() -> Result<(), Box<dyn std::error::Error>> {
    if !std::fs::exists("node_modules.pack")? {
        return Err(
            "node_modules.pack not found. Perhaps you forgot to run `nodmodcomp pack`?".into(),
        );
    }

    if std::fs::exists("node_modules/")? {
        return Err("node_modules/ already exists. Already unpacked?".into());
    }

    warn_on_package_drift()?;

    // use file size as a proxy for progress since we don't know entry count upfront
    let pack_size = std::fs::metadata("node_modules.pack")?.len();

    let pb = ProgressBar::new(pack_size);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})\n  {wide_msg}",
        )?
        .progress_chars("█▓░"),
    );

    let pack_file = File::open("node_modules.pack")?;
    let decoder = zstd::Decoder::new(pack_file)?;
    let mut archive = tar::Archive::new(decoder);

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_string_lossy().to_string();

        pb.set_message(path);

        // update progress by how many compressed bytes have been read so far
        pb.set_position(entry.raw_file_position());

        entry.unpack_in(".")?;
    }

    pb.finish_with_message("✔ unpacked node_modules successfully");

    std::fs::remove_file("node_modules.pack")?;

    if std::fs::exists(METADATA_NAME)? {
        std::fs::remove_file(METADATA_NAME)?;
    }

    Ok(())
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    unpack().await
}
