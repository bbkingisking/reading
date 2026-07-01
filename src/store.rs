use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::ser::SerializeMap;
use serde::Serialize;

use crate::error::{AppError, Result};
use crate::models::Book;

pub fn expand_tilde(path: &str) -> Result<PathBuf> {
    if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var("HOME").map_err(|_| AppError::HomeNotFound)?;
        Ok(PathBuf::from(home).join(rest))
    } else {
        Ok(PathBuf::from(path))
    }
}

pub type Store = HashMap<String, Book>;

pub fn load_store(path: &Path) -> Result<Store> {
    if !path.exists() {
        return Ok(Store::new());
    }
    let data = fs::read_to_string(path)?;
    if data.trim().is_empty() {
        return Ok(Store::new());
    }
    Ok(serde_json::from_str(&data)?)
}

struct SortedEntries<'a>(Vec<(&'a String, &'a Book)>);

impl<'a> Serialize for SortedEntries<'a> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (k, v) in &self.0 {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}

pub fn save_store(store: &Store, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut entries: Vec<(&String, &Book)> = store.iter().collect();
    entries.sort_by(|a, b| b.1.date_added.cmp(&a.1.date_added));
    let json = serde_json::to_string_pretty(&SortedEntries(entries))?;
    let tmp = path.with_extension("tmp");
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(json.as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}
