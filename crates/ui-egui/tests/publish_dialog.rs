//! Publish Sheets: PUBLISH typed or chosen from the menu opens the window; its list, tabs and
//! summary follow the open drawings.

use cadcraft_engine::Session;
use cadcraft_ui_egui::{CadApp, Services};
use egui::vec2;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use serde_json::json;

fn harness(width: f32, height: f32, scale: f32) -> Harness<'static, CadApp> {
    // One drawing (Session::default opens Drawing1) with something in model space, so it has
    // three sheets: Model, Layout1 and Layout2.
    let mut app = CadApp::new(Session::default(), Services::default());
    app.run("line", json!({ "points": [[0, 0], [100, 0]] })).unwrap();
    app.ui.start_tab = false;
    app.ui.in_window_menu = false;
    app.cmdline("PUBLISH");
    assert_eq!(app.ui.dialog.as_deref(), Some("publish"));
    let mut h = Harness::builder().with_size(vec2(width, height)).with_pixels_per_point(scale).build_ui_state(
        |ui, app: &mut CadApp| {
            app.logic(&ui.ctx().clone());
            app.ui(ui);
        },
        app,
    );
    h.run_steps(6);
    h
}

fn save(h: &mut Harness<'static, CadApp>, name: &str) {
    if let Ok(directory) = std::env::var("CRAFT_UI_VISUAL_DIR") {
        std::fs::create_dir_all(&directory).unwrap();
        h.render().unwrap().save(format!("{directory}/{name}.png")).unwrap();
    }
}

#[test]
fn publish_lists_every_sheet_of_the_open_drawing() {
    let mut h = harness(1280.0, 820.0, 1.0);
    save(&mut h, "publish-sheet-tab");
    // The status bar's layout tabs carry the same names, so each appears at least twice.
    for sheet in ["Model", "Layout1", "Layout2"] {
        assert!(h.get_all_by_label(sheet).count() >= 2, "{sheet} is listed");
    }
    h.get_by_label("3 files");
    h.get_by_label("Publish 3 Sheets");
}

#[test]
fn none_clears_the_selection_and_disables_publish() {
    let mut h = harness(1280.0, 820.0, 1.0);
    h.get_by_label("None").click();
    h.run_steps(3);
    h.get_by_label("Nothing selected");
    assert!(h.get_by_label("Publish").accesskit_node().is_disabled());
    h.get_by_label("All").click();
    h.run_steps(3);
    h.get_by_label("Publish 3 Sheets");
}

#[test]
fn output_tab_offers_combining_sheets_of_one_drawing() {
    let mut h = harness(1280.0, 820.0, 1.0);
    h.get_by_label("Output").click();
    h.run_steps(3);
    h.get_by_label("Combine into one").click();
    h.run_steps(3);
    save(&mut h, "publish-output-tab");
    h.get_by_label("1 PDF with 3 pages");
}

#[test]
fn the_window_fits_a_compact_screen_at_2x() {
    let mut h = harness(900.0, 600.0, 2.0);
    save(&mut h, "publish-compact-2x");
    h.get_by_label("Publish Sheets");
}
