use std::{
    error::Error,
    fs::{self, OpenOptions},
    io::{Error as IoError, ErrorKind},
    path::Path,
    time::Instant,
};

use indicatif::{ProgressBar, ProgressStyle};
use walkdir::WalkDir;

use crate::metadata::{ArchiveMetadata, METADATA_NAME, TEMP_METADATA_NAME};

const ARCHIVE_NAME: &str = "node_modules.pack";
const TEMP_ARCHIVE_NAME: &str = "node_modules.temp.pack";

fn pack_error(kind: ErrorKind, message: impl Into<String>) -> IoError {
    IoError::new(kind, format!("packing failed: {}", message.into()))
}

fn format_bytes(bytes: u64) -> String {
    const MB: u64 = 1024 * 1024;
    const KB: u64 = 1024;

    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

fn node_modules_stats(node_modules: &Path) -> Result<(u64, u64), Box<dyn Error>> {
    let mut entries = 0;
    let mut size = 0;

    for entry in WalkDir::new(node_modules) {
        let entry = entry?;
        entries += 1;

        if entry.file_type().is_file() {
            size += entry.metadata()?.len();
        }
    }

    Ok((entries, size))
}

fn create_archive(
    project: &Path,
    node_modules: &Path,
    temporary_archive: &Path,
    total_entries: u64,
) -> Result<(), Box<dyn Error>> {
    let pack_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temporary_archive)?;
    let encoder = zstd::Encoder::new(pack_file, 3)?;
    let mut tar_builder = tar::Builder::new(encoder);
    tar_builder.follow_symlinks(false);

    let progress = ProgressBar::new(total_entries);
    progress.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} files ({eta})\n  {wide_msg}",
        )?
        .progress_chars("█▓░"),
    );

    for entry in WalkDir::new(node_modules) {
        let entry = entry?;
        let source = entry.path();
        let archive_path = source.strip_prefix(project)?;

        progress.set_message(archive_path.to_string_lossy().into_owned());

        if source.is_symlink() || source.is_file() {
            tar_builder.append_path_with_name(source, archive_path)?;
        } else if source.is_dir() {
            tar_builder.append_dir(archive_path, source)?;
        }

        progress.inc(1);
    }

    progress.finish_with_message("finalizing archive...");

    let encoder = tar_builder.into_inner()?;
    encoder.finish()?;

    Ok(())
}

pub(crate) fn pack_at(project: &Path, dry_run: bool) -> Result<(), Box<dyn Error>> {
    let node_modules = project.join("node_modules");
    let archive = project.join(ARCHIVE_NAME);
    let temporary_archive = project.join(TEMP_ARCHIVE_NAME);
    let metadata_path = project.join(METADATA_NAME);
    let temporary_metadata = project.join(TEMP_METADATA_NAME);

    if !node_modules.is_dir() {
        return Err(pack_error(
            ErrorKind::NotFound,
            format!("node_modules not found in {}", project.display()),
        )
        .into());
    }

    if archive.try_exists()? {
        return Err(pack_error(
            ErrorKind::AlreadyExists,
            format!("archive already exists: {}", archive.display()),
        )
        .into());
    }

    if temporary_archive.try_exists()? {
        return Err(pack_error(
            ErrorKind::AlreadyExists,
            format!(
                "temporary archive already exists: {}; remove it if no packing is running",
                temporary_archive.display()
            ),
        )
        .into());
    }

    if metadata_path.try_exists()? {
        return Err(pack_error(
            ErrorKind::AlreadyExists,
            format!(
                "metadata sidecar already exists: {}",
                metadata_path.display()
            ),
        )
        .into());
    }

    if temporary_metadata.try_exists()? {
        return Err(pack_error(
            ErrorKind::AlreadyExists,
            format!(
                "temporary metadata already exists: {}; remove it if no packing is running",
                temporary_metadata.display()
            ),
        )
        .into());
    }

    let metadata = ArchiveMetadata::capture(project)?;
    let (total_entries, original_size) = node_modules_stats(&node_modules)?;

    if dry_run {
        println!(
            "dry run: would pack {}\n  entries: {total_entries}\n  size: {}\n  archive: {}\n  metadata: {}",
            node_modules.display(),
            format_bytes(original_size),
            archive.display(),
            metadata_path.display(),
        );
        return Ok(());
    }

    let started_at = Instant::now();
    let archive_result = (|| -> Result<(), Box<dyn Error>> {
        create_archive(project, &node_modules, &temporary_archive, total_entries)?;
        metadata.write_new(&temporary_metadata)?;
        Ok(())
    })();

    if let Err(error) = archive_result {
        let _ = fs::remove_file(&temporary_archive);
        let _ = fs::remove_file(&temporary_metadata);
        return Err(error);
    }

    if let Err(error) = fs::rename(&temporary_archive, &archive) {
        let _ = fs::remove_file(&temporary_archive);
        let _ = fs::remove_file(&temporary_metadata);
        return Err(error.into());
    }

    if let Err(error) = fs::rename(&temporary_metadata, &metadata_path) {
        let _ = fs::remove_file(&archive);
        let _ = fs::remove_file(&temporary_metadata);
        return Err(error.into());
    }

    fs::remove_dir_all(&node_modules)?;

    let packed_size = fs::metadata(&archive)?.len();

    println!(
        "\n✔ packed successfully\n  project: {}\n  {} → {}  (saved {})\n  archive: {}\n  metadata: {}\n  done in {:.2}s",
        project.display(),
        format_bytes(original_size),
        format_bytes(packed_size),
        format_bytes(original_size.saturating_sub(packed_size)),
        archive.display(),
        metadata_path.display(),
        started_at.elapsed().as_secs_f64(),
    );

    Ok(())
}

pub async fn run() -> Result<(), Box<dyn Error>> {
    pack_at(Path::new("."), false)
}
