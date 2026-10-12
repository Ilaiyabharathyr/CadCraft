//! Tablet controls around the same drawing engine and canvas used on the desktop.

use egui::{Button, Modifiers, Pos2, vec2};
use serde_json::json;

use crate::{
    CadApp,
    icons::{self, Icon},
    theme::Tokens,
};

/// The Android host opts in; an ordinary desktop session keeps its existing layout.
pub fn configure(app: &mut CadApp) {
    app.tablet_mode = true;
    app.ui.system_cursor = true;
    app.ui.show_toolbar = false;
    app.ui.show_toolsets = false;
    app.ui.show_palettes = false;
    app.ui.show_status_bar = false;
    app.ui.show_command_line = false;
    app.ui.history_lines = 1;
    app.session.settings.pickbox = app.session.settings.pickbox.max(12.0);
    app.session.settings.aperture = app.session.settings.aperture.max(18.0);
}

/// Delay canvas picks until a tap is known to be single. Cancelling must not first place a
/// point (which can finish a circle or add a polyline segment).
#[derive(Default)]
pub(crate) struct Taps {
    pending: Option<Tap>,
}

struct Tap {
    pos: Pos2,
    modifiers: Modifiers,
    time: f64,
    doc: u64,
    command: Option<String>,
}

pub(crate) enum TapAction {
    None,
    Pick(Pos2, Modifiers),
    Cancel,
}

impl Taps {
    pub(crate) fn update(
        &mut self,
        ctx: &egui::Context,
        doc: u64,
        command: Option<&str>,
        click: Option<(Pos2, Modifiers)>,
        double: bool,
        clear: bool,
    ) -> TapAction {
        if clear {
            self.pending = None;
            return TapAction::None;
        }
        if self.pending.as_ref().is_some_and(|p| p.doc != doc || p.command.as_deref() != command) {
            self.pending = None;
        }
        if double {
            self.pending = None;
            return TapAction::Cancel;
        }
        let time = ctx.input(|i| i.time);
        if !time.is_finite() {
            self.pending = None;
            return TapAction::None;
        }
        let delay = ctx.options(|o| o.input_options.max_double_click_delay);
        let delay = if delay.is_finite() { delay.clamp(0.05, 0.5) } else { 0.3 };
        let expired = self.pending.as_ref().is_some_and(|p| time - p.time >= delay);
        let previous = if click.is_some() || expired { self.pending.take() } else { None };
        if let Some((pos, modifiers)) = click.filter(|(pos, _)| pos.is_finite()) {
            self.pending = Some(Tap { pos, modifiers, time, doc, command: command.map(str::to_owned) });
        }
        if let Some(tap) = &self.pending {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64((delay - (time - tap.time)).clamp(0.0, 0.5)));
        }
        match previous {
            Some(tap) => TapAction::Pick(tap.pos, tap.modifiers),
            None => TapAction::None,
        }
    }
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
