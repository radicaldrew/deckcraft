# Localization parity with PowerPoint

> **Last reviewed:** 2026-10-25 · **Last updated:** 2026-10-25 · **Change:** major (string catalog, Auto/system language, Preferences picker, `app.language`, `cargo xtask i18n-report`; Hebrew interface: 1,310 rows covering every known string, right to left through display-order reordering, not mirrored; the web build embeds Noto Sans Hebrew, #81; previous: minor, IME composition underlined in table cells; previous: minor, IME composition and CJK interface text, #101; previous: major, created) · **Target:** Microsoft PowerPoint for Mac 16.113 (Microsoft 365)

DeckCraft's interface has a string catalog since 2026-10-25 (`crates/ui-egui/src/i18n`, the system
PdfCraft and WordCraft use): English strings in the code are the keys, one `xx.tsv` per language
maps them to display text at draw time, and the language follows the system (`auto`) or the
Preferences ▸ Interface language setting (also `app.language` from the control channel and MCP).
Hebrew is the first non-English language. Requests: #15 (follow the system language, done), #30
and PR #34 (Ukrainian, open; now a matter of adding `uk.tsv`). Dimension score: **~15% ready**
(infrastructure done, one of twelve key languages at partial, right-to-left layout not mirrored),
**45–75 h** to parity plus native-speaker review.

## How the numbers are measured

- **UI strings:** 1,310 catalog keys (measured 2026-10-25 by `translation_gaps_report`: the
  `tl!(…)` literals and label tables of `crates/ui-egui/src`, the ribbon tabs, the UI and engine
  command labels and menu paths, the dialog, pane and File-tab titles, the menu tree and the
  toolbar). Not keys yet: the engine's own error and status sentences (shown in English), the
  proofing-language names (shown in their own languages) and the built-in layout names.
  `cargo xtask i18n-report` prints the per-language gaps; `DECKCRAFT_I18N_DUMP=<file>` writes the
  key list for translators.
- **Script support** is about slide text (what users type and import), separately from the UI:
  bidi and shaping live in `crates/text` (UAX #9, HarfRust, UAX #14 line breaking).
- **PowerPoint's languages:** `Contents/Resources/*.lproj` in PowerPoint 16.113.4: ar, cs, da, de,
  el, en, en_GB, es, es_MX, fi, fr, fr_CA, he, hu, id, it, ja, ko, nl, no, pl, pt (Brazil), pt_PT,
  ru, sk, sv, th, tr, zh_CN, zh_TW (26 languages). PowerPoint for **Mac** has no Hindi or Vietnamese
  UI; PowerPoint for Windows has both.

## The twelve key languages

| Language | Code | UI strings translated | Dialogs / tooltips / help | Script support (slide text) | Native review | Status | PowerPoint for Mac UI | Hours to full |
|---|---|---|---|---|---|---|---|---|
| English | en | 1,310 / 1,310 (100%) | yes / yes / no help | full | yes | **full** | yes | 0 |
| Hebrew | he | 1,310 / 1,310 (100%) | yes / yes / no help | bidi, shaping, RTL bullets and caret, Hebrew numbering; Noto Sans Hebrew embedded on desktop and web (#81); **UI not mirrored** (labels read right to left inside a left-to-right layout) | no (AI-assisted, clean-room) | partial | yes | 8–12 (mirroring 6–10, native review 2) |
| Simplified Chinese (Mandarin) | zh-Hans | 0 (0%) | no | renders with system CJK fonts (asset policy excludes Noto CJK from builds); UAX #14 breaking; IME composition (#101); no true vertical layout | no | none | yes | 6–10 (incl. IME) |
| Spanish | es | 0 (0%) | no | full (Latin) | no | none | yes (es, es-MX) | 3–5 |
| Hindi | hi | 0 (0%) | no | Devanagari shaping through HarfRust, untested; no bundled Devanagari font | no | none | **no** (Windows only) | 4–7 |
| Arabic | ar | 0 (0%) | no | bidi, contextual shaping, kashida justification, RTL bullets and caret (#4); **UI not mirrored**; browser export shows boxes (#81) | no | none | yes | 10–16 (incl. UI mirroring) |
| French | fr | 0 (0%) | no | full (Latin) | no | none | yes (fr, fr-CA) | 3–5 |
| Portuguese | pt | 0 (0%) | no | full (Latin) | no | none | yes (pt-BR, pt-PT) | 3–5 |
| Indonesian | id | 0 (0%) | no | full (Latin) | no | none | yes | 3–5 |
| Japanese | ja | 0 (0%) | no | renders with system or craft-fonts Japanese faces; IME composition (#101); vertical text is rotated, not laid out vertically | no | none | yes | 4–7 |
| German | de | 0 (0%) | no | full (Latin) | no | none | yes | 3–5 |
| Korean | ko | 0 (0%) | no | system Hangul fonts, in the interface too; IME composition (#101) | no | none | yes | 4–7 |
| Vietnamese | vi | 0 (0%) | no | stacked diacritics through HarfRust | no | none | **no** (Windows only) | 3–5 |

Other languages shipped: **Hebrew** (above; Ukrainian is in review as PR #34). PowerPoint for Mac
ships 15 more than the thirteen above: Czech, Danish, Greek, Finnish, Hungarian, Italian, Dutch,
Norwegian, Polish, Russian, Slovak, Swedish, Thai, Turkish and Traditional Chinese.

The infrastructure landed 2026-10-25 (`crates/ui-egui/src/i18n/mod.rs` documents it): a TSV
catalog per language with plain, `@id`, `@plural` and contextual rows, validated when it loads (a
bad row shows in English, never breaks the UI); `tl!("…")` at every draw site; the language setting
(`auto` follows `LANG`/`LC_*`, the macOS preferred languages, the Windows display language or the
browser's `navigator.languages`); `Lang::glyph_sample` so a language's own characters borrow an
interface face when none is embedded (#101); and the tests `hebrew_covers_every_known_string`
(fails when a new string has no Hebrew row) and `translation_gaps_report` (`cargo xtask
i18n-report`, informational, a `ci` step). Adding a language is one `xx.tsv` and one registry row.
Right-to-left languages get their labels reordered into display order at load
(`crates/ui-egui/src/bidi.rs`), the way PdfCraft's Arabic and Hebrew do; mirroring the ribbon,
panes and dialogs themselves is the remaining right-to-left work. IME composition landed with
#101 (slide text, table cells, notes and text fields; one undo step per composition, underlined in
place).

## Proofing languages

PowerPoint checks spelling in dozens of languages and lets each run carry a proofing language.
DeckCraft keeps `a:rPr/@lang` on round trip but its spelling check is a small English list of
common misspellings (`crates/engine/src/cmd/review.rs`); see [gaps.md](gaps.md#proofing-spelling-thesaurus-autocorrect).

## Revision history

| Date | Change | Summary |
|---|---|---|
| 2026-10-25 | major | String catalog (`crates/ui-egui/src/i18n`), Auto/system language, Preferences ▸ Interface language, `app.language`, `cargo xtask i18n-report`; Hebrew interface, 1,310 rows for 1,310 keys, right to left by display-order reordering, not mirrored; Noto Sans Hebrew embedded in the web build (#81) |
| 2026-10-14 | minor | IME composition in a table cell is underlined in place; the candidate window sits on that cell's caret |
| 2026-10-12 | minor | IME composition for Chinese, Japanese and Korean, and interface text in any installed script (#101) |
| 2026-10-10 | major | Created: twelve-language table, ~1,050 UI strings measured, PowerPoint for Mac's 26 languages from its bundle |
