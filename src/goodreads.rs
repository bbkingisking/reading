use crate::models::{Book, Status};
use chrono::Utc;
use serde::Deserialize;
use thiserror::Error;
use url::Url;
use html_escape::decode_html_entities;

#[derive(Debug, Error)]
pub enum GoodreadsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] ureq::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid Goodreads URL: {0}")]
    InvalidUrl(String),
    #[error("could not extract Goodreads ID from URL: {0}")]
    IdNotFound(String),
    #[error("could not find JSON-LD data in Goodreads page")]
    JsonLdNotFound,
}

#[derive(Debug, Deserialize)]
pub struct GoodreadsJsonLd {
    pub name: String,
    #[serde(rename = "numberOfPages")]
    pub number_of_pages: Option<u32>,
    pub isbn: Option<String>,
    pub author: Option<Vec<GoodreadsAuthor>>,
}

#[derive(Debug, Deserialize)]
pub struct GoodreadsAuthor {
    pub name: String,
}

fn extract_goodreads_id(url: &str) -> Result<String, GoodreadsError> {
    let parsed = Url::parse(url).map_err(|_| GoodreadsError::InvalidUrl(url.to_string()))?;

    if parsed.host_str() != Some("www.goodreads.com")
        && parsed.host_str() != Some("goodreads.com")
    {
        return Err(GoodreadsError::InvalidUrl(url.to_string()));
    }

    // URL format: https://www.goodreads.com/book/show/{id} or {id}-{slug} or {id}.{slug}
    let path = parsed.path();
    let parts: Vec<&str> = path.split('/').collect();

    if parts.len() >= 4 && parts[1] == "book" && parts[2] == "show" {
        let id_part = parts[3];
        // Extract just the numeric ID (before any hyphen or dot)
        let id = id_part.split(&['-', '.'][..]).next().unwrap_or(id_part);
        Ok(id.to_string())
    } else {
        Err(GoodreadsError::IdNotFound(url.to_string()))
    }
}

pub fn fetch_from_goodreads(url: &str) -> Result<(String, Book), GoodreadsError> {
    let goodreads_id = extract_goodreads_id(url)?;

    let body = ureq::get(url).call()?.into_string()?;

    // Extract JSON-LD data
    let json_ld_start = body
        .find(r#"<script type="application/ld+json">"#)
        .ok_or(GoodreadsError::JsonLdNotFound)?;
    let json_start = json_ld_start + r#"<script type="application/ld+json">"#.len();
    let json_end = body[json_start..]
        .find("</script>")
        .ok_or(GoodreadsError::JsonLdNotFound)?;
    let json_str = &body[json_start..json_start + json_end];

    let gr: GoodreadsJsonLd = serde_json::from_str(json_str)?;

    let author = gr
        .author
        .as_ref()
        .and_then(|authors| authors.first())
        .map(|a| decode_html_entities(&a.name).into_owned())
        .unwrap_or_else(|| "Unknown".into());

    let book = Book {
        title: decode_html_entities(&gr.name).into_owned(),
        author,
        pages: gr.number_of_pages,
        publish_date: None,
        subjects: None,
        isbn: gr.isbn,
        status: Status::Unread,
        date_added: Utc::now(),
        date_started: None,
        date_finished: None,
        rating: None,
        notes: Vec::new(),
    };

    Ok((goodreads_id, book))
}