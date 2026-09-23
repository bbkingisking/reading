use chrono::{DateTime, NaiveDate, Utc};
use clap::ValueEnum;
use serde::Serialize;
use url::Url;

use crate::cli::{Cli, Command, UpdateField};
use crate::error::{AppError, Result};
use crate::goodreads;
use crate::models::{Book, Status};
use crate::store::{expand_tilde, load_store, save_store};

/// Parses a `YYYY-MM-DD` string into the inclusive lower bound of that day (midnight UTC).
fn parse_date_since(s: &str) -> Result<DateTime<Utc>> {
    let date = NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| AppError::InvalidDate(s.to_string()))?;
    Ok(date.and_hms_opt(0, 0, 0).unwrap().and_utc())
}

/// Parses a `YYYY-MM-DD` string into the exclusive upper bound of that day
/// (midnight UTC at the start of the following day), so the whole day is included
/// when compared with `<`.
fn parse_date_until(s: &str) -> Result<DateTime<Utc>> {
    let date = NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| AppError::InvalidDate(s.to_string()))?;
    let next_day = date
        .succ_opt()
        .ok_or_else(|| AppError::InvalidDate(s.to_string()))?;
    Ok(next_day.and_hms_opt(0, 0, 0).unwrap().and_utc())
}

pub fn run(cli: Cli) -> Result<()> {
    let store_path = expand_tilde(&cli.store)?;
    match cli.command {
        Command::Add { query } => {
            let mut store = load_store(&store_path)?;

            let url = if Url::parse(&query).is_ok() {
                query
            } else if let Ok(isbn) = query.parse::<isbn::Isbn>() {
                // Goodreads search only redirects for unhyphenated ISBNs, so
                // normalize via Isbn's Display impl rather than passing the
                // raw (possibly hyphenated) query through.
                goodreads::resolve_search_url(&isbn.to_string())?
            } else {
                return Err(AppError::InvalidAddQuery(query));
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
        Command::Ls {
            status,
            read_since,
            read_until,
        } => {
            let store = load_store(&store_path)?;
            let since = read_since.as_deref().map(parse_date_since).transpose()?;
            let until = read_until.as_deref().map(parse_date_until).transpose()?;
            let mut books: Vec<(&String, &Book)> = store
                .iter()
                .filter(|(_, book)| match &status {
                    Some(filter) => *filter == book.status,
                    None => true,
                })
                .filter(|(_, book)| match since {
                    Some(since) => book.date_finished.is_some_and(|d| d >= since),
                    None => true,
                })
                .filter(|(_, book)| match until {
                    Some(until) => book.date_finished.is_some_and(|d| d < until),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_date_since_is_midnight_utc_on_that_day() {
        let d = parse_date_since("2026-01-01").unwrap();
        assert_eq!(d.to_rfc3339(), "2026-01-01T00:00:00+00:00");
    }

    #[test]
    fn parse_date_until_is_midnight_utc_on_the_following_day() {
        let d = parse_date_until("2026-01-01").unwrap();
        assert_eq!(d.to_rfc3339(), "2026-01-02T00:00:00+00:00");
    }

    #[test]
    fn parse_date_until_rolls_over_month_and_year_boundaries() {
        let d = parse_date_until("2026-12-31").unwrap();
        assert_eq!(d.to_rfc3339(), "2027-01-01T00:00:00+00:00");
    }

    #[test]
    fn parse_date_since_rejects_invalid_input() {
        assert!(matches!(
            parse_date_since("not-a-date"),
            Err(AppError::InvalidDate(_))
        ));
        assert!(matches!(
            parse_date_since("2026-13-40"),
            Err(AppError::InvalidDate(_))
        ));
    }

    #[test]
    fn parse_date_until_rejects_invalid_input() {
        assert!(matches!(
            parse_date_until("not-a-date"),
            Err(AppError::InvalidDate(_))
        ));
    }
}
