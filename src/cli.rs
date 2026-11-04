use crate::models::Status;
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "reading",
    about = "Personal reading tracker",
    arg_required_else_help = true,
    version
)]
pub struct Cli {
    /// Path to the YAML store file (required for book commands)
    #[arg(long, short, value_hint = clap::ValueHint::FilePath)]
    pub store: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(name = "__complete-books", hide = true)]
    CompleteBooks {
        #[arg(long)]
        command: Option<String>,
        #[arg(default_value = "")]
        prefix: String,
    },
    /// Print shell completion script
    Completions {
        /// Shell to generate completions for
        shell: clap_complete::Shell,
    },
    /// Add a book from its Goodreads URL, or by ISBN
    Add {
        /// Goodreads URL, or an ISBN-10/ISBN-13
        query: String,
    },
    /// Mark a book as finished
    Done {
        /// ID of the book (its Goodreads ID)
        id: String,
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
        /// Only include books finished on/after this date (YYYY-MM-DD)
        #[arg(long)]
        read_since: Option<String>,
        /// Only include books finished on/before this date (YYYY-MM-DD)
        #[arg(long)]
        read_until: Option<String>,
        /// Only include books started on/after this date (YYYY-MM-DD)
        #[arg(long)]
        started_since: Option<String>,
        /// Only include books started on/before this date (YYYY-MM-DD)
        #[arg(long)]
        started_until: Option<String>,
        /// Only include books added on/after this date (YYYY-MM-DD)
        #[arg(long)]
        added_since: Option<String>,
        /// Only include books added on/before this date (YYYY-MM-DD)
        #[arg(long)]
        added_until: Option<String>,
    },
}

#[derive(Debug, Clone, ValueEnum)]
pub enum UpdateField {
    Notes,
    Status,
    Title,
    Author,
    #[value(name = "publish-date")]
    PublishDate,
}
