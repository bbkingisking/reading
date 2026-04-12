use crate::models::Status;
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "reading", about = "Personal reading tracker", arg_required_else_help = true, version)]
pub struct Cli {
    /// Path to the JSON store file
    #[arg(long, short)]
    pub store: String,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Add a book by ISBN or Goodreads URL
    Add {
        /// ISBN-10, ISBN-13, or Goodreads URL (e.g., https://www.goodreads.com/book/show/12345)
        input: String,
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

#[derive(Debug, Clone, ValueEnum)]
pub enum UpdateField {
    Rating,
    Status,
    Notes,
    Title,
    Author,
    #[value(name = "publish-date")]
    PublishDate,
}