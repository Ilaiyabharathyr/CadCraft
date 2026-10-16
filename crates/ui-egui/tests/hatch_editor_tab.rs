//! The Hatch Editor contextual Ribbon tab: it opens by itself when hatches alone are selected,
//! shows the hatch's settings, and gives the previous tab back once they are deselected.

use cadcraft_engine::Session;
use cadcraft_engine::doc::Handle;
use cadcraft_ui_egui::{CadApp, Services, ribbon_hatch};
use egui::vec2;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use serde_json::json;

fn harness(french: bool) -> (Harness<'static, CadApp>, Handle, Handle) {
    let mut app = CadApp::new(Session::default(), Services::default());
    app.run("new", json!({})).unwrap();
    app.run("ui.layout.pc", json!({})).unwrap();
    app.ui.start_tab = false;
    app.ui.in_window_menu = false;
    app.integrated_titlebar = true;
    if french {
        app.ui.interface_language = serde_json::from_value(json!("fr")).unwrap();
    }
    let r = app.run("rectang", json!({ "p1": [0, 0], "p2": [20, 10] })).unwrap();
    let rect = Handle::parse_hex(r["handle"].as_str().unwrap()).unwrap();
    let h = app.run("hatch", json!({ "handles": [r["handle"]], "pattern": "SOLID" })).unwrap();
    let hatch = Handle::parse_hex(h["handle"].as_str().unwrap()).unwrap();
    app.ui.ribbon_tab = "annotate".into();
    let mut harness = Harness::builder().with_size(vec2(1500.0, 800.0)).build_ui_state(
        |ui, app: &mut CadApp| {
            app.logic(&ui.ctx().clone());
            app.ui(ui);
        },
        app,
    );
    harness.run_steps(3);
    (harness, hatch, rect)
}

#[test]
fn selecting_a_hatch_opens_the_hatch_editor_and_deselecting_closes_it() {
    let (mut h, hatch, rect) = harness(false);
    assert_eq!(h.state().ui.ribbon_tab, "annotate");
    h.state_mut().session.set_selection(vec![hatch]);
    h.run_steps(3);
    assert_eq!(h.state().ui.ribbon_tab, ribbon_hatch::TAB_ID);
    for label in ["Hatch Editor", "Boundaries", "Pattern", "Origin", "Options", "Close Hatch Editor", "Associative", "Pick Points"] {
        assert!(h.query_by_label(label).is_some(), "{label} shown");
    }

    // A hatch with another object: no Hatch Editor.
    h.state_mut().session.set_selection(vec![hatch, rect]);
    h.run_steps(3);
    assert_eq!(h.state().ui.ribbon_tab, "annotate", "back to the tab shown before");
    assert!(h.query_by_label("Hatch Editor").is_none());

    // Close Hatch Editor deselects.
    h.state_mut().session.set_selection(vec![hatch]);
    h.run_steps(3);
    h.get_by_label("Close Hatch Editor").click();
    h.run_steps(3);
    assert!(h.state().session.selection().is_empty());
    assert_eq!(h.state().ui.ribbon_tab, "annotate");
}

#[test]
fn the_gallery_and_the_associative_toggle_edit_the_hatch() {
    let (mut h, hatch, _) = harness(false);
    h.state_mut().session.set_selection(vec![hatch]);
    h.run_steps(3);
    h.get_by_label("ANSI31").click();
    h.run_steps(3);
    let pattern = |h: &Harness<'static, CadApp>| match &h.state().session.doc().unwrap().entity(hatch).unwrap().kind {
        cadcraft_engine::doc::EntityKind::Hatch(x) => (x.pattern.clone(), x.associative),
        _ => panic!("not a hatch"),
    };
    assert_eq!(pattern(&h).0, "ANSI31");
    let assoc = pattern(&h).1;
    h.get_by_label("Associative").click();
    h.run_steps(3);
    assert_eq!(pattern(&h).1, !assoc);
    // The tab stays while the hatch is selected.
    assert_eq!(h.state().ui.ribbon_tab, ribbon_hatch::TAB_ID);
}

#[test]
fn each_slide_out_opens_under_its_panel_title() {
    let (mut h, hatch, _) = harness(false);
    h.state_mut().session.set_selection(vec![hatch]);
    h.run_steps(3);
    for (title, inside) in
        [("Boundaries", "Display Boundary Objects"), ("Properties", "Double"), ("Origin", "Store as Default Origin"), ("Options", "Separate Hatches")]
    {
        h.get_all_by_label(title).next().unwrap().click();
        h.run_steps(3);
        assert!(h.query_by_label(inside).is_some(), "{title} slides out {inside}");
        h.get_all_by_label(title).next().unwrap().click();
        h.run_steps(3);
    }
}

/// With CRAFT_UI_VISUAL_DIR set: the tab in French with each slide-out, to compare with AutoCAD.
#[test]
fn french_pictures_of_the_hatch_editor() {
    let Ok(directory) = std::env::var("CRAFT_UI_VISUAL_DIR") else { return };
    std::fs::create_dir_all(&directory).unwrap();
    let (mut h, hatch, _) = harness(true);
    h.state_mut().session.set_selection(vec![hatch]);
    h.run_steps(3);
    h.render().unwrap().save(format!("{directory}/hatch-editor.png")).unwrap();
    for title in ["Boundaries", "Propriétés", "Origin", "Options"] {
        h.get_all_by_label(title).next().unwrap().click();
        h.run_steps(3);
        h.render().unwrap().save(format!("{directory}/hatch-editor-{title}.png")).unwrap();
        h.get_all_by_label(title).next().unwrap().click();
        h.run_steps(3);
    }
}
