mod cli;
mod commands;
mod error;
mod goodreads;
mod models;
mod sigpipe;
mod store;

use std::process;

use clap::Parser;

use cli::Cli;
use sigpipe::reset_sigpipe;

fn main() {
    reset_sigpipe();
    let cli = Cli::parse();
    if let Err(e) = commands::run(cli) {
        eprintln!("error: {}", e);
        process::exit(1);
    }
}
