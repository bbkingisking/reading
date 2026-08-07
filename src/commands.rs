use chrono::Utc;
use clap::ValueEnum;
use serde::Serialize;
use url::Url;

use crate::cli::{Cli, Command, UpdateField};
use crate::error::{AppError, Result};
use crate::goodreads;
use crate::models::{Book, Status};
use crate::store::{expand_tilde, load_store, save_store};

pub fn run(cli: Cli) -> Result<()> {
    let store_path = expand_tilde(&cli.store)?;
    match cli.command {
        Command::Add { query } => {
            let mut store = load_store(&store_path)?;

            let url = match Url::parse(&query) {
                Ok(_) => query,
                Err(_) => goodreads::resolve_search_url(&query)?,
            };
            let (key, book) = goodreads::fetch_from_goodreads(&url)?;

            if store.contains_key(&key) {
                return Err(AppError::AlreadyExists(key));
            }

            store.insert(key, book);
            save_store(&store, &store_path)?;
        }
        Command::Done { id, rating } => {
            if let Some(r) = rating
                && !(1..=5).contains(&r)
            {
                return Err(AppError::InvalidRating(r));
            }
            let mut store = load_store(&store_path)?;
            let book = store
                .get_mut(&id)
                .ok_or_else(|| AppError::NotFound(id.clone()))?;
            book.status = Status::Read;
            book.date_finished = Some(Utc::now());
            if let Some(r) = rating {
                book.rating = Some(r);
            }
            save_store(&store, &store_path)?;
        }
        Command::Start { id } => {
            let mut store = load_store(&store_path)?;
            let book = store
                .get_mut(&id)
                .ok_or_else(|| AppError::NotFound(id.clone()))?;
            book.status = Status::Reading;
            book.date_started = Some(Utc::now());
            save_store(&store, &store_path)?;
        }
        Command::Update { id, field, value } => {
            let mut store = load_store(&store_path)?;
            let book = store
                .get_mut(&id)
                .ok_or_else(|| AppError::NotFound(id.clone()))?;
            match field {
                UpdateField::Rating => {
                    let r: u8 = value
                        .parse()
                        .map_err(|_| AppError::InvalidRatingValue(value.clone()))?;
                    if !(1..=5).contains(&r) {
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
                UpdateField::Title => {
                    book.title = value;
                }
                UpdateField::Author => {
                    book.author = value;
                }
                UpdateField::PublishDate => {
                    book.publish_date = Some(value);
                }
            }
            save_store(&store, &store_path)?;
        }
        Command::Abandon { id } => {
            let mut store = load_store(&store_path)?;
            let book = store
                .get_mut(&id)
                .ok_or_else(|| AppError::NotFound(id.clone()))?;
            book.status = Status::Abandoned;
            save_store(&store, &store_path)?;
        }
        Command::Show { id } => {
            let store = load_store(&store_path)?;
            let book = store
                .get(&id)
                .ok_or_else(|| AppError::NotFound(id.clone()))?;

            #[derive(Serialize)]
            struct BookView<'a> {
                id: &'a str,
                #[serde(flatten)]
                book: &'a Book,
            }

            println!(
                "{}",
                serde_json::to_string_pretty(&BookView { id: &id, book })?
            );
        }
        Command::Share { id } => {
            let store = load_store(&store_path)?;
            if !store.contains_key(&id) {
                return Err(AppError::NotFound(id));
            }
            println!("https://www.goodreads.com/book/show/{}", id);
        }
        Command::Ls { status } => {
            let store = load_store(&store_path)?;
            let mut books: Vec<(&String, &Book)> = store
                .iter()
                .filter(|(_, book)| match &status {
                    Some(filter) => *filter == book.status,
                    None => true,
                })
                .collect();
            books.sort_by(|a, b| b.1.date_added.cmp(&a.1.date_added));
            for (id, book) in books {
                println!("{} - {} - {} - {}", book.title, book.author, book.status, id);
            }
        }
    }
    Ok(())
}
