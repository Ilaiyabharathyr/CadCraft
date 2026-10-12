//! Platform-independent tablet shell, also runnable on a desktop for layout inspection.
#![forbid(unsafe_code)]

use cadcraft_engine::Session;
use cadcraft_ui_egui::{CadApp, Services};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

pub struct Shell {
    pub cad: CadApp,
    dir: PathBuf,
    checkpoint_at: Instant,
    checkpoint: Option<std::thread::JoinHandle<Result<(), String>>>,
    checkpoint_key: Vec<(u64, u64, bool, String, Option<String>)>,
    checkpoint_active: usize,
    #[cfg(target_os = "android")]
    poll_at: Instant,
}

fn install_io() {
    cadcraft_engine::cmd::file::set_io(cadcraft_engine::cmd::file::IoHooks {
        read: |b, name| cadcraft_io::read(b, name).map_err(|e| e.to_string()),
        write: |d, name| cadcraft_io::write(d, name).map_err(|e| e.to_string()),
        plot: Some(|d, space, opts| cadcraft_io::plotter::plot(d, space, opts).map_err(|e| e.to_string())),
    });
    cadcraft_engine::cmd::file::set_write_as(|d, name, o| {
        cadcraft_io::write_with(d, name, &cadcraft_io::SaveOptions { version: o.version, binary: o.binary }).map_err(|e| e.to_string())
    });
}

/// A save-as creates a distinct file; QSAVE subsequently writes its remembered path.
fn save_path(dir: &Path, name: &str) -> Option<String> {
    let name: String = name.chars().take(180).map(|c| if c == '/' || c == '\\' || c.is_control() { '_' } else { c }).collect();
    let name = if name.is_empty() || name == "." || name == ".." { "Drawing.dxf" } else { &name };
    let stem = Path::new(name).file_stem()?.to_string_lossy();
    let ext = Path::new(name).extension().map(|e| e.to_string_lossy()).unwrap_or_else(|| "dxf".into());
    let folder = dir.join("drawings");
    if std::fs::create_dir_all(&folder).is_err() {
        return None;
    }
    for i in 0..10_000 {
        let path = folder.join(if i == 0 { format!("{stem}.{ext}") } else { format!("{stem}-{i}.{ext}") });
        if !path.exists() {
            return Some(path.to_string_lossy().into_owned());
        }
    }
    None
}

fn files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for folder in [dir.join("drawings"), dir.join("imports")] {
        if let Ok(entries) = std::fs::read_dir(folder) {
            for entry in entries.take(1000).filter_map(Result::ok) {
                let path = entry.path();
                if path.is_dir() {
                    if let Ok(children) = std::fs::read_dir(path) {
                        out.extend(children.take(16).filter_map(Result::ok).map(|entry| entry.path()));
                    }
                } else {
                    out.push(path);
                }
            }
        }
    }
    out.retain(|path| path.is_file() && path.extension().is_some_and(|e| e.eq_ignore_ascii_case("dxf") || e.eq_ignore_ascii_case("dwg")));
    out.sort();
    out
}

impl Shell {
    pub fn new(cc: &eframe::CreationContext<'_>, dir: PathBuf) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        std::fs::create_dir_all(&dir)?;
        install_io();
        let save_dir = dir.clone();
        let services = Services {
            pick_open: Some(Box::new(|| {
                #[cfg(target_os = "android")]
                if let Err(error) = crate::platform::pick_open() {
                    log::warn!("Open: {error}");
                }
                None
            })),
            pick_save: Some(Box::new(move |name| save_path(&save_dir, name))),
        };
        let mut cad = CadApp::new(Session::empty(), services);
        #[cfg(target_os = "android")]
        if let Ok(Some(locale)) = crate::platform::take("interfaceLocale") {
            cad.system_languages.push(locale);
        }
        if let Some(storage) = cc.storage {
            if let Some(prefs) = storage.get_string(cadcraft_ui_egui::PREFS_KEY) {
                cad.load_prefs(&prefs);
            }
            if let Some(profile) = storage.get_string(cadcraft_ui_egui::options::PROFILE_KEY) {
                cad.load_profile(&profile);
            }
        }
        cadcraft_ui_egui::tablet::configure(&mut cad);
        cc.egui_ctx.options_mut(|o| o.input_options.max_click_dist = 12.0);
        cadcraft_engine::autosave::set_savetime(1);
        cad.enable_autosave(&dir.join("autosave"));
        if let Some(rs) = &cc.wgpu_render_state {
            cad.set_wgpu(rs);
        }
        if let Err(error) = restore(&mut cad, &dir) {
            cad.set_status(format!("Recovery: {error}"));
        }
        if cad.session.docs.is_empty() {
            cad.session.new_drawing(false);
        }
        cad.enable_background_open();
        Ok(Self {
            cad,
            dir,
            checkpoint_at: Instant::now(),
            checkpoint: None,
            checkpoint_key: Vec::new(),
            checkpoint_active: 0,
            #[cfg(target_os = "android")]
            poll_at: Instant::now(),
        })
    }

    fn checkpoint(&mut self, force: bool) {
        if self.checkpoint.as_ref().is_some_and(|h| !h.is_finished()) {
            return;
        }
        if let Some(handle) = self.checkpoint.take() {
            match handle.join() {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    self.checkpoint_key.clear();
                    self.cad.set_status(format!("Recovery could not be saved: {error}"));
                }
                Err(_) => {
                    self.checkpoint_key.clear();
                    self.cad.set_status("Recovery could not be saved");
                }
            }
        }
        if !force && self.checkpoint_at.elapsed() < Duration::from_secs(5) {
            return;
        }
        self.checkpoint_at = Instant::now();
        let key: Vec<_> = self.cad.session.docs.iter().map(|d| (d.uid, d.revision, d.is_dirty(), d.title.clone(), d.path.clone())).collect();
        if key == self.checkpoint_key && self.cad.session.active == self.checkpoint_active {
            return;
        }
        let docs: Vec<_> = self
            .cad
            .session
            .docs
            .iter()
            .map(|d| {
                let view = d.view();
                (
                    d.doc.clone(),
                    json!({"title": d.title, "path": d.path, "dirty": d.is_dirty(),
                "center": [view.center.x, view.center.y], "height": view.height}),
                )
            })
            .collect();
        let dir = self.dir.clone();
        let active = self.cad.session.active;
        match std::thread::Builder::new().name("tablet-recovery".into()).spawn(move || checkpoint(&dir, docs, active)) {
            Ok(handle) => {
                self.checkpoint = Some(handle);
                self.checkpoint_key = key;
                self.checkpoint_active = active;
            }
            Err(error) => self.cad.set_status(format!("Recovery could not be saved: {error}")),
        }
    }

    #[cfg(target_os = "android")]
    fn poll_documents(&mut self, ctx: &egui::Context) {
        if std::mem::take(&mut self.cad.tablet_keyboard_requested)
            && let Err(error) = crate::platform::input(&self.cad.cmd.buffer)
        {
            self.cad.set_status(format!("Keyboard: {error}"));
        }
        if self.poll_at.elapsed() >= Duration::from_millis(250) {
            self.poll_at = Instant::now();
            match crate::platform::take("takeImportedPath") {
                Ok(Some(path)) => self.cad.open_path(&path),
                Ok(None) => {}
                Err(error) => log::warn!("Android file picker: {error}"),
            }
            if let Ok(Some(message)) = crate::platform::take("takeNotice") {
                self.cad.set_status(message);
            }
            if let Ok(Some(command)) = crate::platform::take("takeCommand") {
                self.cad.cmd.buffer.clear();
                let _ = self.cad.cmdline(&command);
            }
        }
        ctx.request_repaint_after(Duration::from_millis(250));
    }

    #[cfg(target_os = "android")]
    fn export(&mut self) {
        self.cad.start("qsave");
        let path = self.cad.session.state().ok().filter(|d| !d.is_dirty()).and_then(|d| d.path.clone());
        let Some(path) = path else {
            self.cad.set_status("Save the drawing before exporting");
            return;
        };
        let name = Path::new(&path).file_name().map(|n| n.to_string_lossy()).unwrap_or_else(|| "Drawing.dxf".into());
        if let Err(error) = crate::platform::export(&path, &name) {
            self.cad.set_status(format!("Export: {error}"));
        }
    }
}

impl eframe::App for Shell {
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        self.cad.raw_input_hook(raw);
        cadcraft_ui_egui::cmdline::capture_tab(ctx, raw);
    }
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(target_os = "android")]
        self.poll_documents(ctx);
        self.cad.logic(ctx);
        self.checkpoint(!ctx.input(|i| i.focused));
        ctx.request_repaint_after(Duration::from_secs(5));
        if std::mem::take(&mut self.cad.quit_requested) {
            #[cfg(target_os = "android")]
            if let Err(error) = crate::platform::background() {
                self.cad.set_status(format!("Close: {error}"));
            }
            #[cfg(not(target_os = "android"))]
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("tablet-files").frame(egui::Frame::NONE.fill(cadcraft_ui_egui::theme::Tokens::get().chrome).inner_margin(6.0)).show(
            ui,
            |ui| {
                ui.horizontal(|ui| {
                    ui.menu_button("Drawings", |ui| {
                        for path in files(&self.dir) {
                            let label = path.file_name().map(|s| s.to_string_lossy()).unwrap_or_default();
                            if ui.add_sized(egui::vec2(240.0, 44.0), egui::Button::new(label)).clicked() {
                                self.cad.open_path(&path.to_string_lossy());
                                ui.close();
                            }
                        }
                        if ui.add_sized(egui::vec2(240.0, 44.0), egui::Button::new("Sample drawing")).clicked() {
                            let _ = self.cad.run("ui.sample", json!({}));
                            ui.close();
                        }
                    });
                    #[cfg(target_os = "android")]
                    if ui.add_sized(egui::vec2(96.0, 44.0), egui::Button::new("Export file")).clicked() {
                        self.export();
                    }
                    if let Ok(doc) = self.cad.session.state() {
                        ui.label(format!("{}{}", doc.title, if doc.is_dirty() { " *" } else { "" }));
                    }
                });
            },
        );
        self.cad.ui(ui);
    }
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string(cadcraft_ui_egui::PREFS_KEY, self.cad.prefs_json());
        storage.set_string(cadcraft_ui_egui::options::PROFILE_KEY, self.cad.profile_json());
    }
    fn auto_save_interval(&self) -> Duration {
        Duration::from_secs(5)
    }
    fn persist_egui_memory(&self) -> bool {
        false
    }
    fn on_exit(&mut self) {
        if let Some(handle) = self.checkpoint.take() {
            let _ = handle.join();
        }
        self.checkpoint_active = usize::MAX;
        self.checkpoint(true);
        if let Some(handle) = self.checkpoint.take() {
            let _ = handle.join();
        }
    }
}

fn checkpoint(dir: &Path, docs: Vec<(Arc<cadcraft_engine::doc::Drawing>, Value)>, active: usize) -> Result<(), String> {
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let recovery = dir.join("tablet-recovery");
        std::fs::create_dir_all(&recovery)?;
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos();
        let generation = format!("snapshot-{stamp}");
        let folder = recovery.join(&generation);
        std::fs::create_dir(&folder)?;
        let mut records = Vec::new();
        for (i, (doc, mut record)) in docs.into_iter().enumerate() {
            let name = format!("drawing-{i}.dxf");
            std::fs::write(folder.join(&name), cadcraft_io::write(&doc, &name)?)?;
            record["file"] = json!(format!("{generation}/{name}"));
            records.push(record);
        }
        let tmp = recovery.join("session.tmp");
        std::fs::write(&tmp, serde_json::to_vec(&json!({"active": active, "drawings": records}))?)?;
        std::fs::rename(&tmp, recovery.join("session.json"))?;
        for entry in std::fs::read_dir(&recovery)?.take(1000).filter_map(Result::ok) {
            if entry.file_name().to_string_lossy().starts_with("snapshot-") && entry.file_name() != generation.as_str() {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
        Ok(())
    })();
    result.map_err(|e| e.to_string())
}

fn restore(cad: &mut CadApp, dir: &Path) -> Result<(), String> {
    let recovery = dir.join("tablet-recovery");
    let manifest = recovery.join("session.json");
    if !manifest.exists() {
        return Ok(());
    }
    if std::fs::metadata(&manifest).map_err(|e| e.to_string())?.len() > 1 << 20 {
        return Err("Recovery index is too large".into());
    }
    let data: Value = serde_json::from_slice(&std::fs::read(manifest).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let records = data.get("drawings").and_then(Value::as_array).ok_or("Invalid recovery index")?;
    if records.len() > 128 {
        return Err("Recovery contains too many drawings; saved files remain in Drawings".into());
    }
    for record in records.iter().take(128) {
        let Some(file) = record.get("file").and_then(Value::as_str) else { continue };
        if !Path::new(file).components().all(|c| matches!(c, std::path::Component::Normal(_))) {
            continue;
        }
        let path = recovery.join(file);
        if std::fs::metadata(&path).map_err(|e| e.to_string())?.len() > CadApp::MAX_OPEN_BYTES {
            continue;
        }
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        let doc = cadcraft_io::read(&bytes, "drawing.dxf").map_err(|e| e.to_string())?;
        let title = record.get("title").and_then(Value::as_str).unwrap_or("Drawing");
        let original = record.get("path").and_then(Value::as_str).map(str::to_owned);
        cad.session.open_drawing(doc, title, original);
        if let Ok(st) = cad.session.state_mut() {
            if record.get("dirty").and_then(Value::as_bool) == Some(true) {
                st.saved = Arc::new((*st.doc).clone());
            }
            if let (Some(x), Some(y), Some(height)) = (
                record.pointer("/center/0").and_then(Value::as_f64),
                record.pointer("/center/1").and_then(Value::as_f64),
                record.get("height").and_then(Value::as_f64),
            ) && x.is_finite()
                && y.is_finite()
                && height.is_finite()
                && height > 0.0
            {
                st.set_view_quiet(cadcraft_engine::View { center: cadcraft_engine::geom::Vec2::new(x, y), height });
            }
        }
    }
    let active = data.get("active").and_then(Value::as_u64).and_then(|i| usize::try_from(i).ok()).unwrap_or(0);
    if active < cad.session.docs.len() {
        cad.session.active = active;
    }
    Ok(())
}
