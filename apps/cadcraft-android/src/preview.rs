//! Run the exact tablet shell on a desktop to inspect layouts and input before packaging.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

#[path = "../../cadcraft/src/control_server.rs"]
mod control_server;

fn main() -> eframe::Result {
    let dir = std::env::var_os("CADCRAFT_TABLET_DATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("cadcraft-tablet-preview"));
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title("CADCraft Tablet Preview").with_inner_size([1280.0, 800.0]),
        persist_window: false,
        ..Default::default()
    };
    eframe::run_native(
        "CADCraft Tablet Preview",
        options,
        Box::new(move |cc| {
            let mut app = cadcraft_android::shell::Shell::new(cc, dir)?;
            if let Some(port) = std::env::var("CADCRAFT_CONTROL_PORT").ok().and_then(|s| s.parse().ok()) {
                app.cad = app.cad.with_control(control_server::start(port, cc.egui_ctx.clone()));
            }
            Ok(Box::new(app))
        }),
    )
}
