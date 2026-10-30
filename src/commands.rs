use chrono::{DateTime, NaiveDate, Utc};
use clap::ValueEnum;
use url::Url;

use crate::cli::{Command, UpdateField};
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

/// Returns `true` when `book` has a date field that falls within the optional
/// `[since, until)` bounds. Missing values are treated as out of range.
fn filter_by_date_range<F>(
    book: &Book,
    since: Option<DateTime<Utc>>,
    until: Option<DateTime<Utc>>,
    field: F,
) -> bool
where
    F: Fn(&Book) -> Option<DateTime<Utc>>,
{
    let value = field(book);
    if let Some(since) = since
        && !value.is_some_and(|d| d >= since)
    {
        return false;
    }
    if let Some(until) = until
        && !value.is_some_and(|d| d < until)
    {
        return false;
    }
    true
}

pub fn run(command: Command, store: &str) -> Result<()> {
    let store_path = expand_tilde(store)?;
    match command {
        Command::Completions { .. } | Command::CompleteBooks { .. } => {
            unreachable!("completions are handled before opening the store")
        }
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
            let book = goodreads::fetch_from_goodreads(&url)?;

            if store.iter().any(|existing| existing.id == book.id) {
                return Err(AppError::AlreadyExists(book.id));
            }

            store.push(book);
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
                .iter_mut()
                .find(|book| book.id == id)
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
                .iter_mut()
                .find(|book| book.id == id)
                .ok_or_else(|| AppError::NotFound(id.clone()))?;
            book.status = Status::Reading;
            book.date_started = Some(Utc::now());
            save_store(&store, &store_path)?;
        }
        Command::Update { id, field, value } => {
            let mut store = load_store(&store_path)?;
            let book = store
                .iter_mut()
                .find(|book| book.id == id)
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
                .iter_mut()
                .find(|book| book.id == id)
                .ok_or_else(|| AppError::NotFound(id.clone()))?;
            book.status = Status::Abandoned;
            save_store(&store, &store_path)?;
        }
        Command::Show { id } => {
            let store = load_store(&store_path)?;
            let book = store
                .iter()
                .find(|book| book.id == id)
                .ok_or_else(|| AppError::NotFound(id.clone()))?;

            println!("{}", serde_saphyr::to_string(book)?);
        }
        Command::Share { id } => {
            let store = load_store(&store_path)?;
            if !store.iter().any(|book| book.id == id) {
                return Err(AppError::NotFound(id));
            }
            println!("https://www.goodreads.com/book/show/{}", id);
        }
        Command::Ls {
            status,
            read_since,
            read_until,
            started_since,
            started_until,
            added_since,
            added_until,
        } => {
            let store = load_store(&store_path)?;
            let read_since = read_since.as_deref().map(parse_date_since).transpose()?;
            let read_until = read_until.as_deref().map(parse_date_until).transpose()?;
            let started_since = started_since.as_deref().map(parse_date_since).transpose()?;
            let started_until = started_until.as_deref().map(parse_date_until).transpose()?;
            let added_since = added_since.as_deref().map(parse_date_since).transpose()?;
            let added_until = added_until.as_deref().map(parse_date_until).transpose()?;
            let mut books: Vec<&Book> = store
                .iter()
                .filter(|book| match &status {
                    Some(filter) => *filter == book.status,
                    None => true,
                })
                .filter(|book| {
                    filter_by_date_range(book, read_since, read_until, |b| b.date_finished)
                })
                .filter(|book| {
                    filter_by_date_range(book, started_since, started_until, |b| b.date_started)
                })
                .filter(|book| {
                    filter_by_date_range(book, added_since, added_until, |b| Some(b.date_added))
                })
                .collect();
            books.sort_by_key(|entry| std::cmp::Reverse(entry.date_added));
            for book in books {
                println!(
                    "{} - {} - {} - {}",
                    book.title, book.author, book.status, book.id
                );
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

    fn dt(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().to_utc()
    }

    fn book_with_dates(
        date_added: DateTime<Utc>,
        date_started: Option<DateTime<Utc>>,
        date_finished: Option<DateTime<Utc>>,
    ) -> Book {
        Book {
            id: "123".into(),
            title: "Title".to_string(),
            author: "Author".to_string(),
            pages: None,
            publish_date: None,
            subjects: None,
            isbn: None,
            status: Status::Unread,
            date_added,
            date_started,
            date_finished,
            rating: None,
            notes: vec![],
        }
    }

    #[test]
    fn filter_by_date_range_passes_when_no_bounds_are_given() {
        let book = book_with_dates(dt("2026-01-15T00:00:00Z"), None, None);
        assert!(filter_by_date_range(&book, None, None, |b| Some(
            b.date_added
        )));
    }

    #[test]
    fn filter_by_date_range_respects_since_bound() {
        let book = book_with_dates(dt("2026-01-15T12:00:00Z"), None, None);
        assert!(filter_by_date_range(
            &book,
            Some(dt("2026-01-15T00:00:00Z")),
            None,
            |b| Some(b.date_added)
        ));
        assert!(!filter_by_date_range(
            &book,
            Some(dt("2026-01-16T00:00:00Z")),
            None,
            |b| Some(b.date_added)
        ));
    }

    #[test]
    fn filter_by_date_range_respects_until_bound() {
        let book = book_with_dates(dt("2026-01-15T12:00:00Z"), None, None);
        assert!(filter_by_date_range(
            &book,
            None,
            Some(dt("2026-01-16T00:00:00Z")),
            |b| Some(b.date_added)
        ));
        assert!(!filter_by_date_range(
            &book,
            None,
            Some(dt("2026-01-15T00:00:00Z")),
            |b| Some(b.date_added)
        ));
    }

    #[test]
    fn filter_by_date_range_treats_missing_date_as_out_of_range() {
        let book = book_with_dates(dt("2026-01-15T00:00:00Z"), None, None);
        assert!(!filter_by_date_range(
            &book,
            Some(dt("2026-01-01T00:00:00Z")),
            None,
            |b| b.date_started
        ));
        assert!(!filter_by_date_range(
            &book,
            None,
            Some(dt("2027-01-01T00:00:00Z")),
            |b| b.date_started
        ));
    }
}
