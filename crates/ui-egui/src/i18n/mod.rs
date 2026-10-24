//! Interface localisation. Strings in code stay English and are the lookup keys; a per-language
//! catalog (`*.tsv`, see `he.tsv` for the format) maps them to display text at draw time. Command
//! ids, presentation text, file names, the control channel, the CLI and MCP never see translated
//! text, so agents and scripts are unaffected. A string without a translation is shown in
//! English, so coverage can grow incrementally; `cargo xtask i18n-report` lists the gaps.
//!
//! The system is the one PdfCraft and WordCraft use (`pdfcraft/crates/ui-egui/src/i18n`), copied
//! rather than depended on, so catalogs and habits carry over between the apps.
//!
//! # Adding a language
//! 1. Add `xx.tsv` next to `he.tsv` (copy its header; translate from the *meaning* of the English
//!    text, clean-room, see `he.tsv`).
//! 2. Add one row to [`LANGUAGES`] (code, native name, catalog, plural rule, direction).
//!
//! That is all: the Preferences dropdown, the system-locale match, `app.language` and the catalog
//! tests (parse, placeholders, plural forms, coverage of the ribbon and the commands) pick it up
//! from the registry.
//!
//! # Looking strings up
//! - `tl!` / [`t`]: a plain string in the current language. [`tr`]: in a given one.
//! - [`tr_ctx`]: when one English word needs different translations in different places.
//! - [`tr_id`] / [`menu_label`]: a command-id keyed string with the English label as fallback
//!   (menus, the palette, the toolbar), so a translation survives rewording of the English text.
//! - [`trn`]: plural-aware (`{n}` is filled in). [`fmt`]: fill `{name}` placeholders after [`tr`];
//!   translators may reorder placeholders freely.

mod catalog;
#[cfg(test)]
pub(crate) mod scan;

use std::cell::Cell;
use std::sync::OnceLock;

use catalog::Catalog;
pub use catalog::{Entry, parse_entries, placeholders};

/// The `language` setting value that follows the system locale.
pub const AUTO: &str = "auto";

/// One supported UI language.
pub struct LangInfo {
    /// BCP 47 code, lowercase (`he`, `zh-hans`, `pt-br`). Also the `language` setting value.
    pub code: &'static str,
    /// The language's name in itself, shown in the Preferences dropdown.
    pub name: &'static str,
    /// Catalog file contents (empty for the built-in English).
    pub source: &'static str,
    /// Plural form index for a count (English: 0 = one, 1 = other). A catalog's `@plural`
    /// entries list one form per index.
    pub plural: fn(u64) -> usize,
    /// The language is written right to left. egui lays text out left to right, so the catalog's
    /// translations are put into display order when it loads ([`crate::bidi::display_rtl`]).
    pub rtl: bool,
    catalog: OnceLock<Catalog>,
}

fn plural_one_other(n: u64) -> usize {
    usize::from(n != 1)
}

/// The registry. English first: it is the fallback and the source language.
pub static LANGUAGES: [LangInfo; 2] = [
    LangInfo { code: "en", name: "English", source: "", plural: plural_one_other, rtl: false, catalog: OnceLock::new() },
    // Hebrew; every `he-*` locale (`he-IL`, `he_IL.UTF-8`) resolves here, and so does the legacy
    // `iw` code (Java, older Android), which has no catalog of its own (see `candidates`). Right
    // to left: see `LangInfo::rtl`. CLDR `he` counts have one, two and other, but after a numeral
    // the two and other forms read the same (`2 שקופיות`, `5 שקופיות`), so the catalog lists two
    // forms (one, other).
    LangInfo { code: "he", name: "עברית", source: include_str!("he.tsv"), plural: plural_one_other, rtl: true, catalog: OnceLock::new() },
];

impl LangInfo {
    /// How many plural forms the language's `@plural` entries list.
    pub fn plural_forms(&self) -> usize {
        (0..=1000).map(self.plural).max().unwrap_or(0).saturating_add(1)
    }

    fn catalog(&self) -> &Catalog {
        self.catalog.get_or_init(|| {
            let (mut catalog, errors) = Catalog::parse(self.source, self.plural_forms());
            for error in errors {
                log::warn!("{} interface catalog: {error}; that entry shows in English", self.code);
            }
            if self.rtl {
                catalog.map_translations(crate::bidi::display_rtl);
            }
            catalog
        })
    }
}

/// A language the UI can be shown in (a handle into [`LANGUAGES`]).
#[derive(Clone, Copy)]
pub struct Lang(&'static LangInfo);

impl std::fmt::Debug for Lang {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Lang({})", self.0.code)
    }
}

impl PartialEq for Lang {
    fn eq(&self, other: &Self) -> bool {
        self.0.code == other.0.code
    }
}

impl Eq for Lang {}

impl Lang {
    pub const EN: Lang = Lang(&LANGUAGES[0]);

    pub fn code(self) -> &'static str {
        self.0.code
    }

    /// A language by its exact code.
    pub fn from_code(code: &str) -> Option<Lang> {
        LANGUAGES.iter().find(|l| l.code.eq_ignore_ascii_case(code)).map(Lang)
    }

    /// Resolve the `language` setting: a language code, or [`AUTO`] (and anything unknown, e.g.
    /// a code from a newer version) to follow the system locale.
    pub fn from_pref(pref: &str) -> Lang {
        Lang::from_code(pref).unwrap_or_else(system_lang)
    }

    /// Every registered language.
    pub fn all() -> impl Iterator<Item = Lang> {
        LANGUAGES.iter().map(Lang)
    }

    /// Every registered language in the order a picker lists them: English first (the source
    /// language and the fallback), then the rest by their own name.
    pub fn for_picker() -> Vec<Lang> {
        let mut rest: Vec<Lang> = Lang::all().filter(|l| l.code() != Lang::EN.code()).collect();
        rest.sort_by_key(|l| l.name().to_lowercase());
        std::iter::once(Lang::EN).chain(rest).collect()
    }

    pub fn name(self) -> &'static str {
        self.0.name
    }

    /// Whether the language is written right to left. Its translations come back from the
    /// lookups in display order (see [`LangInfo::rtl`]).
    pub fn rtl(self) -> bool {
        self.0.rtl
    }

    /// The plural form index for a count.
    pub fn plural_index(self, n: u64) -> usize {
        (self.0.plural)(n)
    }

    /// The catalog's entries as written (logical order), for the coverage tests and reports.
    pub fn entries(self) -> Vec<Entry> {
        parse_entries(self.0.source, self.0.plural_forms()).0
    }

    /// Every character the language's labels draw, the name included, for borrowing an
    /// interface face that has them ([`crate::uifonts`]) when no embedded face does.
    pub fn glyph_sample(self) -> String {
        let mut chars: Vec<char> = self.catalog().translations().flat_map(str::chars).chain(self.name().chars()).collect();
        chars.sort_unstable();
        chars.dedup();
        chars.into_iter().collect()
    }

    fn catalog(self) -> &'static Catalog {
        self.0.catalog()
    }
}

/// The canonical `language` setting for user input: [`AUTO`] or a registered code (any case).
/// `None` for anything else, so callers can keep the current setting.
pub fn normalize_pref(pref: &str) -> Option<&'static str> {
    if pref.eq_ignore_ascii_case(AUTO) {
        return Some(AUTO);
    }
    Lang::from_code(pref).map(Lang::code)
}

/// Candidate language codes for a locale tag, most specific first: `zh_TW.UTF-8` →
/// `zh-tw`, `zh-hant`, `zh`.
fn candidates(tag: &str) -> Vec<String> {
    let base = tag.split(['.', '@']).next().unwrap_or("").replace('_', "-").to_ascii_lowercase();
    let parts: Vec<&str> = base.split('-').filter(|p| !p.is_empty()).collect();
    let Some(&primary) = parts.first() else { return Vec::new() };
    let mut out = Vec::new();
    for n in (1..=parts.len()).rev() {
        out.push(parts.get(..n).unwrap_or_default().join("-"));
    }
    if primary == "zh" && !parts.iter().any(|p| matches!(*p, "hans" | "hant")) {
        // Chinese by region when no script is given.
        let script = if parts.iter().any(|p| matches!(*p, "tw" | "hk" | "mo")) { "zh-hant" } else { "zh-hans" };
        out.insert(out.len().saturating_sub(1), script.to_string());
    }
    if primary == "iw" {
        // `iw` was Hebrew's ISO 639-1 code until 1989 and Java and older Android still report it;
        // the catalog is `he`. Last, so a catalog of its own would win.
        out.push("he".to_string());
    }
    out
}

/// The registered language for a locale tag such as `he_IL.UTF-8`, `he-IL`; `None` if the
/// language isn't supported. `C`/`POSIX` mean English.
pub fn lang_from_tag(tag: &str) -> Option<Lang> {
    let cands = candidates(tag);
    if matches!(cands.first().map(String::as_str), Some("c" | "posix")) {
        return Some(Lang::EN);
    }
    cands.iter().find_map(|c| Lang::from_code(c))
}

/// The system language (cached). English when it can't be determined.
pub fn system_lang() -> Lang {
    // Tests drive the UI by its English labels whatever the developer's locale is (the kittest
    // harnesses are unit tests of this crate, so `cfg!(test)` covers them).
    if cfg!(test) {
        return Lang::EN;
    }
    static SYSTEM: OnceLock<Lang> = OnceLock::new();
    *SYSTEM.get_or_init(detect_system_lang)
}

#[cfg(not(target_arch = "wasm32"))]
fn detect_system_lang() -> Lang {
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Some(l) = std::env::var(var).ok().and_then(|v| language_override(&v)) {
            return l;
        }
    }
    // Apps started from the Finder don't inherit LANG: use the macOS preferred-languages list.
    // The absolute path keeps a `defaults` earlier on PATH from running; any failure means English.
    #[cfg(target_os = "macos")]
    if let Ok(out) = std::process::Command::new("/usr/bin/defaults").args(["read", "-g", "AppleLanguages"]).output()
        && out.status.success()
        && let Some(l) = first_supported(&String::from_utf8_lossy(&out.stdout))
    {
        return l;
    }
    #[cfg(target_os = "windows")]
    if let Some(l) = windows_ui_language().as_deref().and_then(first_supported) {
        return l;
    }
    Lang::EN
}

/// A named locale can override the display language. On Windows, the neutral
/// C/POSIX locale inherited from a launcher says nothing about the user's UI language.
#[cfg(not(target_arch = "wasm32"))]
fn language_override(tag: &str) -> Option<Lang> {
    let tag = tag.trim();
    if cfg!(target_os = "windows") && matches!(candidates(tag).first().map(String::as_str), Some("c" | "posix")) {
        return None;
    }
    lang_from_tag(tag)
}

/// The Windows display languages in preference order, one tag per line, queried once when Auto
/// first resolves. `sys-locale` asks Windows directly (`GetUserPreferredUILanguages`), so no
/// process is started.
#[cfg(target_os = "windows")]
fn windows_ui_language() -> Option<String> {
    let tags: Vec<String> = sys_locale::get_locales().collect();
    (!tags.is_empty()).then(|| tags.join("\n"))
}

/// The first supported language in a macOS `defaults read` list or Windows UI-culture output.
#[cfg_attr(not(any(target_os = "macos", target_os = "windows")), allow(dead_code))]
fn first_supported(list: &str) -> Option<Lang> {
    list.split(['(', ')', ',', '"', '\n']).map(str::trim).filter(|s| !s.is_empty()).find_map(lang_from_tag)
}

/// The first supported language among BCP 47 tags in preference order, as the browser lists them
/// in `navigator.languages`. Only the first few tags are looked at: the list is short in
/// practice, and a page can't make Auto walk an unbounded one.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn first_supported_tag<S: AsRef<str>>(tags: impl IntoIterator<Item = S>) -> Option<Lang> {
    tags.into_iter().take(32).find_map(|tag| lang_from_tag(tag.as_ref().trim()))
}

/// The browser's preferred languages, as the desktop builds use the system's: `navigator.languages`
/// in order, then `navigator.language`. English when the browser offers none we have a catalog for.
#[cfg(target_arch = "wasm32")]
fn detect_system_lang() -> Lang {
    let Ok(navigator) = js_sys::Reflect::get(&js_sys::global(), &"navigator".into()) else { return Lang::EN };
    let mut tags: Vec<String> = Vec::new();
    if let Ok(languages) = js_sys::Reflect::get(&navigator, &"languages".into())
        && js_sys::Array::is_array(&languages)
    {
        tags.extend(js_sys::Array::from(&languages).iter().take(31).filter_map(|v| v.as_string()));
    }
    if let Some(language) = js_sys::Reflect::get(&navigator, &"language".into()).ok().and_then(|v| v.as_string()) {
        tags.push(language);
    }
    first_supported_tag(&tags).unwrap_or(Lang::EN)
}

thread_local! {
    // Per thread: independent app and test threads (kittest harnesses run in parallel) must not
    // change each other's drawing language.
    static CURRENT: Cell<Lang> = const { Cell::new(Lang::EN) };
}

/// Set the UI language for drawing (the shell calls this once per frame from the setting), so
/// widgets can translate without every call site carrying a language around.
pub fn set_current(lang: Lang) {
    CURRENT.set(lang);
}

/// The language the UI is drawn in.
pub fn current() -> Lang {
    CURRENT.get()
}

/// Does `lang` have a catalog entry for this plain string? (English never does: it is the source.)
pub fn has(lang: Lang, s: &str) -> bool {
    lang.catalog().plain(s).is_some()
}

/// Translate an English UI string into the current language ([`tr`] with [`current`]).
pub fn t(s: &str) -> &str {
    tr(current(), s)
}

/// Translate an English UI string; unknown strings come back unchanged.
pub fn tr(lang: Lang, s: &str) -> &str {
    lang.catalog().plain(s).unwrap_or(s)
}

/// Like [`tr`], for an English string that needs a disambiguating `context`.
pub fn tr_ctx<'a>(lang: Lang, context: &str, s: &'a str) -> &'a str {
    lang.catalog().contextual(context, s).unwrap_or_else(|| tr(lang, s))
}

/// A string keyed by its command id, falling back to the translation of the English `label`.
pub fn tr_id<'a>(lang: Lang, id: &str, label: &'a str) -> &'a str {
    lang.catalog().id(id).unwrap_or_else(|| tr(lang, label))
}

/// A registered command's label in the current language: its `@id` catalog entry if there is
/// one, otherwise the translation of its English `label` ([`command_label`], so "Undo Typing"
/// translates by its parts).
pub fn menu_label(id: &str, label: &str) -> String {
    match current().catalog().id(id) {
        Some(translated) => translated.to_owned(),
        None => command_label(label),
    }
}

/// A command label such as "Undo Insert Slide" in the current language. A catalog whose word
/// order doesn't put the verb first translates "Undo {action}" and "Redo {action}" as whole
/// phrases; otherwise the translated verb goes before the translated action.
pub fn command_label(text: &str) -> String {
    for (prefix, template) in [("Undo", "Undo {action}"), ("Redo", "Redo {action}")] {
        if let Some(action) = text.strip_prefix(prefix).and_then(|tail| tail.strip_prefix(' ')) {
            let action = t(action);
            if has(current(), template) {
                return fmt(t(template), &[("action", action)]);
            }
            return format!("{} {action}", t(prefix));
        }
    }
    t(text).to_owned()
}

/// Fill `{name}` placeholders in one pass. Unknown placeholders are left as written, and inserted
/// values are never read as templates again, so a file name containing `{n}` stays intact.
pub fn fmt(template: &str, args: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some((before, after_open)) = rest.split_once('{') {
        out.push_str(before);
        let Some((name, after_close)) = after_open.split_once('}') else {
            out.push('{');
            out.push_str(after_open);
            return out;
        };
        match args.iter().find(|(key, _)| *key == name) {
            Some((_, value)) => out.push_str(value),
            None => {
                out.push('{');
                out.push_str(name);
                out.push('}');
            }
        }
        rest = after_close;
    }
    out.push_str(rest);
    out
}

/// A plural-aware message: `one`/`other` are the English forms (with `{n}` where the count goes).
pub fn trn(lang: Lang, n: u64, one: &str, other: &str) -> String {
    let idx = lang.plural_index(n);
    let text = lang.catalog().plural(one, other, idx).unwrap_or(if n == 1 { one } else { other });
    fmt(text, &[("n", &n.to_string())])
}

/// [`trn`] in the current language.
pub fn tn(n: u64, one: &str, other: &str) -> String {
    trn(current(), n, one, other)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hebrew() -> Lang {
        Lang::from_code("he").expect("he registered")
    }

    /// A catalog of its own, so the lookups are tested apart from what `he.tsv` says.
    static TEST_LANG: LangInfo = LangInfo {
        code: "xx",
        name: "Test",
        source: "\tSave\tSpeichern\n\tSave As…\tSpeichern unter…\nmenu\tFormat\tFormatieren\n@id\tslide.new\tNeue Folie\n@plural\t{n} slide|{n} slides\t{n} Folie|{n} Folien\n\tUndo {action}\t{action} rückgängig\n\tTyping\tEingabe\n\tRedo\tWiederholen\n",
        plural: plural_one_other,
        rtl: false,
        catalog: OnceLock::new(),
    };

    #[test]
    fn tags_map_to_languages() {
        let he = hebrew();
        for tag in ["he", "he-IL", "he_IL.UTF-8", "HE-il", "iw", "iw-IL", "iw_IL.UTF-8"] {
            assert_eq!(lang_from_tag(tag), Some(he), "{tag}");
        }
        assert_eq!(lang_from_tag("en_US.UTF-8"), Some(Lang::EN));
        assert_eq!(lang_from_tag("C"), Some(Lang::EN));
        assert_eq!(lang_from_tag("POSIX"), Some(Lang::EN));
        assert_eq!(lang_from_tag("fr_FR"), None, "no French catalog yet");
        assert_eq!(lang_from_tag(""), None);
        assert_eq!(lang_from_tag("_"), None);
        assert_eq!(candidates("zh_TW"), ["zh-tw", "zh-hant", "zh"]);
        assert_eq!(candidates("iw_IL.UTF-8"), ["iw-il", "iw", "he"]);
        assert_eq!(first_supported("(\n    \"he-IL\",\n    \"en-US\"\n)\n"), Some(he));
        assert_eq!(first_supported("he-IL\r\nen-US"), Some(he));
        assert_eq!(first_supported("("), None);
        assert_eq!(first_supported_tag(["xx-YY", "he", "en-US"]), Some(he));
        assert_eq!(first_supported_tag(["en-GB", "he-IL"]), Some(Lang::EN));
        assert_eq!(first_supported_tag(Vec::<String>::new()), None);
        let long = std::iter::repeat_n("xx", 10_000).chain(["he"]);
        assert_eq!(first_supported_tag(long), None, "only the first tags are read");
    }

    #[test]
    fn preferences_resolve_with_fallback() {
        assert_eq!(normalize_pref("HE"), Some("he"));
        assert_eq!(normalize_pref("Auto"), Some(AUTO));
        assert_eq!(normalize_pref("iw"), None, "a locale tag, not a setting value");
        assert_eq!(normalize_pref("klingon"), None);
        assert_eq!(Lang::from_pref("he"), hebrew());
        // Tests always see English for Auto and for unknown values.
        assert_eq!(Lang::from_pref(AUTO), Lang::EN);
        assert_eq!(Lang::from_pref("klingon"), Lang::EN);
        assert_eq!(Lang::for_picker().first().copied(), Some(Lang::EN));
        assert!(Lang::for_picker().contains(&hebrew()));
    }

    #[test]
    fn lookups_fall_back_to_english() {
        let xx = Lang(&TEST_LANG);
        assert_eq!(tr(xx, "Save"), "Speichern");
        assert_eq!(tr(xx, "Open"), "Open");
        assert_eq!(tr_ctx(xx, "menu", "Format"), "Formatieren");
        assert_eq!(tr_ctx(xx, "other", "Format"), "Format", "no plain row either");
        assert_eq!(tr_ctx(xx, "other", "Save"), "Speichern", "falls back to the plain row");
        assert_eq!(tr_id(xx, "slide.new", "New Slide"), "Neue Folie");
        assert_eq!(tr_id(xx, "slide.delete", "Save"), "Speichern", "falls back to the label's translation");
        assert_eq!(trn(xx, 1, "{n} slide", "{n} slides"), "1 Folie");
        assert_eq!(trn(xx, 3, "{n} slide", "{n} slides"), "3 Folien");
        assert_eq!(trn(xx, 2, "{n} page", "{n} pages"), "2 pages", "English forms when untranslated");
        assert_eq!(tr(Lang::EN, "Save"), "Save");
        assert!(has(xx, "Save") && !has(xx, "Open") && !has(Lang::EN, "Save"));
    }

    #[test]
    fn command_labels_translate_by_their_parts() {
        let previous = current();
        set_current(Lang(&TEST_LANG));
        assert_eq!(command_label("Undo Typing"), "Eingabe rückgängig", "the whole-phrase template wins");
        assert_eq!(command_label("Redo Typing"), "Wiederholen Eingabe", "verb first when there is no template");
        assert_eq!(command_label("Save"), "Speichern");
        assert_eq!(menu_label("slide.new", "New Slide"), "Neue Folie");
        assert_eq!(menu_label("edit.undo", "Undo Typing"), "Eingabe rückgängig");
        assert_eq!(t("Save As…"), "Speichern unter…");
        assert_eq!(tn(1, "{n} slide", "{n} slides"), "1 Folie");
        set_current(previous);
        assert_eq!(command_label("Undo Typing"), "Undo Typing");
    }

    #[test]
    fn formatting_reorders_parameters_without_reinterpreting_user_text() {
        assert_eq!(fmt("{a} and {b}", &[("a", "x"), ("b", "y")]), "x and y");
        assert_eq!(fmt("{b} before {a}", &[("a", "x"), ("b", "y")]), "y before x");
        assert_eq!(fmt("{name} opened", &[("name", "deck {n}.pptx")]), "deck {n}.pptx opened");
        assert_eq!(fmt("{missing} stays", &[]), "{missing} stays");
        assert_eq!(fmt("unclosed {brace", &[]), "unclosed {brace");
    }

    #[test]
    fn current_language_is_per_thread() {
        let previous = current();
        set_current(hebrew());
        let other = std::thread::spawn(current).join().expect("thread");
        assert_eq!(other, Lang::EN);
        assert_eq!(current(), hebrew());
        set_current(previous);
    }

    #[test]
    fn malformed_lines_are_reported_not_fatal() {
        let text =
            "\tSave\tשמירה\nbad line\n\tOpen {name}\tפתיחה {nme}\n\tSave\tagain\n@plural\ta|b\tonly one\n@weird\tx\ty\n\tDone…\tסיום\n\tOK\tאישור\n";
        let (catalog, errors) = Catalog::parse(text, 2);
        assert_eq!(errors.len(), 6, "{errors:?}");
        assert!(errors.iter().any(|e| e.starts_with("line 2:")), "{errors:?}");
        assert!(errors.iter().any(|e| e.contains("placeholders")), "{errors:?}");
        assert!(errors.iter().any(|e| e.contains("duplicate")), "{errors:?}");
        assert!(errors.iter().any(|e| e.contains("plural")), "{errors:?}");
        assert!(errors.iter().any(|e| e.contains("ellipsis")), "{errors:?}");
        assert_eq!(catalog.plain("Save"), Some("שמירה"), "the first entry wins");
        assert_eq!(catalog.plain("OK"), Some("אישור"), "lines after a bad one still load");
        assert_eq!(catalog.plain("Open {name}"), None);
    }

    #[test]
    fn hebrew_is_registered_right_to_left_and_in_display_order() {
        let he = hebrew();
        assert_eq!(he.name(), "עברית");
        assert!(he.rtl());
        assert_eq!((0..=3).map(|n| he.plural_index(n)).collect::<Vec<_>>(), [1, 0, 1, 1]);
        // Unknown strings and presentation text are never reordered by a lookup.
        assert_eq!(tr(he, "דוח סופי {n}.pptx"), "דוח סופי {n}.pptx");
        // "שמירה בשם…" as typed in the catalog; the ellipsis ends the phrase on the left.
        assert_eq!(tr(he, "Save As…"), "…בשם שמירה");
        // Latin names keep their own direction inside the phrase.
        assert_eq!(tr(he, "About DeckCraft"), "DeckCraft אודות");
        // Placeholders survive and are filled in place, the count where it is read.
        assert_eq!(fmt(tr(he, "Slide {n} of {total}"), &[("n", "3"), ("total", "12")]), "12 מתוך 3 שקופית");
        assert!(he.glyph_sample().contains('א') && he.glyph_sample().contains('ת'));
    }

    /// Every bundled catalog parses without a single error, has no stray code, and its `@id`
    /// rows name real commands with the English label's placeholders and ellipsis.
    #[test]
    fn bundled_catalogs_are_consistent() {
        let mut codes = std::collections::HashSet::new();
        for l in &LANGUAGES {
            assert!(codes.insert(l.code), "duplicate code {}", l.code);
            assert!(l.code == l.code.to_ascii_lowercase() && !l.name.is_empty(), "{}", l.code);
            let (entries, errors) = parse_entries(l.source, l.plural_forms());
            assert!(errors.is_empty(), "{}: {errors:?}", l.code);
            for e in entries.iter().filter(|e| e.context == "@id") {
                let label = deckcraft_engine::find_command(&e.source)
                    .map(|c| c.label)
                    .or_else(|| crate::UI_COMMANDS.iter().find(|c| c.0 == e.source).map(|c| c.1));
                let Some(label) = label else { panic!("{}: unknown command id {:?}", l.code, e.source) };
                assert_eq!(placeholders(&e.translation), placeholders(label), "{}: {}", l.code, e.source);
                assert_eq!(e.translation.ends_with('…'), label.ends_with('…'), "{}: ellipsis mismatch: {}", l.code, e.source);
            }
        }
    }

    /// Every English string the interface can show, as far as the sources tell: the `tl!(…)`
    /// literals of this crate, the ribbon's button labels, the tabs, the UI and engine commands
    /// (labels and menu paths), the dialog, pane and File-tab titles, the menu tree and the
    /// toolbar, plus every key of every bundled catalog (contextual and plural strings the scan
    /// can't see).
    pub(crate) fn english_keys() -> std::collections::BTreeSet<(String, String)> {
        use std::collections::BTreeSet;
        let mut plain: BTreeSet<String> = scan::ui_literals();
        let mut add = |s: &str| {
            if !s.is_empty() {
                plain.insert(s.to_string());
            }
        };
        for c in deckcraft_engine::command_specs() {
            add(c.label);
            c.menu.iter().for_each(|m| add(m));
        }
        for c in crate::UI_COMMANDS {
            add(c.1);
        }
        crate::ribbon::TABS.iter().for_each(|t| add(t));
        crate::dialogs::TITLES.iter().for_each(|(_, t)| add(t));
        crate::panes::TITLES.iter().for_each(|(_, t)| add(t));
        crate::backstage::LABELS.iter().for_each(|t| add(t));
        for (menu, items) in crate::menus::menu_tree() {
            add(menu);
            items.iter().filter(|(label, _)| *label != "-").for_each(|(label, _)| add(label));
        }
        crate::qat::common().iter().for_each(|q| add(&q.label));
        let mut keys: BTreeSet<(String, String)> = plain.into_iter().map(|s| (String::new(), s)).collect();
        for language in &LANGUAGES {
            let (entries, _) = parse_entries(language.source, language.plural_forms());
            keys.extend(entries.into_iter().filter(|e| e.context != "@id").map(|e| (e.context, e.source)));
        }
        keys
    }

    fn missing_in(lang: Lang, keys: &std::collections::BTreeSet<(String, String)>) -> Vec<(String, String)> {
        let catalog = lang.catalog();
        keys.iter()
            .filter(|(context, source)| match context.as_str() {
                "" => catalog.plain(source).is_none(),
                "@plural" => source.split_once('|').is_none_or(|(one, other)| catalog.plural(one, other, 0).is_none()),
                // A contextual lookup falls back to the plain translation.
                context => catalog.contextual(context, source).is_none() && catalog.plain(source).is_none(),
            })
            .cloned()
            .collect()
    }

    /// The translation gaps report (`cargo xtask i18n-report`, a step of `cargo xtask ci`): for
    /// each language, how many English strings it has no translation for, and the first few.
    /// It never fails: a missing string shows in English. `DECKCRAFT_I18N_DUMP=<file>` writes
    /// every key, one per line, for translators.
    #[test]
    fn translation_gaps_report() {
        const SHOWN: usize = 5;
        let keys = english_keys();
        if let Some(path) = std::env::var_os("DECKCRAFT_I18N_DUMP") {
            let text: String = keys.iter().map(|(c, s)| format!("{c}\t{}\n", s.replace('\n', "\\n").replace('\t', "\\t"))).collect();
            std::fs::write(&path, text).expect("write the key dump");
        }
        let mut report = format!("Translation gaps: {} English strings\n", keys.len());
        for lang in Lang::all().filter(|l| *l != Lang::EN) {
            let missing = missing_in(lang, &keys);
            let mut line = format!("  {:<8} {:>5} missing", lang.code(), missing.len());
            for (i, (context, source)) in missing.iter().take(SHOWN).enumerate() {
                let mut shown: String = source.chars().take(48).collect();
                if shown.len() < source.len() {
                    shown.push('…');
                }
                let shown = shown.replace('\n', "\\n");
                line.push_str(if i == 0 { ": " } else { ", " });
                if context.is_empty() {
                    line.push_str(&format!("{shown:?}"));
                } else {
                    line.push_str(&format!("{context}/{shown:?}"));
                }
            }
            if missing.len() > SHOWN {
                line.push_str(", …");
            }
            report.push_str(&line);
            report.push('\n');
        }
        println!("=== translation gaps ===\n{report}=== end of translation gaps ===");
    }

    /// Hebrew covers the whole interface: every key the report knows has a row. The report above
    /// only informs; this test keeps the first non-English language complete as strings are added.
    #[test]
    fn hebrew_covers_every_known_string() {
        let missing = missing_in(hebrew(), &english_keys());
        assert!(missing.is_empty(), "{} strings without a Hebrew row, e.g. {:?}", missing.len(), missing.iter().take(20).collect::<Vec<_>>());
    }

    /// Every character the Hebrew translations add to the English labels has a glyph in the
    /// interface fonts when a craft-fonts Hebrew face is embedded (desktop and web builds).
    #[test]
    fn hebrew_labels_have_glyphs() {
        use egui::epaint::text::{Fonts, TextOptions};
        use std::collections::BTreeSet;
        let he = hebrew();
        let entries = he.entries();
        let drawn = |c: &char| !c.is_control() && !matches!(c, '\u{200E}' | '\u{200F}');
        let english: BTreeSet<char> = entries.iter().flat_map(|e| e.source.chars()).collect();
        let added: BTreeSet<char> =
            entries.iter().flat_map(|e| e.translation.chars()).chain(he.name().chars()).filter(drawn).filter(|c| !english.contains(c)).collect();
        assert!(added.contains(&'א'), "{added:?}");
        if !deckcraft_fonts::CRAFT_FONTS.iter().any(|f| f.family == "Noto Sans Hebrew") {
            eprintln!("skipping the Hebrew glyph check: build with CRAFT_FONTS_DIR and a Hebrew face to run it");
            return;
        }
        let mut fonts = Fonts::new(TextOptions::default(), crate::theme::font_definitions(&[]));
        for id in [crate::theme::font(13.0), crate::theme::bold(13.0), egui::FontId::proportional(11.0)] {
            let missing: Vec<char> = added.iter().copied().filter(|c| !fonts.has_glyphs(&id, &c.to_string())).collect();
            assert!(missing.is_empty(), "{id:?} lacks Hebrew label glyphs {missing:?}");
        }
    }
}
