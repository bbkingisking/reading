use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process;

use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ── Error type ──────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] ureq::Error),
    #[error("book with ISBN {0} already exists")]
    AlreadyExists(String),
    #[error("book with ISBN {0} not found")]
    NotFound(String),
    #[error("ISBN {0} not found in OpenLibrary")]
    IsbnNotFound(String),
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

// ── Data model ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ValueEnum)]
enum Status {
    Unread,
    Reading,
    Read,
    Abandoned,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Unread => write!(f, "Unread"),
            Status::Reading => write!(f, "Reading"),
            Status::Read => write!(f, "Read"),
            Status::Abandoned => write!(f, "Abandoned"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Book {
    title: String,
    author: String,
    pages: Option<u32>,
    publish_date: Option<String>,
    subjects: Option<Vec<String>>,
    goodreads_id: Option<String>,
    status: Status,
    date_added: DateTime<Utc>,
    date_started: Option<DateTime<Utc>>,
    date_finished: Option<DateTime<Utc>>,
    rating: Option<u8>,
    notes: Vec<String>,
}

// ── OpenLibrary response types ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct OlResponse {
    details: OlDetails,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct OlDetails {
    title: Option<String>,
    authors: Option<Vec<OlAuthor>>,
    number_of_pages: Option<u32>,
    publish_date: Option<String>,
    subjects: Option<Vec<String>>,
    identifiers: Option<OlIdentifiers>,
}

#[derive(Debug, Deserialize)]
struct OlAuthor {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OlIdentifiers {
    goodreads: Option<Vec<String>>,
}

impl Default for OlDetails {
    fn default() -> Self {
        OlDetails {
            title: None,
            authors: None,
            number_of_pages: None,
            publish_date: None,
            subjects: None,
            identifiers: None,
        }
    }
}

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

// ── OpenLibrary fetch ───────────────────────────────────────────────────────

fn fetch_from_openlibrary(isbn: &str) -> Result<Book> {
    let url = format!(
        "https://openlibrary.org/api/books?bibkeys=ISBN:{}&format=json&jscmd=details",
        isbn
    );
    let body = ureq::get(&url).call()?.into_string()?;
    let resp: BTreeMap<String, OlResponse> = serde_json::from_str(&body)?;
    let key = format!("ISBN:{}", isbn);
    let ol = resp.get(&key).ok_or_else(|| AppError::IsbnNotFound(isbn.to_string()))?;
    let d = &ol.details;

    let title = d.title.clone().unwrap_or_else(|| "Unknown".into());
    let author = d
        .authors
        .as_ref()
        .and_then(|a: &Vec<OlAuthor>| a.first())
        .and_then(|a| a.name.clone())
        .unwrap_or_else(|| "Unknown".into());
    let goodreads_id = d
        .identifiers
        .as_ref()
        .and_then(|ids| ids.goodreads.as_ref())
        .and_then(|g: &Vec<String>| g.first())
        .cloned();

    Ok(Book {
        title,
        author,
        pages: d.number_of_pages,
        publish_date: d.publish_date.clone(),
        subjects: d.subjects.clone(),
        goodreads_id,
        status: Status::Unread,
        date_added: Utc::now(),
        date_started: None,
        date_finished: None,
        rating: None,
        notes: Vec::new(),
    })
}

// ── Update field enum ───────────────────────────────────────────────────────

#[derive(Debug, Clone, ValueEnum)]
enum UpdateField {
    Rating,
    Status,
    Notes,
    #[value(name = "publish-date")]
    PublishDate,
}

// ── CLI ─────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "reading", about = "Personal reading tracker")]
struct Cli {
    /// Path to the JSON store file
    #[arg(long, short, default_value = "~/.reading/books.json")]
    store: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Add a book by ISBN (fetches metadata from OpenLibrary)
    Add {
        /// ISBN-10 or ISBN-13
        isbn: String,
    },
    /// Mark a book as finished
    Done {
        /// ISBN of the book
        isbn: String,
        /// Optional rating (1-5)
        #[arg(long)]
        rating: Option<u8>,
    },
    /// Mark a book as currently reading
    Start {
        /// ISBN of the book
        isbn: String,
    },
    /// Update a field on a book
    Update {
        /// ISBN of the book
        isbn: String,
        /// Which field to update
        field: UpdateField,
        /// New value
        value: String,
    },
    /// Mark a book as abandoned
    Abandon {
        /// ISBN of the book
        isbn: String,
    },
    /// Pretty-print a book's details
    Show {
        /// ISBN of the book
        isbn: String,
    },
    /// List books
    #[command(alias = "list")]
    Ls {
        /// Filter by status
        #[arg(long)]
        status: Option<Status>,
    },
}

// ── Main ────────────────────────────────────────────────────────────────────

fn run(cli: Cli) -> Result<()> {
    let store_path = expand_tilde(&cli.store)?;
    match cli.command {
        Command::Add { isbn } => {
            let mut store = load_store(&store_path)?;
            if store.contains_key(&isbn) {
                return Err(AppError::AlreadyExists(isbn));
            }
            eprintln!("Fetching ISBN {} from OpenLibrary...", isbn);
            let book = fetch_from_openlibrary(&isbn)?;
            store.insert(isbn.clone(), book);
            save_store(&store, &store_path)?;
            eprintln!("Added: {}", store[&isbn].title);
        }
        Command::Done { isbn, rating } => {
            if let Some(r) = rating {
                if r < 1 || r > 5 {
                    return Err(AppError::InvalidRating(r));
                }
            }
            let mut store = load_store(&store_path)?;
            let book = store.get_mut(&isbn).ok_or_else(|| AppError::NotFound(isbn.clone()))?;
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
            let book = store.get_mut(&isbn).ok_or_else(|| AppError::NotFound(isbn.clone()))?;
            book.status = Status::Reading;
            book.date_started = Some(Utc::now());
            let title = book.title.clone();
            save_store(&store, &store_path)?;
            eprintln!("Started reading: {}", title);
        }
        Command::Update { isbn, field, value } => {
            let mut store = load_store(&store_path)?;
            let book = store.get_mut(&isbn).ok_or_else(|| AppError::NotFound(isbn.clone()))?;
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
            let book = store.get_mut(&isbn).ok_or_else(|| AppError::NotFound(isbn.clone()))?;
            book.status = Status::Abandoned;
            let title = book.title.clone();
            save_store(&store, &store_path)?;
            eprintln!("Abandoned: {}", title);
        }
        Command::Show { isbn } => {
            let store = load_store(&store_path)?;
            let book = store.get(&isbn).ok_or_else(|| AppError::NotFound(isbn.clone()))?;
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
