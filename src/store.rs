use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

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

pub type Store = Vec<Book>;

pub fn load_store(path: &Path) -> Result<Store> {
    if !path.exists() {
        return Ok(Store::new());
    }
    let data = fs::read_to_string(path)?;
    if data.trim().is_empty() {
        return Ok(Store::new());
    }
    let books: Store = serde_saphyr::from_str(&data)?;
    let mut ids = HashSet::new();
    for book in &books {
        if book.id.is_empty() {
            return Err(AppError::MissingBookId);
        }
        if !ids.insert(&book.id) {
            return Err(AppError::AlreadyExists(book.id.clone()));
        }
    }
    Ok(books)
}

pub fn save_store(store: &Store, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut entries: Vec<&Book> = store.iter().collect();
    entries.sort_by(|a, b| b.date_added.cmp(&a.date_added).then(a.id.cmp(&b.id)));
    let yaml = serde_saphyr::to_string(&entries)?;
    let tmp = path.with_extension("tmp");
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(yaml.as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}
