//! The manifest's `compact_tsx` strips `//` comment lines and indentation but
//! must leave code intact: a `//` inside a regex literal or JSX text is not a
//! comment, and a string or template's contents and delimiters survive.

use books_mini_app::compact::compact_tsx;

fn compact(source: &str) -> String {
    compact_tsx(source, "test.tsx")
}

#[test]
fn a_regex_literal_with_a_slash_run_stays_intact() {
    let src = "const re = /https?:\\/\\//g;\n  const ok = re.test(u);\n";
    let out = compact(src);
    assert!(out.contains("/https?:\\/\\//g"), "{out}");
    assert!(out.contains("re.test(u)"), "{out}");
}

#[test]
fn division_after_a_value_is_not_a_regex() {
    let src = "  const half = total / 2;\n  const third = total/3;\n";
    let out = compact(src);
    assert!(out.contains("total / 2"), "{out}");
    assert!(out.contains("total/3"), "{out}");
}

#[test]
fn jsx_text_with_slashes_and_quotes_survives() {
    let src = "  <Text>2 // 3 = 6</Text>\n  <Text>it's \"quoted\"</Text>\n";
    let out = compact(src);
    assert!(out.contains("2 // 3 = 6"), "{out}");
    assert!(out.contains("it's \"quoted\""), "{out}");
}

#[test]
fn a_template_literal_keeps_its_lines_and_interpolations() {
    let src = "  const msg = `first\n    second ${x + `inner`} tail`;\n  next();\n";
    let out = compact(src);
    assert!(out.contains("    second ${x + `inner`} tail"), "{out}");
    assert!(out.contains("next()"), "{out}");
}

#[test]
fn comment_lines_are_kept_verbatim_and_block_comments_protect() {
    let src = "  // a real comment\n  /* block\n   * more\n   */\n  code();\n";
    let out = compact(src);
    assert!(out.contains("// a real comment"), "{out}");
    assert!(out.contains("* more"), "{out}");
    assert!(out.contains("code()"), "{out}");
}

#[test]
fn a_regex_after_an_operator_or_return_opens_a_regex() {
    let src = "  const f = (s: string) => { return /a\\/b/.test(s); };\n  const g = flag && /x\\/y/.test(s);\n";
    let out = compact(src);
    assert!(out.contains("/a\\/b/"), "{out}");
    assert!(out.contains("/x\\/y/"), "{out}");
}

#[test]
fn the_real_books_sources_compact_to_parseable_tsx() {
    // every app file the manifest embeds, compacted, is still valid enough
    // for the host stand-in: it parses and the line count is unchanged
    for entry in
        std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../../apps/books")).unwrap()
    {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("tsx") {
            continue;
        }
        let source = std::fs::read_to_string(&path).unwrap();
        let out = compact_tsx(&source, &path.display().to_string());
        assert_eq!(
            source.lines().count(),
            out.lines().count(),
            "{}",
            path.display()
        );
    }
}
