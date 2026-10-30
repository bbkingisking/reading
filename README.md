# reading

A personal reading tracker with a YAML store.

Books are stored as a top-level YAML list, with each book's Goodreads ID in its
`id` field. The app uses `Vec<Book>` in memory and saves books by `date_added`
descending (newest first), breaking ties by ID. Missing or duplicate IDs in a
list are rejected.

## Zsh completions

Build and install into `~/bin` (matching an alias that invokes `~/bin/reading`):

```sh
cargo install --path . --root ~ --force
```

After zsh's completion system has been initialized, load the generated script:

```zsh
autoload -Uz compinit
compinit
source <(reading completions zsh)
```

If your shell configuration already initializes completion, only add the `source`
line, after that initialization and your `reading` alias. For example:

```zsh
alias reading='/Users/seok/bin/reading -s /Users/seok/literature/log/books.yaml'
source <(reading completions zsh)
```

Run the `source` line in your current terminal to enable completion immediately;
keep it in `~/.zshrc` to enable it in future sessions. Aliases with a fixed store
argument work with zsh's default alias completion behavior. If you have enabled
`COMPLETE_ALIASES`, use `unsetopt COMPLETE_ALIASES` to let zsh expand the alias
before completing it.

Try `reading upd<Tab>`, `reading ls --<Tab>`, or
`reading ls --status <Tab>`. Completion covers commands, flags, store file paths,
and enum values such as statuses and update fields.

For `done`, `start`, `update`, `abandon`, `show`, and `share`, Tab also suggests
books from the selected store, displaying their IDs, titles, and authors while
inserting only the ID. Type part of a title or author (case-insensitive), or an
ID prefix, then press Tab. `done` suggests only books with status `reading`,
and `start` suggests only books with status `unread`. The other commands suggest
books of any status.
Suggestions read the current store on each completion and never modify it.
Missing or invalid stores quietly produce no book suggestions.

`reading completions zsh` prints the script without opening the store, even when
the alias supplies one. Other supported shells are listed by
`reading completions --help`.
