use chrono::{DateTime, Utc};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Status {
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
pub struct Book {
    pub title: String,
    pub author: String,
    pub pages: Option<u32>,
    pub publish_date: Option<String>,
    pub subjects: Option<Vec<String>>,
    pub isbn: Option<String>,
    pub status: Status,
    pub date_added: DateTime<Utc>,
    pub date_started: Option<DateTime<Utc>>,
    pub date_finished: Option<DateTime<Utc>>,
    pub rating: Option<u8>,
    pub notes: Vec<String>,
}