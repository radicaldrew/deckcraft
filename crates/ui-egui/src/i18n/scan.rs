//! Test-only scan of this crate's sources for the English strings the interface shows: the
//! literal arguments of `tl!(…)`, and the labels of the ribbon helpers that keep the English
//! text for the toolbar and the tests (`big(…)`, `small(…)`, `menu_item(…)`, `tip(…)`), which
//! translate it themselves when they draw.
//!
//! The coverage test and the gaps report build their key list from this, so a string shows up
//! as missing the moment it is added to the code.

use std::collections::BTreeSet;
use std::path::Path;

/// The call patterns whose string-literal argument is a lookup key: the text that opens the
/// argument list, and which argument (0-based) is the key.
const CALLS: &[(&str, usize)] = &[("tl!(", 0), ("big(", 3), ("small(", 3), ("menu_item(", 3), ("tip(", 1)];

/// Every string literal passed as a lookup key anywhere under `src/` (tests excluded).
pub fn ui_literals() -> BTreeSet<String> {
    let mut stack = vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("src")];
    let mut literals = BTreeSet::new();
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(dir).expect("UI source directory") {
            let path = entry.expect("UI source entry").path();
            if path.is_dir() {
                if path.file_name().is_some_and(|name| name != "i18n") {
                    stack.push(path);
                }
            } else if path.extension().is_some_and(|ext| ext == "rs")
                && !path.to_string_lossy().ends_with("_tests.rs")
                // The proofing languages are listed in their own names, whatever the interface language.
                && path.file_name().is_some_and(|name| name != "squiggles.rs")
            {
                let source = std::fs::read_to_string(&path).expect("UI source file").replace("\r\n", "\n");
                let source = source.replace("crate::i18n::t(", "tl!(");
                let code = source.split("#[cfg(test)]\nmod ").next().unwrap_or_default();
                literals.extend(literals_in(code));
            }
        }
    }
    literals
}

/// The lookup-key literals of one source text: the call arguments, and the labels of the
/// tables the interface draws from through a variable (`[("Solid", "solid"), …]`, the
/// `=> "From Left"` arms of a name lookup, the `v.push("Shape Format")` of the contextual tabs).
pub fn literals_in(code: &str) -> BTreeSet<String> {
    let mut out = table_literals(code);
    for (open, index) in CALLS {
        let mut rest = code;
        while let Some((before, after)) = rest.split_once(open) {
            rest = after;
            // Only a call: not `tl!` inside a longer identifier such as `rtl!(` or `is_big(`.
            if before.chars().next_back().is_some_and(|c| c.is_alphanumeric() || c == '_' || c == ':') {
                continue;
            }
            if let Some(literal) = literal_argument(after, *index) {
                out.insert(literal);
            }
        }
    }
    out
}

/// A table's label literals: a tuple's first element, a tuple's last element after an id-like
/// literal, a match arm's value, or a pushed name. Capitalised text with a lowercase letter only,
/// so ids, keys and codes stay out.
fn table_literals(code: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    // A label: capitalised, with a lowercase letter, no `format!` holes (`{}`, `{:02}`) and no
    // escapes; `{name}` holes of an `i18n::fmt` template are fine.
    let is_label = |raw: &str| {
        raw.starts_with(|c: char| c.is_ascii_uppercase())
            && raw.chars().any(|c| c.is_ascii_lowercase())
            && !raw.contains("{}")
            && !raw.contains("{:")
            && !raw.contains('\\')
    };
    let mut harvest = |open: &str, close: &str, tuple: bool| {
        let mut rest = code;
        while let Some((before, after)) = rest.split_once(open) {
            rest = after;
            // A tuple's `("`, not a call's (`format!("…")`, `.replace("…")`): nothing of a name before it.
            if tuple && before.chars().next_back().is_some_and(|c| c.is_alphanumeric() || matches!(c, '_' | '!' | ')' | ']' | '?')) {
                continue;
            }
            let Some((raw, _)) = after.split_once('"') else { break };
            let Some(tail) = after.get(raw.len() + 1..) else { break };
            if tail.trim_start().starts_with(close) && is_label(raw) {
                out.insert(raw.to_string());
            }
        }
    };
    harvest("(\"", ",", true);
    harvest("=> \"", "", false);
    harvest(".push(\"", ")", false);
    // `("id", "Label")`: the label after an id-like first element.
    let mut rest = code;
    while let Some((_, after)) = rest.split_once("(\"") {
        rest = after;
        let Some((id, tail)) = after.split_once('"') else { break };
        if !id.starts_with(|c: char| c.is_ascii_lowercase()) || id.contains(' ') {
            continue;
        }
        let Some(tail) = tail.strip_prefix(", \"") else { continue };
        let Some((raw, tail)) = tail.split_once('"') else { break };
        if tail.starts_with(')') && is_label(raw) {
            out.insert(raw.to_string());
        }
    }
    out
}

/// The `index`th argument of the call whose argument list starts at `args`, when it is a plain
/// string literal (`"…"`), unescaped.
fn literal_argument(args: &str, index: usize) -> Option<String> {
    let bytes = args.as_bytes();
    let mut depth = 0usize;
    let mut arg = 0usize;
    let mut start = 0usize;
    let mut i = 0usize;
    // Every delimiter looked at is ASCII, so each index below is a char boundary.
    while let Some(&b) = bytes.get(i) {
        match b {
            b'"' => {
                // Skip the string, whatever it holds.
                let mut escaped = false;
                i += 1;
                while let Some(&d) = bytes.get(i) {
                    if d == b'"' && !escaped {
                        break;
                    }
                    escaped = d == b'\\' && !escaped;
                    i += 1;
                }
            }
            b'\'' => {
                // A char literal (`'('`, `'…'`, `'\n'`) or a lifetime; skip a char literal's quote pair.
                let tail = args.get(i + 1..).unwrap_or_default();
                let mut t = tail.chars();
                match (t.next(), t.next()) {
                    (Some('\\'), _) => {
                        if let Some(end) = tail.get(1..).and_then(|s| s.find('\'')) {
                            i += 2 + end;
                        }
                    }
                    (Some(c0), Some('\'')) => i += c0.len_utf8() + 1,
                    _ => {}
                }
            }
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                if depth == 0 {
                    // The end of the call.
                    return if arg == index { argument_literal(args.get(start..i)?) } else { None };
                }
                depth -= 1;
            }
            b',' if depth == 0 => {
                if arg == index {
                    return argument_literal(args.get(start..i)?);
                }
                arg += 1;
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// The text of a plain string literal argument, or `None` for anything else (`format!`, a
/// variable, a `concat!`).
fn argument_literal(arg: &str) -> Option<String> {
    let arg = arg.trim();
    let raw = arg.strip_prefix('"')?.strip_suffix('"')?;
    if raw.contains('"') && !raw.contains("\\\"") {
        return None;
    }
    serde_json::from_str(&format!("\"{raw}\"")).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_lookup_keys_and_skips_everything_else() {
        let code = r#"
            ui.button(tl!("Save"));
            ui.label(tl!( "Two\nlines" ));
            let x = tip("format.bold", "Format");
            big(app, ui, Icon::NewSlide, "New\nSlide", "slide.new", json!({}));
            small(app, ui, Icon::Search, "Find", "app.find", json!({"a": (1, 2)}));
            menu_item(app, ui, Icon::Picture, "Edit Photo Album…", "insert.photoAlbumEdit", json!({}));
            ui.label(tl!(format!("{n} items")));
            ui.label(tl!(name));
            is_big(app, ui, Icon::X, "not a key", "id", json!({}));
            ui.rtl!("not either");
            paint(ui, '(', "neither");
            for (l, v) in [("No Shadow", "none"), ("Outer", "outer")] {}
            let rows = [("list", "List"), ("slide.new", "New Slide"), ("rrggbb", "RRGGBB")];
            let name = match dir { "l" => "From Left".into(), "r" => "From Right", _ => "x" };
            v.push("Shape Format");
            json!({"kind": "Rectangle", "list": ["Alpha", "beta"]});
            let id = ("fill", 3);
            let s = s.replace("Cmd+", "⌘");
            let v = format!("Version {}", 1);
            let w = format!("Brightness {:+}%", 1);
        "#;
        let found = literals_in(code);
        let expected: BTreeSet<String> = [
            "Save",
            "Two\nlines",
            "Format",
            "New\nSlide",
            "Find",
            "Edit Photo Album…",
            "No Shadow",
            "Outer",
            "List",
            "New Slide",
            "From Left",
            "From Right",
            "Shape Format",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        assert_eq!(found, expected);
    }

    #[test]
    fn the_crate_has_lookup_keys() {
        let all = ui_literals();
        assert!(all.len() > 100, "{}", all.len());
        assert!(all.contains("Cancel"), "the shared dialog buttons are keys");
    }
}
