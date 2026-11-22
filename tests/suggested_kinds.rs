//! The New Category flow offers every suggestion in
//! `apps/books/suggested-categories.json`; its `subtype` becomes the new
//! account's `kind`, which `books_app_account_create` refuses unless it is a
//! catalogue kind, `mixed` or `unsure` — the earlier code passed the subtype
//! raw, so e.g. `fixed_assets` was refused. entry.tsx maps the subtypes that
//! name a Books kind; the rest fall back to `unsure`, and the account's
//! `type` still comes from the suggestion. These tests pin that contract:
//! every suggested subtype is itself a catalogue kind or is mapped to one,
//! and every mapped kind is real.

use std::collections::BTreeSet;

const ENTRY: &str = include_str!("../apps/books/entry.tsx");
const CATALOG: &str = include_str!("../apps/books/suggested-categories.json");

/// The `SUGGESTED_KIND` table's `subtype: "kind"` rows.
fn mapped() -> Vec<(String, String)> {
    let start = ENTRY.find("SUGGESTED_KIND").expect("SUGGESTED_KIND exists");
    let body = &ENTRY[start..];
    let end = body.find("};").expect("the map closes");
    body[..end]
        .lines()
        .filter_map(|line| {
            let (key, value) = line.trim().split_once(':')?;
            let value = value.trim().trim_end_matches(',');
            (value.len() > 2 && value.starts_with('"') && value.ends_with('"'))
                .then(|| (key.trim().to_string(), value.trim_matches('"').to_string()))
        })
        .collect()
}

fn subtypes() -> BTreeSet<String> {
    serde_json::from_str::<serde_json::Value>(CATALOG)
        .unwrap()
        .as_array()
        .expect("the suggestions file is an array")
        .iter()
        .filter_map(|s| s.get("subtype")?.as_str().map(String::from))
        .collect()
}

#[test]
fn every_mapped_kind_is_a_catalogue_kind() {
    let mapped = mapped();
    assert!(!mapped.is_empty());
    for (subtype, kind) in &mapped {
        assert!(
            books_core::catalog::Catalog::get().kind(kind).is_some(),
            "SUGGESTED_KIND maps {subtype} to unknown kind {kind}"
        );
    }
}

/// What `newCategoryKind` resolves to in entry.tsx: the mapped kind, `unsure`
/// when the subtype has no Books kind — `books_app_account_create` accepts
/// either, so every suggestion saves.
#[test]
fn every_suggested_subtype_resolves_to_a_kind_the_create_accepts() {
    let map: std::collections::BTreeMap<String, String> = mapped().into_iter().collect();
    for subtype in subtypes() {
        let resolved = map.get(&subtype).map(String::as_str).unwrap_or("unsure");
        assert!(
            books_core::ledger::KindRef::parse(resolved).is_some(),
            "subtype {subtype} resolves to {resolved}, which the create refuses"
        );
    }
}
