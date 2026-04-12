mod cli;
mod goodreads;
mod models;
mod openlibrary;

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process;

use chrono::Utc;
use clap::{Parser, ValueEnum};
use thiserror::Error;
use url::Url;

use cli::{Cli, Command, UpdateField};
use models::{Book, Status};

// ── Error type ──────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("OpenLibrary error: {0}")]
    OpenLibrary(#[from] openlibrary::OpenLibraryError),
    #[error("Goodreads error: {0}")]
    Goodreads(#[from] goodreads::GoodreadsError),
    #[error("book with ISBN {0} already exists")]
    AlreadyExists(String),
    #[error("book with ISBN {0} not found")]
    NotFound(String),
    #[error("invalid rating: {0} (must be 1-5)")]
    InvalidRating(u8),
    #[error("invalid rating value: '{0}'")]
    InvalidRatingValue(String),
    #[error("invalid status: '{0}' (use unread/reading/read/abandoned)")]
    InvalidStatus(String),
    #[error("HOME environment variable not set")]
    HomeNotFound,
}

type Result<T> = std::result::Result<T, AppError>;

// ── Storage ─────────────────────────────────────────────────────────────────

fn expand_tilde(path: &str) -> Result<PathBuf> {
    if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var("HOME").map_err(|_| AppError::HomeNotFound)?;
        Ok(PathBuf::from(home).join(rest))
    } else {
        Ok(PathBuf::from(path))
    }
}

type Store = BTreeMap<String, Book>;

fn load_store(path: &PathBuf) -> Result<Store> {
    let path = path.clone();
    if !path.exists() {
        return Ok(Store::new());
    }
    let data = fs::read_to_string(&path)?;
    if data.trim().is_empty() {
        return Ok(Store::new());
    }
    Ok(serde_json::from_str(&data)?)
}

fn save_store(store: &Store, path: &PathBuf) -> Result<()> {
    let path = path.clone();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(store)?;
    let tmp = path.with_extension("tmp");
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(json.as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(&tmp, &path)?;
    Ok(())
}

// ── Main ────────────────────────────────────────────────────────────────────

fn run(cli: Cli) -> Result<()> {
    let store_path = expand_tilde(&cli.store)?;
    match cli.command {
        Command::Add { input } => {
            let mut store = load_store(&store_path)?;

            // Try to parse as URL first
            let (key, book) = if let Ok(_) = Url::parse(&input) {
                // It's a URL, treat as Goodreads
                eprintln!("Fetching from Goodreads...");
                let book = goodreads::fetch_from_goodreads(&input)?;
                let key = book.goodreads_id.clone().unwrap_or_else(|| input.clone());
                (key, book)
            } else {
                // It's not a URL, treat as ISBN
                if store.contains_key(&input) {
                    return Err(AppError::AlreadyExists(input));
                }
                eprintln!("Fetching ISBN {} from OpenLibrary...", input);
                let book = openlibrary::fetch_from_openlibrary(&input)?;
                (input.clone(), book)
            };

            if store.contains_key(&key) {
                return Err(AppError::AlreadyExists(key));
            }

            let title = book.title.clone();
            store.insert(key, book);
            save_store(&store, &store_path)?;
            eprintln!("Added: {}", title);
        }
        Command::Done { isbn, rating } => {
            if let Some(r) = rating {
                if r < 1 || r > 5 {
                    return Err(AppError::InvalidRating(r));
                }
            }
            let mut store = load_store(&store_path)?;
            let book = store
                .get_mut(&isbn)
                .ok_or_else(|| AppError::NotFound(isbn.clone()))?;
            book.status = Status::Read;
            book.date_finished = Some(Utc::now());
            if let Some(r) = rating {
                book.rating = Some(r);
            }
            let title = book.title.clone();
            save_store(&store, &store_path)?;
            eprintln!("Marked as read: {}", title);
        }
        Command::Start { isbn } => {
            let mut store = load_store(&store_path)?;
            let book = store
                .get_mut(&isbn)
                .ok_or_else(|| AppError::NotFound(isbn.clone()))?;
            book.status = Status::Reading;
            book.date_started = Some(Utc::now());
            let title = book.title.clone();
            save_store(&store, &store_path)?;
            eprintln!("Started reading: {}", title);
        }
        Command::Update { isbn, field, value } => {
            let mut store = load_store(&store_path)?;
            let book = store
                .get_mut(&isbn)
                .ok_or_else(|| AppError::NotFound(isbn.clone()))?;
            match field {
                UpdateField::Rating => {
                    let r: u8 = value
                        .parse()
                        .map_err(|_| AppError::InvalidRatingValue(value.clone()))?;
                    if r < 1 || r > 5 {
                        return Err(AppError::InvalidRating(r));
                    }
                    book.rating = Some(r);
                }
                UpdateField::Status => {
                    book.status = Status::from_str(&value, true)
                        .map_err(|_| AppError::InvalidStatus(value.clone()))?;
                }
                UpdateField::Notes => {
                    book.notes.push(value);
                }
                UpdateField::PublishDate => {
                    book.publish_date = Some(value);
                }
            }
            save_store(&store, &store_path)?;
            eprintln!("Updated {} {:?}", isbn, field);
        }
        Command::Abandon { isbn } => {
            let mut store = load_store(&store_path)?;
            let book = store
                .get_mut(&isbn)
                .ok_or_else(|| AppError::NotFound(isbn.clone()))?;
            book.status = Status::Abandoned;
            let title = book.title.clone();
            save_store(&store, &store_path)?;
            eprintln!("Abandoned: {}", title);
        }
        Command::Show { isbn } => {
            let store = load_store(&store_path)?;
            let book = store
                .get(&isbn)
                .ok_or_else(|| AppError::NotFound(isbn.clone()))?;
            println!("{}", serde_json::to_string_pretty(book)?);
        }
        Command::Ls { status } => {
            let store = load_store(&store_path)?;
            for (_isbn, book) in &store {
                if let Some(ref filter) = status {
                    if *filter != book.status {
                        continue;
                    }
                }
                println!(
                    "{} - {} - {} - {}",
                    book.title, book.author, book.status, _isbn
                );
            }
        }
    }
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("error: {}", e);
        process::exit(1);
    }
}