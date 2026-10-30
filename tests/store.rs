use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct TempStore(std::path::PathBuf);

impl TempStore {
    fn new(data: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "reading-store-{}-{}.yaml",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&path, data).unwrap();
        Self(path)
    }

    fn run(&self, args: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_reading"))
            .arg("-s")
            .arg(&self.0)
            .args(args)
            .output()
            .unwrap()
    }
}

impl Drop for TempStore {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[test]
fn saving_a_list_orders_by_date_added_descending_and_updates_by_id() {
    let store = TempStore::new(include_str!("fixtures/books.yaml"));
    let output = store.run(&["start", "456"]);
    assert!(output.status.success(), "{:?}", output);
    let data = std::fs::read_to_string(&store.0).unwrap();
    assert!(data.starts_with("- id:"));
    assert!(data.find("456").unwrap() < data.find("123").unwrap());
    let output = store.run(&["show", "456"]);
    assert!(output.status.success());
    let book = String::from_utf8(output.stdout).unwrap();
    assert!(book.contains("status: reading"));
    assert!(book.contains("id:"));
}

#[test]
fn lists_reject_duplicate_and_missing_ids_without_modifying_the_file() {
    let fixture = include_str!("fixtures/books.yaml");
    for data in [
        fixture.replace("\"456\"", "\"123\""),
        fixture.replace("- id: \"123\"\n  title:", "- title:"),
    ] {
        let store = TempStore::new(&data);
        assert!(!store.run(&["start", "123"]).status.success());
        assert_eq!(std::fs::read_to_string(&store.0).unwrap(), data);
    }
}
