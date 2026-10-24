//! The interface language as a user sees it: the `app.language` command, the setting that
//! survives a restart, and the ribbon relabelling itself in Hebrew through real egui frames.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use deckcraft_engine::Session;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use serde_json::json;

use crate::{Services, SlideApp, UiState, i18n};

fn new_app() -> SlideApp {
    SlideApp::new(Session::with_new(), Services::default())
}

fn harness(app: SlideApp) -> Harness<'static, SlideApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_ui_state(
        |ui, app: &mut SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app,
    );
    h.run_steps(3);
    h
}

/// `app.language` sets, reports and validates the interface language, and `auto` resolves to a
/// language (English under test, whatever the developer's locale is).
#[test]
fn language_command_sets_reports_and_validates() {
    let mut app = new_app();
    let r = app.run("app.language", json!({})).unwrap();
    assert_eq!(r["language"], "auto");
    assert_eq!(r["shown"], "en");
    let codes: Vec<&str> = r["languages"].as_array().unwrap().iter().map(|l| l["code"].as_str().unwrap()).collect();
    assert_eq!(codes.first().copied(), Some("en"), "English is listed first");
    assert!(codes.contains(&"he"), "{codes:?}");

    let r = app.run("app.language", json!({"language": "HE"})).unwrap();
    assert_eq!((r["language"].as_str(), r["shown"].as_str()), (Some("he"), Some("he")), "codes are normalised");
    assert_eq!(app.ui.language, "he");
    assert_eq!(i18n::current().code(), "he", "the change applies at once, not next frame");
    assert_eq!(i18n::t("Home"), "בית");

    let e = app.run("app.language", json!({"language": "klingon"})).unwrap_err();
    assert!(e.contains("klingon") && e.contains("auto") && e.contains("he"), "{e}");
    assert_eq!(app.ui.language, "he", "an unknown code keeps the setting");

    app.run("app.language", json!({"language": "auto"})).unwrap();
    assert_eq!(app.ui.language, "auto");
    assert_eq!(i18n::current(), i18n::Lang::EN);
    // The command is in the palette's table and documents its parameters.
    assert!(crate::UI_COMMANDS.iter().any(|c| c.0 == "app.language" && c.3.contains("language?")));
}

/// `ui.json` keeps the language; a value from a newer version or a hand edit means Auto.
#[test]
fn language_setting_round_trips_and_sanitizes() {
    let saved = UiState { language: "he".into(), ..Default::default() };
    let json = serde_json::to_string(&saved).unwrap();
    assert!(json.contains("\"language\":\"he\""), "{json}");
    let back: UiState = serde_json::from_str(&json).unwrap();
    assert_eq!(back.sanitized().language, "he");
    for junk in ["klingon", "", "iw", "HE "] {
        let s = UiState { language: junk.into(), ..Default::default() }.sanitized();
        assert_eq!(s.language, i18n::AUTO, "{junk:?}");
    }
    assert_eq!(UiState { language: "He".into(), ..Default::default() }.sanitized().language, "he");
    // An old ui.json without the field is Auto.
    let old: UiState = serde_json::from_str(r#"{"zoom": 1.0}"#).unwrap();
    assert_eq!(old.language, i18n::AUTO);
}

/// With Hebrew chosen the ribbon's buttons are named in Hebrew (the accessibility names the tests
/// and screen readers read), and English again after switching back; the toolbar keeps working
/// because it stores the English label.
#[test]
fn ribbon_relabels_in_hebrew_and_back() {
    let mut app = new_app();
    app.run("app.language", json!({"language": "he"})).unwrap();
    let mut h = harness(app);
    // Home ▸ New Slide ("שקופית\nחדשה" is spoken as one line).
    assert!(h.query_by_label("שקופית חדשה").is_some(), "New Slide in Hebrew");
    assert!(h.query_by_label("New Slide").is_none(), "no English duplicate");
    // The File tab and the status bar's counter.
    assert!(h.query_by_label("קובץ").is_some(), "File tab");
    assert!(h.state().ui.qat.iter().all(|q| q.label.is_ascii()), "the toolbar stores English labels: {:?}", h.state().ui.qat);

    h.state_mut().run("app.language", json!({"language": "en"})).unwrap();
    h.run_steps(2);
    assert!(h.query_by_label("New Slide").is_some());
    assert!(h.query_by_label("שקופית חדשה").is_none());
}

/// Preferences ▸ Interface language shows the picker with Auto and every registered language,
/// and Cancel puts the previous language back.
#[test]
fn preferences_picker_lists_languages_and_cancel_restores() {
    let mut app = new_app();
    app.run("app.preferences", json!({})).unwrap();
    let mut h = harness(app);
    assert!(h.query_by_label("Interface language").is_some());
    // The combo box carries its selected text as a value, not a label: find it by role.
    h.get_by_role(egui::accesskit::Role::ComboBox).click();
    h.run_steps(2);
    assert!(h.query_by_label("עברית").is_some(), "Hebrew is offered by its own name");
    assert!(h.query_by_label("English").is_some());
    h.get_by_label("עברית").click();
    h.run_steps(2);
    assert_eq!(h.state().ui.language, "he");
    assert!(h.query_by_label("ביטול").is_some(), "the dialog relabels at once (Cancel)");
    h.get_by_label("ביטול").click();
    h.run_steps(2);
    assert_eq!(h.state().ui.language, "auto", "Cancel restores the language the dialog opened with");
    assert!(h.state().dialog.is_none());
}
