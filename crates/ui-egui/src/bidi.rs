//! Display order for the labels of a right-to-left interface language (the Hebrew catalog).
//!
//! egui shapes each font-face run with the script's own direction, so the letters of a Hebrew
//! word run right to left, but it places the runs themselves left to right in logical order:
//! `שמירה בשם…` would show its two words swapped and the ellipsis first. [`display_rtl`] puts the
//! runs of a label in display order once, when its catalog loads, and leaves the letters of each
//! word alone for the shaper.
//!
//! Copied from PdfCraft's `crates/ui-egui/src/bidi.rs` (the part that reorders catalog labels);
//! the slide text itself goes through the full UAX #9 in `crates/text`, not through here.

use unicode_bidi::{BidiInfo, Level, ParagraphInfo};

/// A label of a right-to-left interface language, in logical order, rearranged for egui: each
/// line reads right to left, with Latin words and numbers inside it left to right.
///
/// - `{name}` placeholders stay whole and unmirrored, so [`crate::i18n::fmt`] still fills them.
///   What it inserts is drawn left to right at that place.
/// - Spaces and separator dots at either end stay at that end: the code appends to a label on
///   its right (`"Your name: "` + a field), as the interface is still laid out left to right.
/// - A short caption's closing colon stays on the right too, next to the widget it captions.
///
/// Text without right-to-left characters comes back unchanged. This is for painting one line
/// at a time; a label egui wraps breaks at the wrong end.
pub fn display_rtl(text: &str) -> String {
    if !text.chars().any(is_rtl_script) {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len());
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        push_rtl_line(&mut out, line);
    }
    out
}

/// The most words a caption may have for its closing colon to stay on the right.
const CAPTION_WORDS: usize = 3;
/// First of the private-use characters that stand for placeholders while a line is reordered.
/// They are strong left-to-right, like the names they replace.
const PLACEHOLDER_BASE: u32 = 0xE000;
/// Placeholders protected per line; a line with more keeps the rest as plain text.
const MAX_PLACEHOLDERS: usize = 64;

fn push_rtl_line(out: &mut String, line: &str) {
    let edge = |c: char| c.is_whitespace() || c == '·';
    let core = line.trim_start_matches(edge);
    let lead = line.get(..line.len().saturating_sub(core.len())).unwrap_or_default();
    let trimmed = core.trim_end_matches(edge);
    let tail = core.get(trimmed.len()..).unwrap_or_default();
    let mut core = trimmed;
    let mut colon = "";
    if let Some(caption) = core.strip_suffix(':')
        && caption.split_whitespace().count() <= CAPTION_WORDS
    {
        core = caption;
        colon = ":";
    }
    // A line that already uses the stand-in characters is reordered with its placeholders as text.
    let reserved = core.chars().any(|c| (PLACEHOLDER_BASE..PLACEHOLDER_BASE.saturating_add(MAX_PLACEHOLDERS as u32)).contains(&u32::from(c)));
    let (masked, names) = if reserved { (core.to_owned(), Vec::new()) } else { mask_placeholders(core) };
    let mut visual = String::with_capacity(masked.len());
    push_visual(&mut visual, &masked, Level::rtl());
    out.push_str(lead);
    for c in visual.chars() {
        let index = u32::from(c).checked_sub(PLACEHOLDER_BASE).and_then(|i| usize::try_from(i).ok());
        match index.and_then(|i| names.get(i)) {
            Some(name) => out.push_str(name),
            None => out.push(c),
        }
    }
    out.push_str(colon);
    out.push_str(tail);
}

/// `text` in display order for the paragraph direction `base`, appended to `out`.
fn push_visual(out: &mut String, text: &str, base: Level) {
    let info = BidiInfo::new(text, Some(base));
    for para in &info.paragraphs {
        push_line(out, text, &info, para, para.range.clone());
    }
}

/// The `line` of paragraph `para` (a non-empty byte range of `text`) in display order, as egui
/// needs it, appended to `out`.
fn push_line(out: &mut String, text: &str, info: &BidiInfo<'_>, para: &ParagraphInfo, line: std::ops::Range<usize>) {
    if line.is_empty() {
        return;
    }
    let (levels, runs) = info.visual_runs(para, line);
    // First the true display order, every character left to right as it appears on screen.
    let mut shown = String::with_capacity(text.len());
    for run in runs {
        let rtl = levels.get(run.start).is_some_and(Level::is_rtl);
        // Runs lie on character boundaries; a missing slice is skipped, never a panic.
        let Some(piece) = text.get(run) else { continue };
        let chars = piece.chars().filter(|c| !is_bidi_control(*c));
        if rtl {
            shown.extend(chars.rev().map(mirrored));
        } else {
            shown.extend(chars);
        }
    }
    push_for_shaper(out, &shown);
}

/// `shown` (in screen order) as egui must get it: egui hands each stretch of right-to-left
/// script to the shaper as one run, which lays it out right to left, so each such stretch goes
/// back to logical order. What lies between them (spaces, punctuation, Latin) is drawn by the
/// Latin face left to right and stays as it is.
fn push_for_shaper(out: &mut String, shown: &str) {
    let mut stretch: Vec<char> = Vec::new();
    for c in shown.chars() {
        if is_rtl_script(c) || is_joiner(c) {
            stretch.push(c);
        } else {
            out.extend(stretch.drain(..).rev());
            out.push(c);
        }
    }
    out.extend(stretch.drain(..).rev());
}

/// `text` with each `{name}` placeholder and each keyboard shortcut (`⇧⌘P`) replaced by one
/// stand-in character, and what they replaced, in order. A shortcut's symbols have no direction
/// of their own, so left alone they would be drawn in reverse.
fn mask_placeholders(text: &str) -> (String, Vec<&str>) {
    let mut masked = String::with_capacity(text.len());
    let mut names: Vec<&str> = Vec::new();
    let mut rest = text;
    while names.len() < MAX_PLACEHOLDERS
        && let Some((start, end)) = next_atom(rest)
    {
        let (Some(before), Some(atom), Some(after)) = (rest.get(..start), rest.get(start..end), rest.get(end..)) else { break };
        // `names.len()` is below MAX_PLACEHOLDERS, so the sum is a private-use scalar.
        let Some(stand_in) = u32::try_from(names.len()).ok().and_then(|i| char::from_u32(PLACEHOLDER_BASE.saturating_add(i))) else { break };
        masked.push_str(before);
        masked.push(stand_in);
        names.push(atom);
        rest = after;
    }
    masked.push_str(rest);
    (masked, names)
}

/// The byte range of the first placeholder or keyboard shortcut in `text`.
fn next_atom(text: &str) -> Option<(usize, usize)> {
    let placeholder = text.find('{').and_then(|open| {
        let len = text.get(open..)?.find('}')?;
        Some((open, open.saturating_add(len).saturating_add(1)))
    });
    let shortcut = text.find(is_modifier_key).map(|at| {
        // The shortcut is the stretch around the symbol up to a space, a bracket or script text.
        let outside = |c: char| c.is_whitespace() || is_rtl_script(c) || matches!(c, '(' | ')' | '[' | ']' | '{' | '}');
        let start = text
            .get(..at)
            .and_then(|before| before.rfind(outside).and_then(|i| before.get(i..)?.chars().next().map(|c| i.saturating_add(c.len_utf8()))))
            .unwrap_or(0);
        let end = text.get(at..).and_then(|after| after.find(outside)).map_or(text.len(), |i| at.saturating_add(i));
        (start, end)
    });
    match (placeholder, shortcut) {
        (Some(p), Some(s)) => Some(if s.0 < p.0 { s } else { p }),
        (p, s) => p.or(s),
    }
}

/// The modifier-key symbols of keyboard shortcuts: ⌘ ⇧ ⌥ ⌃.
fn is_modifier_key(c: char) -> bool {
    matches!(c, '\u{2318}' | '\u{21E7}' | '\u{2325}' | '\u{2303}')
}

/// Characters of the right-to-left scripts' blocks: letters, marks and the scripts' own
/// punctuation, all drawn by the script's face.
pub fn is_rtl_script(c: char) -> bool {
    matches!(u32::from(c), 0x0590..=0x08FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFC | 0x1_0800..=0x1_0FFF | 0x1_E800..=0x1_EFFF)
}

/// The zero-width (non-)joiners inside a word: egui shapes them with the letter before them.
fn is_joiner(c: char) -> bool {
    matches!(c, '\u{200C}' | '\u{200D}')
}

/// Direction marks, embeddings, overrides and isolates: they steer the reordering and are not drawn.
fn is_bidi_control(c: char) -> bool {
    matches!(c, '\u{061C}' | '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
}

fn mirrored(c: char) -> char {
    match c {
        '(' => ')',
        ')' => '(',
        '[' => ']',
        ']' => '[',
        '{' => '}',
        '}' => '{',
        '<' => '>',
        '>' => '<',
        '«' => '»',
        '»' => '«',
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_without_rtl_script_is_unchanged() {
        for s in ["", "Save", "New\nSlide", "日本語", "a (b) [c]", "Slide {n} of {total}", "caf\u{00E9} \u{200E}x"] {
            assert_eq!(display_rtl(s), s);
        }
    }

    #[test]
    fn hebrew_labels_are_put_in_display_order() {
        // One word is already in display order.
        assert_eq!(display_rtl("שמירה"), "שמירה");
        // "Save As…": the words swap and the ellipsis ends the phrase on the left.
        assert_eq!(display_rtl("שמירה בשם…"), "…בשם שמירה");
        // Latin names keep their own direction inside the phrase.
        assert_eq!(display_rtl("אודות DeckCraft"), "DeckCraft אודות");
        assert_eq!(display_rtl("ייצוא ל-PDF עכשיו"), "עכשיו PDF-ל ייצוא");
        // Numbers read left to right where they stand.
        assert_eq!(display_rtl("תקף למשך 5 שנים"), "שנים 5 למשך תקף");
        // Brackets mirror with the text they wrap.
        assert_eq!(display_rtl("טקסט (רגיל)"), "(רגיל) טקסט");
    }

    #[test]
    fn shortcuts_and_placeholders_stay_whole() {
        assert_eq!(display_rtl("הדפסה (⌘P)"), "(⌘P) הדפסה");
        assert_eq!(display_rtl("סיבוב (⇧⌘+)"), "(⇧⌘+) סיבוב");
        assert_eq!(display_rtl("לא ניתן לפתוח את {name}: {e}"), "{name}: {e} את לפתוח ניתן לא");
        assert_eq!(display_rtl("שקופית {n} מתוך {total}"), "{total} מתוך {n} שקופית");
    }

    #[test]
    fn edges_and_caption_colons_stay_put() {
        // A caption's colon stays beside the widget on its right.
        assert_eq!(display_rtl("שם:"), "שם:");
        assert_eq!(display_rtl("השם שלך (הערות):"), "(הערות) שלך השם:");
        // Spaces the code appends to stay at the end.
        assert_eq!(display_rtl("הונפק על ידי: "), "ידי על הונפק: ");
        // Each line of a two-line ribbon label is reordered on its own.
        assert_eq!(display_rtl("שקופית\nחדשה"), "שקופית\nחדשה");
        assert_eq!(display_rtl("עיצוב שקופית\nחדש"), "שקופית עיצוב\nחדש");
    }
}
