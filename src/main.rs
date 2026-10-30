mod cli;
mod commands;
mod completions;
mod error;
mod goodreads;
mod models;
mod sigpipe;
mod store;

use std::process;

use clap::{CommandFactory, Parser};

use cli::Cli;
use sigpipe::reset_sigpipe;

fn main() {
    reset_sigpipe();
    let cli = Cli::parse();
    if let cli::Command::Completions { shell } = &cli.command {
        if let Err(e) = completions::generate(*shell) {
            eprintln!("error: {e}");
            process::exit(1);
        }
        return;
    }
    if let cli::Command::CompleteBooks { prefix, command } = &cli.command {
        // Completion is read-only and deliberately quiet when a store is unavailable.
        if let Some(store) = &cli.store {
            let _ = completions::books(store, prefix, command.as_deref());
        }
        return;
    }
    let Some(store) = cli.store else {
        Cli::command()
            .error(
                clap::error::ErrorKind::MissingRequiredArgument,
                "--store <STORE> is required",
            )
            .exit();
    };
    if let Err(e) = commands::run(cli.command, &store) {
        eprintln!("error: {}", e);
        process::exit(1);
    }
}
