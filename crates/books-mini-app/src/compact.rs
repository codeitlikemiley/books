//! Books TSX compaction for the manifest: strips `//` comment lines and
//! leading indentation outside strings, templates and comments. Used by
//! `build.rs`; the tests prove the compacted sources still parse.
//!
//! The scanner tracks strings, template literals (with `${}` nesting), block
//! comments and regex literals so a `//` or a quote inside one of those is
//! never mistaken for code. Regex-vs-division is the standard heuristic: a
//! `/` opens a regex unless the previous significant token could end an
//! expression (identifier or literal, `)`, `]`, `}`, string or template end).
//! A regex left open at end of line is treated as closed — real regexes never
//! span lines, so that keeps a false positive from corrupting the next line.

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Code,
    Template,
    Interpolation(usize),
    String(u8),
    Regex,
    RegexClass,
    BlockComment,
}

/// Identifier-ish keywords after which a `/` opens a regex, not a division.
const REGEX_AFTER: &[&str] = &[
    "await",
    "case",
    "delete",
    "do",
    "else",
    "in",
    "instanceof",
    "new",
    "of",
    "return",
    "throw",
    "typeof",
    "void",
    "yield",
];

struct Scanner {
    modes: Vec<Mode>,
    /// Last significant byte seen in code: decides regex vs division.
    last_sig: u8,
    /// Trailing identifier run, for the keyword check.
    word: String,
}

impl Scanner {
    fn new() -> Self {
        Self {
            modes: vec![Mode::Code],
            last_sig: b';',
            word: String::new(),
        }
    }

    fn protects_line(&self) -> bool {
        self.modes.iter().any(|mode| {
            matches!(
                mode,
                Mode::Template
                    | Mode::Interpolation(_)
                    | Mode::String(_)
                    | Mode::Regex
                    | Mode::RegexClass
            )
        })
    }

    fn value_ends_here(&self) -> bool {
        if matches!(
            self.last_sig,
            b')' | b']' | b'}' | b'\'' | b'"' | b'`' | b'/'
        ) {
            return true;
        }
        if self.last_sig.is_ascii_alphanumeric() || self.last_sig == b'_' || self.last_sig == b'$' {
            return !REGEX_AFTER.contains(&self.word.as_str());
        }
        false
    }

    fn note(&mut self, byte: u8) {
        if byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$' {
            self.word.push(byte as char);
            if self.word.len() > 16 {
                self.word.remove(0);
            }
        } else if byte != b' ' && byte != b'\t' {
            self.word.clear();
        }
        if byte != b' ' && byte != b'\t' {
            self.last_sig = byte;
        }
    }

    fn scan_line(&mut self, line: &str) {
        let bytes = line.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let mode = *self.modes.last().expect("scanner has a root mode");
            match mode {
                Mode::BlockComment => {
                    if bytes[i..].starts_with(b"*/") {
                        self.modes.pop();
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                Mode::String(quote) => {
                    if bytes[i] == b'\\' {
                        i += 1;
                        if i < bytes.len() {
                            i += 1;
                        }
                    } else if bytes[i] == quote {
                        self.modes.pop();
                        self.last_sig = quote;
                        self.word.clear();
                        i += 1;
                    } else {
                        i += 1;
                    }
                }
                Mode::Regex | Mode::RegexClass => {
                    if bytes[i] == b'\\' {
                        i += 1;
                        if i < bytes.len() {
                            i += 1;
                        }
                    } else if mode == Mode::Regex && bytes[i] == b'[' {
                        self.modes.push(Mode::RegexClass);
                        i += 1;
                    } else if mode == Mode::RegexClass && bytes[i] == b']' {
                        self.modes.pop();
                        i += 1;
                    } else if mode == Mode::Regex && bytes[i] == b'/' {
                        self.modes.pop();
                        self.last_sig = b'/';
                        self.word.clear();
                        i += 1;
                    } else {
                        i += 1;
                    }
                }
                Mode::Template => {
                    if bytes[i] == b'\\' {
                        i += 1;
                        if i < bytes.len() {
                            i += 1;
                        }
                    } else if bytes[i..].starts_with(b"${") {
                        self.modes.push(Mode::Interpolation(1));
                        i += 2;
                    } else if bytes[i] == b'`' {
                        self.modes.pop();
                        self.last_sig = b'`';
                        self.word.clear();
                        i += 1;
                    } else {
                        i += 1;
                    }
                }
                Mode::Code | Mode::Interpolation(_) => {
                    if bytes[i..].starts_with(b"//") {
                        break;
                    } else if bytes[i..].starts_with(b"/*") {
                        self.modes.push(Mode::BlockComment);
                        i += 2;
                    } else if bytes[i] == b'/' && !self.value_ends_here() {
                        self.modes.push(Mode::Regex);
                        i += 1;
                    } else if matches!(bytes[i], b'\'' | b'"') && !self.value_ends_here() {
                        // a real string starts where an expression can; a
                        // letter-then-apostrophe ("it's") is JSX text
                        self.modes.push(Mode::String(bytes[i]));
                        i += 1;
                    } else if bytes[i] == b'`' {
                        // tagged templates (`tag`tpl``) mean a backtick opens a
                        // template even after an identifier
                        self.modes.push(Mode::Template);
                        i += 1;
                    } else if let Mode::Interpolation(depth) = mode {
                        if bytes[i] == b'{' {
                            *self.modes.last_mut().unwrap() =
                                Mode::Interpolation(depth.saturating_add(1));
                            self.note(bytes[i]);
                            i += 1;
                        } else if bytes[i] == b'}' {
                            if depth == 1 {
                                self.modes.pop();
                                self.last_sig = b'`';
                                self.word.clear();
                            } else {
                                *self.modes.last_mut().unwrap() = Mode::Interpolation(depth - 1);
                            }
                            i += 1;
                        } else {
                            self.note(bytes[i]);
                            i += 1;
                        }
                    } else {
                        self.note(bytes[i]);
                        i += 1;
                    }
                }
            }
        }
        // A regex or quoted string never spans lines: an unclosed one is a
        // mis-scan of `a / b` or a JSX apostrophe, so close it rather than let
        // it protect the next line. Templates do span lines and stay open.
        while matches!(
            self.modes.last(),
            Some(Mode::Regex | Mode::RegexClass | Mode::String(_))
        ) {
            self.modes.pop();
        }
    }
}

/// Strip `//` comment lines and leading indentation outside strings,
/// templates and comments. Lines inside a string, template or block comment
/// pass through untouched, and a line that starts with `//` also passes
/// through verbatim: inside JSX children it could be rendered text.
#[doc(hidden)]
pub fn compact_tsx(source: &str, path: &str) -> String {
    let mut scanner = Scanner::new();
    let mut compacted = String::with_capacity(source.len());
    for line in source.split_inclusive('\n') {
        let protected = scanner.protects_line();
        if protected || line.trim_start().starts_with("//") {
            compacted.push_str(line);
        } else {
            compacted.push_str(line.trim_start_matches([' ', '\t']));
        }
        scanner.scan_line(line);
    }
    assert!(
        scanner.modes.len() == 1 && scanner.modes[0] == Mode::Code,
        "unclosed string, template, or comment in {path}"
    );
    compacted
}
