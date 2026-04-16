use crate::models::{Book, Status};
use chrono::Utc;
use serde::Deserialize;
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OpenLibraryError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] ureq::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("ISBN {0} not found in OpenLibrary")]
    IsbnNotFound(String),
}

#[derive(Debug, Deserialize)]
pub struct OlResponse {
    pub details: OlDetails,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct OlDetails {
    pub title: Option<String>,
    pub authors: Option<Vec<OlAuthor>>,
    pub number_of_pages: Option<u32>,
    pub publish_date: Option<String>,
    pub subjects: Option<Vec<String>>,
    pub identifiers: Option<OlIdentifiers>,
}

#[derive(Debug, Deserialize)]
pub struct OlAuthor {
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OlIdentifiers {
    pub goodreads: Option<Vec<String>>,
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

pub fn fetch_from_openlibrary(isbn: &str) -> Result<Book, OpenLibraryError> {
    let url = format!(
        "https://openlibrary.org/api/books?bibkeys=ISBN:{}&format=json&jscmd=details",
        isbn
    );
    let body = ureq::get(&url).call()?.into_string()?;
    let resp: BTreeMap<String, OlResponse> = serde_json::from_str(&body)?;
    let key = format!("ISBN:{}", isbn);
    let ol = resp
        .get(&key)
        .ok_or_else(|| OpenLibraryError::IsbnNotFound(isbn.to_string()))?;
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
        isbn: Some(isbn.to_string()),
        goodreads_id,
        status: Status::Unread,
        date_added: Utc::now(),
        date_started: None,
        date_finished: None,
        rating: None,
        notes: Vec::new(),
    })
}