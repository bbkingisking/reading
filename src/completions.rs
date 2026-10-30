use std::io::{self, Write};

use clap::CommandFactory;
use clap_complete::Shell;

use crate::cli::Cli;
use crate::error::Result;
use crate::models::Status;
use crate::store::{expand_tilde, load_store};

pub fn generate(shell: Shell) -> io::Result<()> {
    let mut script = Vec::new();
    clap_complete::generate(shell, &mut Cli::command(), "reading", &mut script);
    if shell == Shell::Zsh {
        let script = String::from_utf8(script).expect("completion script is UTF-8");
        // Keep clap's command/flag completion, replacing only book ID actions.
        let script = script
            .replace("_reading() {", "_reading_generated() {")
            // clap_complete currently includes hidden commands in zsh menus.
            .replace("'__complete-books:' \\\n", "")
            .replace(
                ":id -- ID of the book (its Goodreads ID):_default",
                ":id -- ID of the book (its Goodreads ID):_reading_books",
            );
        let mut out = io::stdout().lock();
        writeln!(out, "#compdef reading")?;
        writeln!(out, "{}", include_str!("../completions/books.zsh"))?;
        out.write_all(script.as_bytes())
    } else {
        io::stdout().lock().write_all(&script)
    }
}

fn single_line(value: &str) -> String {
    value
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

pub fn books(path: &str, prefix: &str, command: Option<&str>) -> Result<()> {
    let store = load_store(&expand_tilde(path)?)?;
    let prefix = prefix.to_lowercase();
    let mut entries: Vec<_> = store
        .iter()
        .filter(|book| match command {
            Some("done") => book.status == Status::Reading,
            Some("start") => book.status == Status::Unread,
            _ => true,
        })
        .filter(|book| {
            book.id.starts_with(&prefix)
                || book.title.to_lowercase().contains(&prefix)
                || book.author.to_lowercase().contains(&prefix)
        })
        .collect();
    entries.sort_by(|a, b| {
        a.title
            .to_lowercase()
            .cmp(&b.title.to_lowercase())
            .then(a.id.cmp(&b.id))
    });
    let mut out = io::stdout().lock();
    for book in entries {
        writeln!(
            out,
            "{}\t{} — {} — {}",
            single_line(&book.id),
            single_line(&book.id),
            single_line(&book.title),
            single_line(&book.author)
        )?;
    }
    Ok(())
}
