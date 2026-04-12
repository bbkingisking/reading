Build a personal reading tracker CLI in Rust. Single file is fine (main.rs) unless it gets unwieldy, in which case split into main.rs + types.rs + store.rs.
Storage
A single JSON file at ~/.reading/books.json, an object keyed by ISBN string. Read the whole file into memory, mutate, write back atomically (write to a temp file, rename). Create the file and directory if they don't exist.
Dependencies

clap
serde
serde_json
ureq
chrono with derive feature
thierror

Error handling
Use thiserror for a top-level AppError enum covering IO errors, JSON parse errors, ureq errors, and "book not found" / "ISBN not found in OpenLibrary" cases. No unwrap outside of genuinely unreachable cases.

Data model
One flat struct for everything, no nesting:

```rs
struct Book {
    title: String,
    author: String,
    pages: Option<u32>,
    publish_date: Option<String>,
    subjects: Option<Vec<String>>,
    goodreads_id: Option<String>,
    status: Status,
    date_added: DateTime<Utc>,
    date_started: Option<DateTime<Utc>>,
    date_finished: Option<DateTime<Utc>>,
    rating: Option<u8>,
    notes: Vec<String>,
}
```

enum Status { Unread, Reading, Read, Abandoned }
OpenLibrary API

https://openlibrary.org/api/books?bibkeys=ISBN:{isbn}&format=json&jscmd=details

The response is a map keyed by "ISBN:{isbn}". The book data lives under .details. Fields to extract:

details.title
details.authors[0].name (authors is a vec of { key, name })
details.number_of_pages
details.publish_date
details.subjects (vec of strings, optional)
details.identifiers.goodreads[0] (optional)

Define a private OlResponse / OlDetails / OlAuthor set of structs just for deserializing this response, then map into Book. These don't need to be exhaustive — use #[serde(default)] and Option liberally to handle missing fields gracefully.
CLI commands
reading add <isbn>
    - Fetch from OpenLibrary, populate metadata fields
    - status = Unread, date_added = now, notes = vec![]
    - Error if ISBN already exists in the file

reading done <isbn> [--rating <1-5>]
    - Set status = Read, date_finished = now
    - Optionally set rating
    - Error if book not found

reading start <isbn>
    - Set status = Reading, date_started = now

reading update <isbn> <field> <value>
    - field is a ValueEnum: rating, status, notes (append), publish-date
    - For notes, append to the vec rather than replacing

reading abandon <isbn>
    - Set status = Abandoned

reading show <isbn>
    - Pretty-print the book as JSON to stdout (serde_json::to_string_pretty)
    
reading ls              # all books, one line each
reading ls --status reading   # just in-progress
