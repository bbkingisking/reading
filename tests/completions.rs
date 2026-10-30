use std::process::Command;

#[test]
fn completions_work_without_a_store_and_ignore_an_alias_store() {
    for args in [
        vec!["completions", "zsh"],
        vec![
            "-s",
            "/nonexistent/reading-store.yaml",
            "completions",
            "zsh",
        ],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_reading"))
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success(), "{:?}", output);
        assert!(output.stdout.starts_with(b"#compdef reading"));
    }
}

#[test]
fn book_commands_still_require_a_store() {
    let output = Command::new(env!("CARGO_BIN_EXE_reading"))
        .arg("ls")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--store <STORE> is required"));
}

#[test]
fn book_candidates_match_titles_authors_and_ids_without_changing_the_store() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/books.yaml");
    let before = std::fs::read(path).unwrap();
    for (prefix, expected) in [
        ("DUNE", "123\t123 — Dune: Part One — Frank Herbert\n"),
        ("herbert", "123\t123 — Dune: Part One — Frank Herbert\n"),
        ("12", "123\t123 — Dune: Part One — Frank Herbert\n"),
        (
            "earthsea",
            "456\t456 — A Wizard of Earthsea — Ursula K. Le Guin\n",
        ),
        ("unknown", ""),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_reading"))
            .args(["-s", path, "__complete-books", "--", prefix])
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
        assert!(output.stderr.is_empty());
    }
    assert_eq!(std::fs::read(path).unwrap(), before);
}

#[test]
fn start_and_done_suggest_books_with_the_appropriate_status() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/books.yaml");
    for (command, prefix, expected) in [
        ("done", "", "123\t123 — Dune: Part One — Frank Herbert\n"),
        ("done", "earthsea", ""),
        (
            "start",
            "",
            "456\t456 — A Wizard of Earthsea — Ursula K. Le Guin\n",
        ),
        ("start", "dune", ""),
        (
            "start",
            "earthsea",
            "456\t456 — A Wizard of Earthsea — Ursula K. Le Guin\n",
        ),
        (
            "show",
            "earthsea",
            "456\t456 — A Wizard of Earthsea — Ursula K. Le Guin\n",
        ),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_reading"))
            .args([
                "-s",
                path,
                "__complete-books",
                "--command",
                command,
                "--",
                prefix,
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
    }
}

#[test]
fn zsh_script_connects_all_six_book_arguments_to_dynamic_completion() {
    let output = Command::new(env!("CARGO_BIN_EXE_reading"))
        .args(["completions", "zsh"])
        .output()
        .unwrap();
    let script = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        script
            .matches(":id -- ID of the book (its Goodreads ID):_reading_books")
            .count(),
        6
    );
    assert!(!script.contains("__complete-books:"));
}

#[test]
fn unavailable_stores_produce_no_completion_errors() {
    for args in [
        vec!["__complete-books"],
        vec!["-s", "/nonexistent/reading-store.yaml", "__complete-books"],
        vec!["-s", "Cargo.toml", "__complete-books"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_reading"))
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
}
