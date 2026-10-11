//! Tablet controls around the same drawing engine and canvas used on the desktop.

use egui::{Button, vec2};
use serde_json::json;

use crate::{
    CadApp,
    icons::{self, Icon},
    theme::Tokens,
};

/// The Android host opts in; an ordinary desktop session keeps its existing layout.
pub fn configure(app: &mut CadApp) {
    app.tablet_mode = true;
    app.ui.show_toolbar = false;
    app.ui.show_toolsets = false;
    app.ui.show_palettes = false;
    app.ui.show_status_bar = false;
    app.ui.show_command_line = false;
    app.ui.history_lines = 1;
    app.session.settings.pickbox = app.session.settings.pickbox.max(12.0);
    app.session.settings.aperture = app.session.settings.aperture.max(18.0);
}

fn button(ui: &mut egui::Ui, text: &str) -> bool {
    ui.add_sized(vec2(64.0, 44.0), Button::new(crate::i18n::t(text))).clicked()
}

pub fn chrome(app: &mut CadApp, ui: &mut egui::Ui) {
    let t = Tokens::get();
    egui::Panel::top("tablet-tools").frame(egui::Frame::NONE.fill(t.chrome).inner_margin(6.0)).show(ui, |ui| {
        egui::ScrollArea::horizontal().id_salt("tablet-tool-scroll").show(ui, |ui| {
            ui.horizontal(|ui| {
                for (icon, command, label) in [
                    (Icon::New, "new", "New Drawing"),
                    (Icon::Open, "ui.open", "Open"),
                    (Icon::Save, "qsave", "Save"),
                    (Icon::Undo, "u", "Undo"),
                    (Icon::Redo, "redo", "Redo"),
                    (Icon::ZoomExtents, "zoom.extents", "Zoom Extents"),
                ] {
                    if icons::button(ui, icon, 44.0, label, false).clicked() {
                        app.start(command);
                    }
                }
                ui.separator();
                for (label, command) in [
                    ("Line", "line"),
                    ("Polyline", "pline"),
                    ("Circle", "circle"),
                    ("Rectangle", "rectang"),
                    ("Move", "move"),
                    ("Offset", "offset"),
                    ("Trim", "trim"),
                    ("Measure", "dist"),
                ] {
                    if button(ui, label) {
                        app.start(command);
                    }
                }
                ui.menu_button(crate::i18n::t("More"), |ui| {
                    for (label, command) in [
                        ("Tool Sets", "ui.toggle.toolsets"),
                        ("Properties", "ui.toggle.palettes"),
                        ("Layers", "ui.dialog.layers"),
                        ("Drafting Settings", "ui.dialog.dsettings"),
                        ("Options", "options"),
                        ("Save As", "ui.saveas"),
                        ("Help", "ui.dialog.about"),
                    ] {
                        if button(ui, label) {
                            app.start(command);
                            ui.close();
                        }
                    }
                });
            });
        });
    });
    egui::Panel::bottom("tablet-command").frame(egui::Frame::NONE.fill(t.chrome).inner_margin(6.0)).show(ui, |ui| {
        let prompt = app.session.current_prompt();
        ui.horizontal_wrapped(|ui| {
            if let Some(prompt) = &prompt {
                ui.label(crate::i18n::t(&prompt.message));
                for keyword in &prompt.keywords {
                    if button(ui, keyword)
                        && let Err(error) = app.session.input(cadcraft_engine::Input::Keyword(keyword.clone()))
                    {
                        app.set_status(error.to_string());
                    }
                }
            } else {
                ui.label("CADCraft");
                for (name, value) in
                    [("Grid", app.session.settings.gridmode), ("Snap", app.session.settings.snapmode), ("Ortho", app.session.settings.orthomode)]
                {
                    if ui.add_sized(vec2(64.0, 44.0), Button::new(crate::i18n::t(name)).selected(value)).clicked() {
                        let var = match name {
                            "Grid" => "GRIDMODE",
                            "Snap" => "SNAPMODE",
                            _ => "ORTHOMODE",
                        };
                        let _ = app.run("setvar", json!({"name": var, "value": i32::from(!value)}));
                    }
                }
            }
            if let Some((status, _)) = &app.status {
                ui.label(status);
            }
        });
        ui.horizontal(|ui| {
            let width = (ui.available_width() - if cfg!(target_os = "android") { 270.0 } else { 160.0 }).max(80.0);
            let response = ui.add_sized(
                vec2(width, 44.0),
                egui::TextEdit::singleline(&mut app.cmd.buffer).hint_text(crate::i18n::t("Command")).font(crate::theme::mono()),
            );
            let submit = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if cfg!(target_os = "android") && ui.add_sized(vec2(100.0, 44.0), Button::new(crate::i18n::t("Keyboard"))).clicked() {
                app.tablet_keyboard_requested = true;
                response.surrender_focus();
            }
            if button(ui, "Enter") || submit {
                let text = std::mem::take(&mut app.cmd.buffer);
                let _ = app.cmdline(&text);
                response.surrender_focus();
            }
            if button(ui, "Cancel") {
                app.session.cancel();
                app.cmd.buffer.clear();
                response.surrender_focus();
            }
        });
    });
}
