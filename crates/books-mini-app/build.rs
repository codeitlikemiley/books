use std::{env, fs, path::PathBuf};

const BOOKS_FILES: &[&str] = &[
    "../../apps/books/kit.tsx",
    "../../apps/books/suggested-categories.json",
    "../../apps/books/today.tsx",
    "../../apps/books/ledger_review.tsx",
    "../../apps/books/ledger_findings.tsx",
    "../../apps/books/ledger_amend.tsx",
    "../../apps/books/ledger_bulk.tsx",
    "../../apps/books/entry.tsx",
    "../../apps/books/entry_side.tsx",
    "../../apps/books/inbox.tsx",
    "../../apps/books/accounts.tsx",
    "../../apps/books/category_detail.tsx",
    "../../apps/books/unpaid.tsx",
    "../../apps/books/owed_print.tsx",
    "../../apps/books/owed_repeat.tsx",
    "../../apps/books/people.tsx",
    "../../apps/books/tags.tsx",
    "../../apps/books/reports.tsx",
    "../../apps/books/reconcile.tsx",
    "../../apps/books/statement_read.tsx",
    "../../apps/books/statement_snapshot.tsx",
    "../../apps/books/reconcile_flags.tsx",
    "../../apps/books/settings.tsx",
    "../../apps/books/oldbooks.tsx",
    "../../apps/books/forms.tsx",
    "../../apps/books/payroll.tsx",
    "../../apps/books/certificates.tsx",
    "../../apps/books/activity.tsx",
    "../../apps/books/transactions_side.tsx",
    "../../apps/books/transactions.tsx",
    "../../apps/books/window.tsx",
];

#[path = "src/compact.rs"]
mod compact;

use compact::{compact_json_lines, compact_tsx};

fn join_books(sources: &[String]) -> String {
    let mut joined = String::new();
    joined.push_str(&sources[0]);
    joined.push('\n');
    joined.push_str("const SUGGESTED_CATEGORY_CATALOG: Any[] = ");
    joined.push_str(&sources[1]);
    joined.push_str(";\n");
    let pages = &sources[2..];
    for (index, page) in pages.iter().enumerate() {
        joined.push_str(page);
        if index + 1 < pages.len() {
            joined.push('\n');
        }
    }
    joined
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    for path in BOOKS_FILES {
        println!("cargo:rerun-if-changed={path}");
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let sources = BOOKS_FILES
        .iter()
        .map(|path| fs::read_to_string(manifest_dir.join(path)))
        .collect::<Result<Vec<_>, _>>()?;
    let plain = join_books(&sources);
    let compacted_sources = BOOKS_FILES
        .iter()
        .zip(&sources)
        .map(|(path, source)| {
            if path.ends_with(".tsx") {
                compact_tsx(source, path)
            } else if path.ends_with(".json") {
                compact_json_lines(source)
            } else {
                source.to_owned()
            }
        })
        .collect::<Vec<_>>();
    let compacted = join_books(&compacted_sources);
    assert_eq!(
        plain.lines().count(),
        compacted.lines().count(),
        "Books source compaction changed the line count"
    );

    let bulk_index = BOOKS_FILES
        .iter()
        .position(|path| path.ends_with("/ledger_bulk.tsx"))
        .expect("Books file list includes the bulk panel");
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    fs::write(out_dir.join("books.tsx"), compacted)?;
    fs::write(
        out_dir.join("ledger_bulk.tsx"),
        &compacted_sources[bulk_index],
    )?;
    println!(
        "cargo:rustc-env=BOOKS_PLAIN_LINE_COUNT={}",
        plain.lines().count()
    );
    println!("cargo:rustc-env=BOOKS_PLAIN_BYTES={}", plain.len());
    Ok(())
}
