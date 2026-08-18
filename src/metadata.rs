use std::{
    collections::BTreeSet,
    error::Error,
    fs::{File, OpenOptions},
    io::{Error as IoError, ErrorKind},
    path::Path,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const METADATA_NAME: &str = "node_modules.pack.meta.json";
pub const TEMP_METADATA_NAME: &str = "node_modules.temp.pack.meta.json";

const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Deserialize, Serialize)]
pub struct ArchiveMetadata {
    format_version: u32,
    package_json: Value,
}

impl ArchiveMetadata {
    pub fn capture(project: &Path) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            format_version: FORMAT_VERSION,
            package_json: read_package_json(project)?,
        })
    }

    pub fn read(path: &Path) -> Result<Self, Box<dyn Error>> {
        let metadata: Self = serde_json::from_reader(File::open(path)?)?;

        if metadata.format_version != FORMAT_VERSION {
            return Err(IoError::new(
                ErrorKind::InvalidData,
                format!(
                    "unsupported archive metadata version: {}",
                    metadata.format_version
                ),
            )
            .into());
        }

        Ok(metadata)
    }

    pub fn write_new(&self, path: &Path) -> Result<(), Box<dyn Error>> {
        let file = OpenOptions::new().write(true).create_new(true).open(path)?;
        serde_json::to_writer_pretty(file, self)?;
        Ok(())
    }

    pub fn changed_package_fields(&self, current: &Value) -> Vec<String> {
        match (&self.package_json, current) {
            (Value::Object(snapshot), Value::Object(current)) => snapshot
                .keys()
                .chain(current.keys())
                .cloned()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .filter(|key| snapshot.get(key) != current.get(key))
                .collect(),
            _ if self.package_json == *current => Vec::new(),
            _ => vec!["package.json".to_owned()],
        }
    }
}

pub fn read_package_json(project: &Path) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_reader(File::open(
        project.join("package.json"),
    )?)?)
}
