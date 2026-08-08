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
    /// Add a book from its Goodreads URL, or by ISBN (resolved via Goodreads search)
    Add {
        /// Goodreads URL, or an ISBN-10/ISBN-13
        query: String,
    },
    /// Mark a book as finished
    Done {
        /// ID of the book (its Goodreads ID)
        id: String,
        /// Optional rating (1-5)
        #[arg(long)]
        rating: Option<u8>,
    },
    /// Mark a book as currently reading
    Start {
        /// ID of the book (its Goodreads ID)
        id: String,
    },
    /// Update a field on a book
    Update {
        /// ID of the book (its Goodreads ID)
        id: String,
        /// Which field to update
        field: UpdateField,
        /// New value
        value: String,
    },
    /// Mark a book as abandoned
    Abandon {
        /// ID of the book (its Goodreads ID)
        id: String,
    },
    /// Pretty-print a book's details
    Show {
        /// ID of the book (its Goodreads ID)
        id: String,
    },
    /// Print the book's Goodreads URL
    Share {
        /// ID of the book (its Goodreads ID)
        id: String,
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