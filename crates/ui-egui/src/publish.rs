//! Publish Sheets: choose sheets from the open drawings, inspect them, choose where the PDFs go,
//! and publish them. A typed or menu-invoked PUBLISH opens this window; JSON calls (scripts, the
//! control channel, MCP) never get here and run the engine's `publish` directly.
//!
//! Publishing is one call to the engine's `publish` with the ticked sheets, so the window and
//! scripts produce the same files and names. The Status column and the file names come from
//! `publish {"action": "status"}`, refreshed a moment after the list or the options change.
//! Add Drawings... lists a drawing file's sheets without opening it, and a list can be saved to and
//! opened from a `.ccpub` file (the engine's `publishlist`). Options the engine can't honour yet
//! are shown disabled with "Not available yet".

use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};

use egui::{Align, Align2, Color32, FontId, Layout, Painter, Rect, RichText, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use serde_json::{Value, json};

use crate::CadApp;
use crate::theme::Tokens;

const ROW: f32 = 22.0;
const INSPECTOR: f32 = 340.0;
const CHECK: f32 = 13.0;
const INDENT: f32 = 29.0;
const PAPER_COL: f32 = 96.0;
const STATUS_COL: f32 = 110.0;
const GAP: f32 = 8.0;
/// Seconds after the last change before the Status column is checked again.
const STATUS_DELAY: f64 = 0.25;
/// Papers offered for a one-run page setup change.
const PAPERS: &[&str] = &["A4", "A3", "A2", "A1", "A0", "Letter", "Legal", "Tabloid", "ANSI C", "ANSI D", "ANSI E"];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Tab {
    #[default]
    Sheet,
    Output,
}

/// What the files are.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum To {
    #[default]
    Pdf,
    Svg,
    Png,
}

impl To {
    fn id(self) -> &'static str {
        match self {
            To::Pdf => "pdf",
            To::Svg => "svg",
            To::Png => "png",
        }
    }
}

/// A sheet's page setup changed for one run (the drawing keeps its own).
#[derive(Clone, Debug, PartialEq)]
struct SetupChange {
    /// A short paper name (`A3`, `Letter`…).
    paper: String,
    landscape: bool,
    fit: bool,
}

/// What `publish {"action": "status"}` said, for the request it was asked about.
#[derive(Clone, Debug, Default)]
struct Checked {
    /// The request's fingerprint (see [`fingerprint`]).
    key: u64,
    /// Every listed sheet: (status, reason).
    sheets: BTreeMap<(u64, String), (String, String)>,
    /// The ticked sheets' file names, in order (one name when combining).
    files: Vec<String>,
}

/// The window's state between frames (egui temp data, not saved with the workspace).
#[derive(Clone, Debug, Default)]
pub struct PublishState {
    /// Sheets left out, by (drawing uid, sheet name).
    skipped: BTreeSet<(u64, String)>,
    /// Drawings taken out of the list (they stay open).
    removed: BTreeSet<u64>,
    /// Sheet order per drawing when the user moved sheets.
    order: BTreeMap<u64, Vec<String>>,
    inspected: Option<(u64, String)>,
    tab: Tab,
    only_current: bool,
    combine: bool,
    drop_layers: bool,
    ask_name: bool,
    folder: Option<String>,
    report: Option<String>,
    to: To,
    /// Page setups changed for this run, by (drawing uid, sheet name).
    changes: BTreeMap<(u64, String), SetupChange>,
    checked: Option<Checked>,
    /// A change waiting to be checked: (fingerprint, when it was seen).
    pending: Option<(u64, f64)>,
    /// Drawings listed from files (Add Drawings..., or a list), in the order they were added.
    files: Vec<FileDrawing>,
    /// The list file this list was opened from or saved to.
    list: Option<String>,
    /// A file name pattern from a list (the window has no field for it yet).
    name: Option<String>,
    /// Publish now, in the window, instead of on a worker thread.
    foreground: bool,
    /// Open the PDF (or the folder) when the run ends.
    open_result: bool,
    /// The sheet stamp: whether it's added, and its settings.
    stamp_on: bool,
    stamp: StampSettings,
    /// The Sheet Stamp window is open.
    editing_stamp: bool,
}

/// Text in a corner of every sheet, with `{drawing}`, `{sheet}`, `{n}`, `{date}` and `{time}`.
#[derive(Clone, Debug, PartialEq)]
struct StampSettings {
    text: String,
    corner: usize,
    height: f64,
}

/// The stamp's corners as the engine names them, and as the window shows them.
const CORNERS: [(&str, &str); 4] =
    [("bottom-right", "Bottom right"), ("bottom-left", "Bottom left"), ("top-right", "Top right"), ("top-left", "Top left")];

impl Default for StampSettings {
    fn default() -> Self {
        StampSettings { text: "{drawing} · {sheet} · {date} {time}".into(), corner: 0, height: 2.5 }
    }
}

impl StampSettings {
    fn json(&self) -> Value {
        json!({ "text": self.text, "corner": CORNERS.get(self.corner).map_or("bottom-right", |c| c.0), "height": self.height })
    }
}

/// A drawing listed from its file: read for publishing, never opened as a tab.
#[derive(Clone, Debug)]
struct FileDrawing {
    path: String,
    uid: u64,
    title: String,
    sheets: Vec<Sheet>,
    /// False when the file is gone; its sheets are then the ones the list named.
    found: bool,
}

/// Where a group's drawing comes from.
#[derive(Clone, Debug, PartialEq)]
enum Source {
    /// An open drawing, by its index in the session.
    Open(usize),
    /// A drawing file, by its path.
    File(String),
}

/// A listed file's id: its path's hash with the top bit set, so it never meets an open
/// drawing's uid (those count up from 1).
fn file_uid(path: &str) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut h);
    h.finish() | (1 << 63)
}

/// One sheet as the list shows it.
#[derive(Clone, Debug)]
struct Sheet {
    name: String,
    model: bool,
    setup: String,
    device: String,
    size: String,
    scale: String,
    landscape: bool,
    default_setup: bool,
}

/// One open drawing and its sheets, in publishing order.
#[derive(Clone, Debug)]
struct Group {
    source: Source,
    uid: u64,
    /// False for a listed file that is gone.
    found: bool,
    /// The drawing's revision: the Status column is checked again when it changes.
    revision: u64,
    title: String,
    stem: String,
    folder: String,
    current: bool,
    sheets: Vec<Sheet>,
}

#[derive(Default)]
struct Actions {
    /// In: a run is in progress; the host can open files.
    busy: bool,
    can_open: bool,
    publish: bool,
    add: bool,
    open_list: bool,
    save_list: bool,
    choose: bool,
    help: bool,
    close: bool,
}

fn ratio(k: f64) -> String {
    if !k.is_finite() || k <= 0.0 {
        return "—".into();
    }
    let num = |v: f64| if (v - v.round()).abs() < 1e-6 { format!("{}", v.round()) } else { format!("{v:.2}") };
    if k < 1.0 { format!("1:{}", num(1.0 / k)) } else { format!("{}:1", num(k)) }
}

fn model_sheet() -> Sheet {
    Sheet {
        name: "Model".into(),
        model: true,
        setup: "Default".into(),
        device: "PDF".into(),
        size: String::new(),
        scale: "Fit".into(),
        landscape: true,
        default_setup: true,
    }
}

fn layout_sheet(name: &str, p: &cadcraft_doc::PageSetup) -> Sheet {
    let default_setup = *p == cadcraft_doc::PageSetup::default();
    let (w, h) = if p.landscape {
        (p.height_mm.max(p.width_mm), p.height_mm.min(p.width_mm))
    } else {
        (p.width_mm.min(p.height_mm), p.width_mm.max(p.height_mm))
    };
    let device = if p.device.is_empty() || p.device.eq_ignore_ascii_case("none") { "PDF".to_string() } else { p.device.clone() };
    Sheet {
        name: name.to_string(),
        model: false,
        setup: if default_setup {
            "Default".into()
        } else if device == "PDF" {
            p.paper.clone()
        } else {
            device.clone()
        },
        device,
        size: if w.is_finite() && h.is_finite() && w > 0.0 { format!("{w:.0} × {h:.0} mm") } else { String::new() },
        scale: if p.scale_to_fit { "Fit".into() } else { ratio(p.scale) },
        landscape: p.landscape,
        default_setup,
    }
}

/// A sheet a list names in a drawing that is gone: only its name is known.
fn named_sheet(name: &str) -> Sheet {
    if name.eq_ignore_ascii_case("model") {
        return model_sheet();
    }
    Sheet {
        name: name.to_string(),
        model: false,
        setup: "—".into(),
        device: String::new(),
        size: String::new(),
        scale: String::new(),
        landscape: true,
        default_setup: false,
    }
}

fn sheets_of(d: &cadcraft_doc::Drawing) -> Vec<Sheet> {
    let mut out = Vec::new();
    if !d.model.is_empty() {
        out.push(model_sheet());
    }
    let mut layouts: Vec<_> = d.layouts.iter().collect();
    layouts.sort_by_key(|l| l.tab_order);
    out.extend(layouts.into_iter().map(|l| layout_sheet(&l.name, &l.page)));
    out
}

/// A drawing file's sheets as the engine's `publishlist {"action": "drawing"}` lists them.
fn file_drawing(path: &str, r: &Value) -> FileDrawing {
    let sheets = r["sheets"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|x| {
            let name = x["layout"].as_str()?;
            if x["model"] == true {
                return Some(model_sheet());
            }
            let page: cadcraft_doc::PageSetup = serde_json::from_value(x["page"].clone()).unwrap_or_default();
            Some(layout_sheet(name, &page))
        })
        .collect();
    let title = r["title"].as_str().map_or_else(|| path.to_string(), str::to_string);
    FileDrawing { path: path.to_string(), uid: file_uid(path), title, sheets, found: true }
}

fn stem_of(name: &str) -> String {
    std::path::Path::new(name).file_stem().map(|s| s.to_string_lossy().to_string()).filter(|s| !s.is_empty()).unwrap_or_else(|| "Drawing".into())
}

fn gather(app: &CadApp, st: &PublishState) -> Vec<Group> {
    let active = app.session.active;
    let ordered = |uid: u64, mut sheets: Vec<Sheet>| {
        if let Some(order) = st.order.get(&uid) {
            sheets.sort_by_key(|s| order.iter().position(|n| *n == s.name).unwrap_or(usize::MAX));
        }
        sheets
    };
    let mut groups = Vec::new();
    for (index, ds) in app.session.docs.iter().enumerate() {
        if st.removed.contains(&ds.uid) || (st.only_current && index != active) {
            continue;
        }
        let folder = ds.path.as_deref().and_then(|p| std::path::Path::new(p).parent()).map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
        groups.push(Group {
            source: Source::Open(index),
            uid: ds.uid,
            found: true,
            revision: ds.revision,
            title: ds.title.clone(),
            stem: stem_of(&ds.title),
            folder,
            current: index == active,
            sheets: ordered(ds.uid, sheets_of(&ds.doc)),
        });
    }
    for f in &st.files {
        let folder = std::path::Path::new(&f.path).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
        groups.push(Group {
            source: Source::File(f.path.clone()),
            uid: f.uid,
            found: f.found,
            revision: 0,
            title: f.title.clone(),
            stem: stem_of(&f.title),
            folder,
            current: false,
            sheets: ordered(f.uid, f.sheets.clone()),
        });
    }
    groups
}

/// Add a drawing file's sheets to the list without opening it. A drawing that is already open
/// comes back into the list as the open drawing.
fn add_drawing(app: &mut CadApp, st: &mut PublishState, path: &str) -> Result<(), String> {
    if let Some(ds) = app.session.docs.iter().find(|d| d.path.as_deref() == Some(path)) {
        st.removed.remove(&ds.uid);
        return Ok(());
    }
    if st.files.iter().any(|f| f.path == path && f.found) {
        return Ok(());
    }
    let r = app.run("publishlist", json!({ "action": "drawing", "path": path }))?;
    st.files.retain(|f| f.path != path);
    st.files.push(file_drawing(path, &r));
    Ok(())
}

/// Open a list: its drawings (open ones as they are, others from their files), its ticks, order
/// and page setup changes, and its output settings.
fn open_list(app: &mut CadApp, st: &mut PublishState, path: &str) -> Result<(), String> {
    let r = app.run("publishlist", json!({ "action": "read", "path": path }))?;
    let entries: Vec<&Value> = r["sheets"].as_array().into_iter().flatten().collect();
    let mut next = PublishState { tab: st.tab, list: Some(path.to_string()), ..PublishState::default() };
    // The drawings in the list's order, with the sheets it names.
    let mut drawings: Vec<(String, bool, Vec<&Value>)> = Vec::new();
    for e in &entries {
        let d = e["drawing"].as_str().unwrap_or_default().to_string();
        match drawings.iter_mut().find(|(p, _, _)| *p == d) {
            Some((_, _, v)) => v.push(e),
            None => drawings.push((d, e["found"] == true, vec![e])),
        }
    }
    let open: Vec<(u64, Option<String>)> = app.session.docs.iter().map(|d| (d.uid, d.path.clone())).collect();
    for (uid, p) in &open {
        if !p.as_ref().is_some_and(|p| drawings.iter().any(|(d, _, _)| d == p)) {
            next.removed.insert(*uid);
        }
    }
    for (d, found, named) in &drawings {
        let uid = match open.iter().find(|(_, p)| p.as_deref() == Some(d.as_str())) {
            Some((uid, _)) => *uid,
            None => {
                let file =
                    if *found { app.run("publishlist", json!({ "action": "drawing", "path": d })).ok().map(|r| file_drawing(d, &r)) } else { None };
                let file = file.unwrap_or_else(|| FileDrawing {
                    path: d.clone(),
                    uid: file_uid(d),
                    title: std::path::Path::new(d).file_name().map_or_else(|| d.clone(), |n| n.to_string_lossy().to_string()),
                    sheets: named.iter().filter_map(|e| e["layout"].as_str()).map(named_sheet).collect(),
                    found: false,
                });
                let uid = file.uid;
                next.files.push(file);
                uid
            }
        };
        next.order.insert(uid, named.iter().filter_map(|e| e["layout"].as_str().map(str::to_string)).collect());
        for e in named {
            let layout = e["layout"].as_str().unwrap_or_default().to_string();
            if let Some(paper) = e["pageSetup"]["paper"].as_str() {
                let landscape = e["pageSetup"]["landscape"].as_bool().unwrap_or(true);
                let fit = e["pageSetup"]["fit"].as_bool().unwrap_or(false);
                next.changes.insert((uid, layout), SetupChange { paper: paper.to_string(), landscape, fit });
            }
        }
    }
    // A sheet is ticked when the list ticks it; sheets the list doesn't name are left out.
    for g in gather(app, &next) {
        let named = drawings.iter().find(|(d, _, _)| match &g.source {
            Source::Open(i) => app.session.docs.get(*i).and_then(|ds| ds.path.as_deref()) == Some(d.as_str()),
            Source::File(p) => p == d,
        });
        for s in &g.sheets {
            let ticked = named.is_some_and(|(_, _, es)| es.iter().any(|e| e["layout"].as_str() == Some(s.name.as_str()) && e["include"] != false));
            if !ticked {
                next.skipped.insert((g.uid, s.name.clone()));
            }
        }
    }
    let out = &r["output"];
    next.to = match out["to"].as_str() {
        Some("svg") => To::Svg,
        Some("png") => To::Png,
        _ => To::Pdf,
    };
    next.combine = out["combine"] == true;
    next.drop_layers = out["layers"] == false;
    next.folder = out["folder"].as_str().map(str::to_string);
    next.name = out["name"].as_str().map(str::to_string);
    if let Some(stamp) = out["stamp"].as_object() {
        next.stamp_on = true;
        let d = StampSettings::default();
        next.stamp = StampSettings {
            text: stamp.get("text").and_then(Value::as_str).map_or(d.text, str::to_string),
            corner: stamp.get("corner").and_then(Value::as_str).and_then(|c| CORNERS.iter().position(|k| k.0 == c)).unwrap_or(0),
            height: stamp.get("height").and_then(Value::as_f64).unwrap_or(d.height),
        };
    } else if out["stamp"] == true {
        next.stamp_on = true;
    }
    next.foreground = st.foreground;
    next.open_result = st.open_result;
    *st = next;
    Ok(())
}

/// Save the list as it is shown: every sheet with its tick, order and page setup change, and the
/// output settings. Returns the report line. Drawings that were never saved have no file to
/// point to and are left out.
fn save_list(app: &mut CadApp, groups: &[Group], st: &mut PublishState, path: &str) -> String {
    let mut sheets = Vec::new();
    let mut unsaved = Vec::new();
    for g in groups {
        let drawing = match &g.source {
            Source::Open(i) => app.session.docs.get(*i).and_then(|d| d.path.clone()),
            Source::File(p) => Some(p.clone()),
        };
        let Some(drawing) = drawing else {
            unsaved.push(g.title.clone());
            continue;
        };
        for s in &g.sheets {
            let mut e = json!({ "drawing": drawing, "layout": s.name, "include": !st.skipped.contains(&(g.uid, s.name.clone())) });
            if let Some(c) = st.changes.get(&(g.uid, s.name.clone())) {
                e["pageSetup"] = json!({ "paper": c.paper, "landscape": c.landscape, "fit": c.fit });
            }
            sheets.push(e);
        }
    }
    let mut output = json!({ "to": st.to.id(), "combine": st.combine, "layers": !st.drop_layers });
    if let Some(folder) = st.folder.as_ref().filter(|f| !f.trim().is_empty()) {
        output["folder"] = json!(folder);
    }
    if let Some(name) = &st.name {
        output["name"] = json!(name);
    }
    if st.stamp_on {
        output["stamp"] = st.stamp.json();
    }
    match app.run("publishlist", json!({ "action": "write", "path": path, "sheets": sheets, "output": output })) {
        Ok(_) => {
            st.list = Some(path.to_string());
            let saved = crate::tf!("Saved the list to {path}", path = path);
            if unsaved.is_empty() {
                saved
            } else {
                format!("{saved} · {}", crate::tf!("Not saved yet, so left out: {names}", names = unsaved.join(", ")))
            }
        }
        Err(e) => e,
    }
}

/// The (group, sheet) pairs that will be published, in order.
fn picked<'a>(groups: &'a [Group], st: &PublishState) -> Vec<(&'a Group, &'a Sheet)> {
    groups.iter().flat_map(|g| g.sheets.iter().filter(|s| !st.skipped.contains(&(g.uid, s.name.clone()))).map(move |s| (g, s))).collect()
}

fn default_folder(groups: &[Group], st: &PublishState) -> String {
    st.folder.clone().filter(|f| !f.trim().is_empty()).or_else(|| groups.iter().map(|g| g.folder.clone()).find(|f| !f.is_empty())).unwrap_or_default()
}

/// A `sheets` entry for the engine: the drawing by its index, the sheet, and a page setup
/// changed for this run.
fn sheet_json(g: &Group, s: &Sheet, st: &PublishState) -> Value {
    let mut v = match &g.source {
        Source::Open(i) => json!({ "drawing": i, "layout": s.name }),
        Source::File(path) => json!({ "drawing": path, "layout": s.name }),
    };
    if let Some(c) = st.changes.get(&(g.uid, s.name.clone())) {
        v["pageSetup"] = json!({ "paper": c.paper, "landscape": c.landscape, "fit": c.fit });
    }
    v
}

/// Combining is for PDFs.
fn combining(st: &PublishState) -> bool {
    st.combine && st.to == To::Pdf
}

/// The publish request for the ticked sheets (without a folder: the caller adds it).
fn publish_request(groups: &[Group], st: &PublishState) -> Value {
    let sheets: Vec<Value> = picked(groups, st).into_iter().map(|(g, s)| sheet_json(g, s, st)).collect();
    let mut r = json!({ "sheets": sheets, "to": st.to.id(), "combine": combining(st) });
    if let Some(name) = &st.name {
        r["name"] = json!(name);
    }
    if st.stamp_on {
        r["stamp"] = st.stamp.json();
    }
    if st.to == To::Pdf {
        r["pdf"] = json!({ "layers": !st.drop_layers });
    }
    r
}

/// The two status requests: every listed sheet (for the Status column), and the ticked ones as
/// they will be published (for the file names).
fn status_requests(groups: &[Group], st: &PublishState) -> (Value, Value) {
    let all: Vec<Value> = groups.iter().flat_map(|g| g.sheets.iter().map(move |s| (g, s))).map(|(g, s)| sheet_json(g, s, st)).collect();
    let mut names = publish_request(groups, st);
    names["action"] = json!("status");
    (json!({ "action": "status", "sheets": all }), names)
}

/// A fingerprint of what the status depends on: the requests and the drawings' revisions.
fn fingerprint(groups: &[Group], requests: &(Value, Value)) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    requests.0.to_string().hash(&mut h);
    requests.1.to_string().hash(&mut h);
    for g in groups {
        (g.uid, g.revision).hash(&mut h);
    }
    h.finish()
}

/// Ask the engine about the listed sheets: a moment after the last change, so typing a folder or
/// ticking several sheets checks once. The first check runs at once.
fn refresh_status(app: &mut CadApp, ctx: &egui::Context, groups: &[Group], st: &mut PublishState) {
    let requests = status_requests(groups, st);
    let key = fingerprint(groups, &requests);
    if st.checked.as_ref().is_some_and(|c| c.key == key) {
        st.pending = None;
        return;
    }
    let now = ctx.input(|i| i.time);
    let since = match st.pending {
        Some((k, t)) if k == key => now - t,
        _ => {
            st.pending = Some((key, now));
            0.0
        }
    };
    if st.checked.is_some() && since < STATUS_DELAY {
        ctx.request_repaint_after(std::time::Duration::from_secs_f64((STATUS_DELAY - since).max(0.01)));
        return;
    }
    let mut checked = Checked { key, ..Checked::default() };
    if let Ok(r) = app.run("publish", requests.0) {
        let listed = groups.iter().flat_map(|g| g.sheets.iter().map(move |s| (g.uid, s.name.clone())));
        for (key, x) in listed.zip(r["sheets"].as_array().into_iter().flatten()) {
            let status = x["status"].as_str().unwrap_or("error").to_string();
            checked.sheets.insert(key, (status, x["reason"].as_str().unwrap_or_default().to_string()));
        }
    }
    if !picked(groups, st).is_empty()
        && let Ok(r) = app.run("publish", requests.1)
    {
        checked.files = match r["combined"]["file"].as_str() {
            Some(f) => vec![f.to_string()],
            None => r["sheets"].as_array().into_iter().flatten().filter_map(|x| x["file"].as_str().map(str::to_string)).collect(),
        };
    }
    st.checked = Some(checked);
    st.pending = None;
}

/// How many ticked sheets the last check says will fail.
fn will_fail(groups: &[Group], st: &PublishState) -> usize {
    let Some(c) = &st.checked else { return 0 };
    picked(groups, st).iter().filter(|(g, s)| c.sheets.get(&(g.uid, s.name.clone())).is_some_and(|(k, _)| k == "error")).count()
}

/// Show the window (dialog name `"publish"`).
pub fn dialog(app: &mut CadApp, ctx: &egui::Context, open: &mut bool) {
    let id = egui::Id::new("cc_publish_state");
    let mut st: PublishState = ctx.data(|d| d.get_temp(id)).unwrap_or_default();
    let groups = gather(app, &st);
    let valid = |key: &(u64, String)| groups.iter().any(|g| g.uid == key.0 && g.sheets.iter().any(|s| s.name == key.1));
    if !st.inspected.as_ref().is_some_and(valid) {
        st.inspected = picked(&groups, &st)
            .first()
            .copied()
            .or_else(|| groups.first().and_then(|g| g.sheets.first().map(|s| (g, s))))
            .map(|(g, s)| (g.uid, s.name.clone()));
    }
    refresh_status(app, ctx, &groups, &mut st);
    let mut act = Actions { busy: app.publishing.busy(), can_open: app.services.open_path.is_some(), ..Actions::default() };
    let screen = ctx.content_rect();
    let center = screen.center();
    // Fit the screen: the window's frame and title bar take about 60 points beyond its content.
    let room = (screen.size() - vec2(40.0, 90.0)).max(vec2(480.0, 360.0));
    let size = vec2(1080.0, 640.0).min(room);
    egui::Window::new(crate::tl!("Publish Sheets"))
        .id(egui::Id::new("cc_publish"))
        .open(open)
        .collapsible(false)
        .resizable(true)
        .default_size(size)
        .min_size(vec2(860.0, 500.0).min(size))
        .max_size(room)
        .pivot(Align2::CENTER_CENTER)
        .default_pos(center)
        .show(ctx, |ui| body(ui, &groups, &mut st, &mut act));
    if act.close {
        *open = false;
    }
    if act.help {
        app.ui.dialog = Some("commands".into());
    }
    if act.add
        && let Some(path) = app.services.pick_open.as_ref().and_then(|pick| pick())
        && let Err(e) = add_drawing(app, &mut st, &path)
    {
        st.report = Some(e);
    }
    if act.open_list
        && let Some(path) = app.services.pick_list.as_ref().or(app.services.pick_open.as_ref()).and_then(|pick| pick())
    {
        st.report = Some(match open_list(app, &mut st, &path) {
            Ok(()) => crate::tf!("Opened the list {path}", path = path),
            Err(e) => e,
        });
    }
    if act.save_list {
        let suggested = st
            .list
            .as_deref()
            .and_then(|l| std::path::Path::new(l).file_name())
            .map_or_else(|| format!("{}.ccpub", groups.first().map_or("Sheets", |g| g.stem.as_str())), |n| n.to_string_lossy().to_string());
        if let Some(path) = app.services.pick_save.as_ref().and_then(|pick| pick(&suggested)) {
            let path = if path.to_ascii_lowercase().ends_with(".ccpub") { path } else { format!("{path}.ccpub") };
            let report = save_list(app, &groups, &mut st, &path);
            st.report = Some(report);
        }
    }
    if act.choose {
        let stem = groups.first().map_or_else(|| "Drawing".to_string(), |g| g.stem.clone());
        if let Some(pick) = app.services.pick_save.as_ref()
            && let Some(path) = pick(&format!("{stem}.{}", st.to.id()))
            && let Some(dir) = std::path::Path::new(&path).parent()
        {
            st.folder = Some(dir.to_string_lossy().to_string());
        }
    }
    if act.publish {
        match run(app, &groups, &st) {
            // In the background the window closes; the report opens when the run ends.
            Ok(true) => *open = false,
            Ok(false) => {}
            Err(report) => {
                app.set_status(report.clone());
                st.report = Some(report);
            }
        }
        // The files exist now; check again so the names and statuses are current.
        st.checked = None;
    }
    if st.editing_stamp {
        stamp_window(ctx, &mut st);
    }
    ctx.data_mut(|d| d.insert_temp(id, st));
}

/// Publish the ticked sheets with one engine `publish` call: on a worker thread, or now when
/// "Run in the background" is off. `Ok(true)` when it runs in the background; `Err` says why it
/// didn't start.
fn run(app: &mut CadApp, groups: &[Group], st: &PublishState) -> Result<bool, String> {
    let sheets = picked(groups, st);
    let Some((first, first_sheet)) = sheets.first() else {
        return Err(crate::tl!("Nothing selected").to_string());
    };
    let mut request = publish_request(groups, st);
    let mut folder = default_folder(groups, st);
    if st.ask_name || folder.is_empty() {
        let suggested = st
            .checked
            .as_ref()
            .and_then(|c| c.files.first().cloned())
            .unwrap_or_else(|| format!("{}-{}.{}", first.stem, first_sheet.name, st.to.id()));
        let Some(pick) = app.services.pick_save.as_ref() else {
            return Err(crate::tl!("Choose a folder to publish to.").to_string());
        };
        let Some(path) = pick(&suggested) else { return Err(crate::tl!("Publishing cancelled.").to_string()) };
        let p = std::path::Path::new(&path);
        folder = p.parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_default();
        // A combined file takes the name given; braces in it stay braces.
        if combining(st)
            && let Some(stem) = p.file_stem().map(|n| n.to_string_lossy().to_string()).filter(|n| !n.is_empty())
        {
            request["name"] = json!(stem.replace('{', "{{").replace('}', "}}"));
        }
    }
    request["folder"] = json!(folder);
    let background = !st.foreground && !cfg!(target_arch = "wasm32");
    crate::publish_run::start(app, request, background, st.open_result)?;
    Ok(background)
}

/// The Sheet Stamp window: the stamp's text, corner and height, with a sample of how it reads.
fn stamp_window(ctx: &egui::Context, st: &mut PublishState) {
    let t = Tokens::get();
    let mut open = true;
    let mut done = false;
    egui::Window::new(crate::tl!("Sheet Stamp"))
        .id(egui::Id::new("cc_publish_stamp"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(420.0)
        .pivot(Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .show(ctx, |ui| {
            ui.label(RichText::new(crate::tl!("Text")).color(t.text_dim));
            ui.add(egui::TextEdit::singleline(&mut st.stamp.text).desired_width(f32::INFINITY).char_limit(200));
            ui.label(
                RichText::new(crate::tl!("Use {drawing}, {sheet}, {n}, {date} and {time}. The date and time are UTC."))
                    .font(crate::theme::small())
                    .color(t.text_faint),
            );
            ui.add_space(6.0);
            egui::Grid::new("cc_publish_stamp_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label(RichText::new(crate::tl!("Corner")).color(t.text_dim));
                let shown = CORNERS.get(st.stamp.corner).map_or("", |c| c.1);
                egui::ComboBox::from_id_salt("cc_publish_stamp_corner").selected_text(crate::i18n::t(shown)).show_ui(ui, |ui| {
                    for (i, (_, label)) in CORNERS.iter().enumerate() {
                        ui.selectable_value(&mut st.stamp.corner, i, crate::i18n::t(label));
                    }
                });
                ui.end_row();
                ui.label(RichText::new(crate::tl!("Height on paper")).color(t.text_dim));
                ui.add(egui::DragValue::new(&mut st.stamp.height).range(0.5..=50.0).speed(0.1).suffix(" mm"));
                ui.end_row();
            });
            ui.add_space(6.0);
            let sample = st
                .stamp
                .text
                .replace("{{", "\u{1}")
                .replace("}}", "\u{2}")
                .replace("{drawing}", "Drawing1")
                .replace("{sheet}", "Layout1")
                .replace("{n}", "1")
                .replace("{date}", "2026-10-15")
                .replace("{time}", "09:30 UTC")
                .replace('\u{1}', "{")
                .replace('\u{2}', "}");
            ui.label(RichText::new(crate::tl!("Example")).color(t.text_dim));
            ui.label(RichText::new(sample).font(crate::theme::mono()).color(t.text));
            ui.separator();
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button(crate::tl!("Done")).clicked() {
                    done = true;
                }
                if ui.button(crate::tl!("Reset")).clicked() {
                    st.stamp = StampSettings::default();
                }
            });
        });
    if !open || done {
        st.editing_stamp = false;
        if st.stamp.text.trim().is_empty() {
            st.stamp = StampSettings::default();
        }
    }
}

fn body(ui: &mut Ui, groups: &[Group], st: &mut PublishState, act: &mut Actions) {
    let size = ui.available_size();
    let footer = 34.0;
    let main_h = (size.y - footer - 14.0).max(240.0);
    // The separator between the columns takes its own width plus the item spacing on each side;
    // counting it keeps the content inside the window, which would otherwise grow every frame.
    let gap = 2.0 * ui.spacing().item_spacing.x + 8.0;
    let list_w = (size.x - INSPECTOR - gap).max(320.0);
    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(vec2(list_w, main_h), Layout::top_down(Align::Min), |ui| {
            ui.set_min_size(vec2(list_w, main_h));
            list_column(ui, groups, st, act, main_h);
        });
        ui.separator();
        ui.allocate_ui_with_layout(vec2(INSPECTOR, main_h), Layout::top_down(Align::Min), |ui| {
            ui.set_min_size(vec2(INSPECTOR, main_h));
            inspector(ui, groups, st, act, main_h);
        });
    });
    ui.separator();
    footer_row(ui, groups, st, act);
}

fn list_column(ui: &mut Ui, groups: &[Group], st: &mut PublishState, act: &mut Actions, height: f32) {
    let t = Tokens::get();
    let total: usize = groups.iter().map(|g| g.sheets.len()).sum();
    let n = picked(groups, st).len();
    ui.horizontal(|ui| {
        let list_name = st.list.as_deref().and_then(|l| std::path::Path::new(l).file_name()).map(|n| n.to_string_lossy().to_string());
        let title = list_name.unwrap_or_else(|| crate::tl!("Unsaved list").to_string());
        ui.menu_button(format!("{title} ⏷"), |ui| {
            if ui.button(crate::tl!("New List")).on_hover_text(crate::tl!("Every open drawing's sheets")).clicked() {
                *st = PublishState { folder: st.folder.clone(), tab: st.tab, ..PublishState::default() };
                ui.close();
            }
            if ui.button(crate::tl!("Open List...")).clicked() {
                act.open_list = true;
                ui.close();
            }
            if ui.button(crate::tl!("Save List As...")).clicked() {
                act.save_list = true;
                ui.close();
            }
        })
        .response
        .on_hover_text(st.list.clone().unwrap_or_default());
        ui.label(RichText::new(crate::tf!("{n} of {total} sheets", n = n, total = total)).color(t.text_faint));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let inspected = st.inspected.clone();
            if ui.add_enabled(inspected.is_some(), egui::Button::new("↓")).on_hover_text(crate::tl!("Move down")).clicked() {
                move_sheet(groups, st, 1);
            }
            if ui.add_enabled(inspected.is_some(), egui::Button::new("↑")).on_hover_text(crate::tl!("Move up")).clicked() {
                move_sheet(groups, st, -1);
            }
            if ui.button(crate::tl!("Add Drawings...")).on_hover_text(crate::tl!("Add a drawing's sheets without opening it")).clicked() {
                act.add = true;
            }
            ui.separator();
            if ui.small_button(crate::tl!("None")).clicked() {
                for g in groups {
                    for s in &g.sheets {
                        st.skipped.insert((g.uid, s.name.clone()));
                    }
                }
            }
            if ui.small_button(crate::tl!("All")).clicked() {
                st.skipped.clear();
            }
        });
    });
    ui.add_space(4.0);
    let table_h = (height - 70.0).max(120.0);
    let frame = egui::Frame::NONE.fill(t.list_bg).stroke(Stroke::new(1.0, t.border)).corner_radius(4.0);
    frame.show(ui, |ui| {
        ui.set_min_size(vec2(ui.available_width(), table_h));
        ui.spacing_mut().item_spacing.y = 0.0;
        header_row(ui);
        egui::ScrollArea::vertical().id_salt("cc_publish_list").max_height(table_h - ROW).auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            if groups.is_empty() {
                ui.add_space(12.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(crate::tl!("No drawings are open. Use Add Drawings... to open one.")).color(t.text_faint))
                });
            }
            for g in groups {
                group_row(ui, g, st);
                for s in &g.sheets {
                    sheet_row(ui, g, s, st);
                }
            }
        });
    });
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        let mut every = !st.only_current;
        if ui.checkbox(&mut every, crate::tl!("Include every open drawing")).changed() {
            st.only_current = !every;
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(crate::tl!("Tick to publish · click a row to inspect it")).color(t.text_faint).font(crate::theme::small()));
        });
    });
}

fn move_sheet(groups: &[Group], st: &mut PublishState, dir: isize) {
    let Some((uid, name)) = st.inspected.clone() else { return };
    let Some(g) = groups.iter().find(|g| g.uid == uid) else { return };
    let mut names: Vec<String> = g.sheets.iter().map(|s| s.name.clone()).collect();
    let Some(i) = names.iter().position(|n| *n == name) else { return };
    let Some(j) = i.checked_add_signed(dir).filter(|j| *j < names.len()) else { return };
    names.swap(i, j);
    st.order.insert(uid, names);
}

/// Column rectangles for a row: (name, setup, paper, status).
fn columns(row: Rect) -> [Rect; 4] {
    let x0 = row.left() + INDENT + CHECK + GAP;
    let right = row.right() - GAP;
    let flex = (right - x0 - PAPER_COL - STATUS_COL - 3.0 * GAP).max(80.0);
    let name_w = flex * 1.3 / 2.3;
    let setup_w = flex - name_w;
    let mut x = x0;
    let mut out = [Rect::NOTHING; 4];
    for (i, w) in [name_w, setup_w, PAPER_COL, STATUS_COL].into_iter().enumerate() {
        if let Some(r) = out.get_mut(i) {
            *r = Rect::from_min_max(pos2(x, row.top()), pos2(x + w, row.bottom()));
        }
        x += w + GAP;
    }
    out
}

fn cell_text(p: &Painter, r: Rect, text: &str, color: Color32, font: FontId) {
    p.with_clip_rect(r).text(pos2(r.left(), r.center().y), Align2::LEFT_CENTER, text, font, color);
}

fn header_row(ui: &mut Ui) {
    let t = Tokens::get();
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::hover());
    let p = ui.painter();
    p.line_segment([rect.left_bottom(), rect.right_bottom()], Stroke::new(1.0, t.border));
    let font = crate::theme::small();
    let [a, b, c, d] = columns(rect);
    let a = Rect::from_min_max(pos2(rect.left() + GAP + CHECK + GAP, a.top()), a.max);
    for (r, label) in [(a, "Sheet"), (b, "Page setup"), (c, "Paper"), (d, "Status")] {
        cell_text(p, r, crate::i18n::t(label), t.text_faint, font.clone());
    }
}

fn paint_check(p: &Painter, r: Rect, state: Option<bool>) {
    let t = Tokens::get();
    let on = state != Some(false);
    p.rect(r, 2.0, if on { t.accent } else { t.field }, Stroke::new(1.0, if on { t.accent } else { t.text_faint }), StrokeKind::Inside);
    let w = Stroke::new(1.6, t.paper);
    match state {
        Some(true) => {
            let (x, y, s) = (r.left(), r.top(), r.width());
            p.line_segment([pos2(x + s * 0.22, y + s * 0.52), pos2(x + s * 0.42, y + s * 0.72)], w);
            p.line_segment([pos2(x + s * 0.42, y + s * 0.72), pos2(x + s * 0.78, y + s * 0.3)], w);
        }
        None => {
            p.line_segment([pos2(r.left() + 3.0, r.center().y), pos2(r.right() - 3.0, r.center().y)], w);
        }
        Some(false) => {}
    }
}

fn group_row(ui: &mut Ui, g: &Group, st: &mut PublishState) {
    let t = Tokens::get();
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::click());
    let on = g.sheets.iter().filter(|s| !st.skipped.contains(&(g.uid, s.name.clone()))).count();
    let state = if on == 0 {
        Some(false)
    } else if on == g.sheets.len() {
        Some(true)
    } else {
        None
    };
    let check = Rect::from_center_size(pos2(rect.left() + GAP + CHECK / 2.0, rect.center().y), vec2(CHECK, CHECK));
    let remove = Rect::from_center_size(pos2(rect.right() - GAP - 7.0, rect.center().y), vec2(18.0, 18.0));
    if resp.clicked()
        && let Some(at) = resp.interact_pointer_pos()
    {
        if remove.expand(2.0).contains(at) {
            match &g.source {
                Source::Open(_) => {
                    st.removed.insert(g.uid);
                }
                Source::File(_) => st.files.retain(|f| f.uid != g.uid),
            }
        } else {
            let all_on = state == Some(true);
            for s in &g.sheets {
                let key = (g.uid, s.name.clone());
                if all_on {
                    st.skipped.insert(key);
                } else {
                    st.skipped.remove(&key);
                }
            }
        }
    }
    crate::icons::describe_control(ui, &resp, &g.title, state, false);
    let p = ui.painter();
    p.rect_filled(rect, 0.0, t.chrome_dark);
    p.line_segment([rect.left_bottom(), rect.right_bottom()], Stroke::new(1.0, t.border));
    paint_check(p, check, state);
    let font = crate::theme::body();
    let title = p.text(pos2(check.right() + GAP, rect.center().y), Align2::LEFT_CENTER, &g.title, FontId::proportional(font.size), t.text);
    let (note, color) = match &g.source {
        Source::Open(_) if g.current => (crate::tl!("current"), t.text_faint),
        Source::Open(_) => (crate::tl!("open"), t.text_faint),
        Source::File(_) if g.found => (crate::tl!("not open"), t.text_faint),
        Source::File(_) => (crate::tl!("File not found"), t.warn),
    };
    p.text(pos2(title.right() + GAP, rect.center().y), Align2::LEFT_CENTER, note, crate::theme::small(), color);
    ui.ctx().accesskit_node_builder(resp.id, |node| node.set_value(note.to_string()));
    let count = crate::tf!("{n} of {total}", n = on, total = g.sheets.len());
    p.text(pos2(remove.left() - GAP, rect.center().y), Align2::RIGHT_CENTER, count, crate::theme::small(), t.text_dim);
    let hot = resp.hovered() && resp.hover_pos().is_some_and(|h| remove.expand(2.0).contains(h));
    let c = if hot { t.text } else { t.text_faint };
    let k = remove.shrink(5.0);
    p.line_segment([k.left_top(), k.right_bottom()], Stroke::new(1.3, c));
    p.line_segment([k.right_top(), k.left_bottom()], Stroke::new(1.3, c));
    if hot {
        resp.on_hover_text(crate::tl!("Remove from the list"));
    }
}

fn sheet_row(ui: &mut Ui, g: &Group, s: &Sheet, st: &mut PublishState) {
    let t = Tokens::get();
    let key = (g.uid, s.name.clone());
    let on = !st.skipped.contains(&key);
    let inspected = st.inspected.as_ref() == Some(&key);
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::click());
    let check = Rect::from_center_size(pos2(rect.left() + INDENT + CHECK / 2.0, rect.center().y), vec2(CHECK, CHECK));
    if resp.clicked()
        && let Some(at) = resp.interact_pointer_pos()
    {
        if check.expand(3.0).contains(at) {
            if on {
                st.skipped.insert(key.clone());
            } else {
                st.skipped.remove(&key);
            }
        } else {
            st.inspected = Some(key.clone());
        }
    }
    if resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Space)) {
        if on {
            st.skipped.insert(key.clone());
        } else {
            st.skipped.remove(&key);
        }
    }
    crate::icons::describe_control(ui, &resp, &s.name, Some(on), false);
    // The status is painted, so screen readers get it as the row's value and the reason as its
    // description.
    let (label, color, reason) = sheet_status(g, s, st);
    ui.ctx().accesskit_node_builder(resp.id, |node| {
        node.set_value(label.clone());
        if !reason.is_empty() {
            node.set_description(reason.clone());
        }
    });
    let p = ui.painter();
    if inspected {
        p.rect(rect.shrink2(vec2(1.0, 0.5)), 0.0, t.tab_active, Stroke::new(1.0, t.accent), StrokeKind::Inside);
    } else if resp.hovered() {
        p.rect_filled(rect, 0.0, t.list_row);
    }
    paint_check(p, check, Some(on));
    let fg = if on { t.text } else { t.text_faint };
    let font = crate::theme::body();
    let [name, setup, paper, status] = columns(rect);
    let glyph = Rect::from_center_size(pos2(name.left() + 6.0, rect.center().y), vec2(11.0, 11.0));
    let gs = Stroke::new(1.2, t.text_faint);
    p.rect_stroke(glyph, 1.0, gs, StrokeKind::Inside);
    if s.model {
        p.line_segment([pos2(glyph.left() + 1.5, glyph.bottom() - 3.0), pos2(glyph.center().x, glyph.center().y - 1.0)], gs);
        p.line_segment([pos2(glyph.center().x, glyph.center().y - 1.0), pos2(glyph.right() - 1.5, glyph.bottom() - 4.0)], gs);
    } else {
        let tb = Rect::from_min_max(pos2(glyph.center().x + 0.5, glyph.bottom() - 4.0), pos2(glyph.right() - 1.5, glyph.bottom() - 1.5));
        p.rect_stroke(tb, 0.0, gs, StrokeKind::Inside);
    }
    let name_text = Rect::from_min_max(pos2(glyph.right() + 6.0, name.top()), name.max);
    cell_text(p, name_text, &s.name, fg, font.clone());
    cell_text(p, setup, crate::i18n::t(&s.setup), if on { t.text_dim } else { t.text_faint }, font.clone());
    cell_text(p, paper, &paper_text(s), if on { t.text_dim } else { t.text_faint }, font.clone());
    cell_text(p, status, &label, if on { color } else { t.text_faint }, font);
    if !reason.is_empty() && resp.hovered() {
        resp.on_hover_text(reason);
    }
}

/// The Status column: the last check's answer, or "Checking..." until it comes.
fn sheet_status(g: &Group, s: &Sheet, st: &PublishState) -> (String, Color32, String) {
    let t = Tokens::get();
    let key = (g.uid, s.name.clone());
    if !g.found {
        let path = match &g.source {
            Source::File(p) => p.clone(),
            Source::Open(_) => g.title.clone(),
        };
        return (crate::tl!("File not found").to_string(), t.warn, crate::tf!("{path} isn't there any more", path = path));
    }
    let Some((status, reason)) = st.checked.as_ref().and_then(|c| c.sheets.get(&key)) else {
        return (crate::tl!("Checking...").to_string(), t.text_faint, String::new());
    };
    let changed = st.changes.contains_key(&key);
    match status.as_str() {
        "error" => (crate::tl!("Will fail").to_string(), t.warn, reason.clone()),
        "default" => (crate::tl!("Uses default").to_string(), t.text_faint, reason.clone()),
        _ if changed => (crate::tl!("Changed for this run").to_string(), t.text_dim, reason.clone()),
        _ => (crate::tl!("Ready").to_string(), t.text_dim, reason.clone()),
    }
}

fn paper_text(s: &Sheet) -> String {
    if s.model {
        return crate::tl!("Fit to paper").to_string();
    }
    if s.size.is_empty() {
        return s.scale.clone();
    }
    format!("{} · {}", s.size.trim_end_matches(" mm").replace(" × ", "×"), s.scale)
}

/// A row of equal segments; returns the clicked index. `enabled[i] == false` greys a segment out.
fn segmented(ui: &mut Ui, id: &str, labels: &[&str], selected: usize, enabled: &[bool]) -> Option<usize> {
    let t = Tokens::get();
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, 28.0), Sense::hover());
    ui.painter().rect(rect, 7.0, t.chrome_dark, Stroke::new(1.0, t.border), StrokeKind::Inside);
    let n = labels.len().max(1) as f32;
    let inner = rect.shrink(2.0);
    let cw = (inner.width() - 2.0 * (n - 1.0)) / n;
    let mut clicked = None;
    for (i, label) in labels.iter().enumerate() {
        let r = Rect::from_min_size(pos2(inner.left() + i as f32 * (cw + 2.0), inner.top()), vec2(cw, inner.height()));
        let en = enabled.get(i).copied().unwrap_or(true);
        let resp = ui.interact(r, ui.id().with((id, i)), if en { Sense::click() } else { Sense::hover() });
        let on = i == selected;
        crate::icons::describe_control(ui, &resp, label, Some(on), true);
        let p = ui.painter();
        if on {
            p.rect_filled(r, 5.0, t.control);
        } else if resp.hovered() && en {
            p.rect_filled(r, 5.0, t.control_hover.gamma_multiply(0.5));
        }
        let color = if !en {
            t.text_faint.gamma_multiply(0.6)
        } else if on {
            t.text
        } else {
            t.text_dim
        };
        p.text(r.center(), Align2::CENTER_CENTER, crate::i18n::t(label), crate::theme::small(), color);
        if !en {
            resp.on_hover_text(crate::tl!("Not available yet"));
        } else if resp.clicked() {
            clicked = Some(i);
        }
    }
    clicked
}

fn section(ui: &mut Ui, label: &str) {
    ui.add_space(6.0);
    ui.label(RichText::new(crate::i18n::t(label).to_uppercase()).font(FontId::proportional(11.0)).color(Tokens::get().text_faint).strong());
}

fn inspector(ui: &mut Ui, groups: &[Group], st: &mut PublishState, act: &mut Actions, height: f32) {
    let t = Tokens::get();
    let tab = if st.tab == Tab::Sheet { 0 } else { 1 };
    if let Some(i) = segmented(ui, "cc_publish_tab", &["Sheet", "Output"], tab, &[true, true]) {
        st.tab = if i == 0 { Tab::Sheet } else { Tab::Output };
    }
    let body_h = (height - 28.0 - 104.0).max(120.0);
    egui::ScrollArea::vertical().id_salt("cc_publish_inspector").max_height(body_h).auto_shrink([false, false]).show(ui, |ui| match st.tab {
        Tab::Sheet => sheet_tab(ui, groups, st),
        Tab::Output => output_tab(ui, groups, st, act),
    });
    ui.separator();
    ui.horizontal(|ui| {
        ui.checkbox(&mut st.stamp_on, crate::tl!("Add a sheet stamp")).on_hover_text(crate::tl!("A line of text in a corner of every sheet"));
        if ui.add_enabled(st.stamp_on, egui::Button::new(crate::tl!("Edit stamp...")).small()).clicked() {
            st.editing_stamp = true;
        }
    });
    let mut background = !st.foreground;
    let threads = !cfg!(target_arch = "wasm32");
    let resp = ui.add_enabled(threads, egui::Checkbox::new(&mut background, crate::tl!("Run in the background")));
    if threads {
        resp.on_hover_text(crate::tl!("Keep working while the sheets are published; a report opens when they're done"));
    } else {
        resp.on_disabled_hover_text(crate::tl!("The web version publishes while you wait."));
    }
    st.foreground = !background;
    ui.add_enabled(act.can_open, egui::Checkbox::new(&mut st.open_result, crate::tl!("Open the result when done")))
        .on_disabled_hover_text(crate::tl!("Not available yet"));
    let _ = t;
}

fn sheet_tab(ui: &mut Ui, groups: &[Group], st: &mut PublishState) {
    let t = Tokens::get();
    let inspected = st.inspected.clone();
    let found = inspected
        .as_ref()
        .and_then(|(uid, name)| groups.iter().find(|g| g.uid == *uid).and_then(|g| g.sheets.iter().find(|s| s.name == *name).map(|s| (g, s))));
    let Some((g, s)) = found else {
        ui.add_space(12.0);
        ui.label(RichText::new(crate::tl!("Click a sheet to see it here.")).color(t.text_faint));
        return;
    };
    ui.add_space(8.0);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 150.0), Sense::hover());
    let p = ui.painter();
    p.rect(rect, 4.0, t.canvas, Stroke::new(1.0, t.border), StrokeKind::Inside);
    let paper = if s.landscape || s.model { vec2(176.0, 124.0) } else { vec2(88.0, 124.0) };
    let sheet = Rect::from_center_size(rect.center(), paper);
    p.rect_filled(sheet.translate(vec2(0.0, 2.0)), 1.0, t.chrome_dark);
    p.rect_filled(sheet, 1.0, t.paper);
    let ink = Stroke::new(1.5, t.canvas_ink.gamma_multiply(0.6));
    let frame = Rect::from_min_size(sheet.min + vec2(sheet.width() * 0.12, sheet.height() * 0.16), vec2(sheet.width() * 0.54, sheet.height() * 0.6));
    p.rect_stroke(frame, 0.0, ink, StrokeKind::Inside);
    if !s.model {
        let tb = Rect::from_min_size(
            pos2(sheet.right() - sheet.width() * 0.3, sheet.bottom() - sheet.height() * 0.22),
            vec2(sheet.width() * 0.24, sheet.height() * 0.14),
        );
        p.rect_stroke(tb, 0.0, Stroke::new(1.0, ink.color), StrokeKind::Inside);
    }
    p.text(rect.right_bottom() - vec2(8.0, 6.0), Align2::RIGHT_BOTTOM, crate::tl!("Preview"), crate::theme::small(), t.text_faint);

    section(ui, "Page setup");
    let key = (g.uid, s.name.clone());
    let mut change = st.changes.get(&key).cloned();
    let saved = crate::tf!("As saved ({setup})", setup = crate::i18n::t(&s.setup));
    let shown = change.as_ref().map_or_else(|| saved.clone(), |c| c.paper.clone());
    egui::ComboBox::from_id_salt("cc_publish_setup")
        .width(ui.available_width())
        .selected_text(shown)
        .show_ui(ui, |ui| {
            if ui.selectable_label(change.is_none(), &saved).clicked() {
                change = None;
            }
            for paper in PAPERS {
                let on = change.as_ref().is_some_and(|c| c.paper == *paper);
                if ui.selectable_label(on, *paper).clicked() {
                    let (landscape, fit) = change.as_ref().map_or((true, s.model || s.scale == "Fit"), |c| (c.landscape, c.fit));
                    change = Some(SetupChange { paper: (*paper).to_string(), landscape, fit });
                }
            }
        })
        .response
        .on_hover_text(crate::tl!("Change the paper for this run only. The drawing keeps its page setup."));
    if let Some(c) = change.as_mut() {
        ui.horizontal(|ui| {
            ui.radio_value(&mut c.landscape, false, crate::tl!("Portrait"));
            ui.radio_value(&mut c.landscape, true, crate::tl!("Landscape"));
            ui.checkbox(&mut c.fit, crate::tl!("Fit to paper"));
        });
        ui.label(RichText::new(crate::tl!("For this run only. The drawing keeps its page setup.")).font(crate::theme::small()).color(t.text_faint));
    }
    match change {
        Some(c) => st.changes.insert(key, c),
        None => st.changes.remove(&key),
    };

    section(ui, "Details");
    egui::Grid::new("cc_publish_details").num_columns(2).spacing([10.0, 5.0]).show(ui, |ui| {
        let kind = if s.model { crate::tl!("Model space") } else { crate::tl!("Layout") };
        let folder = if g.folder.is_empty() { crate::tl!("Not saved yet").to_string() } else { g.folder.clone() };
        let change = st.changes.get(&(g.uid, s.name.clone()));
        let paper = match change {
            Some(c) => format!("{}, {}", c.paper, if c.landscape { crate::tl!("landscape") } else { crate::tl!("portrait") }),
            None if s.model => crate::tl!("Fit to paper").to_string(),
            None => format!("{}, {}", s.size, if s.landscape { crate::tl!("landscape") } else { crate::tl!("portrait") }),
        };
        let setup = match st.changes.get(&(g.uid, s.name.clone())) {
            Some(c) => crate::tf!("{paper}, for this run", paper = c.paper),
            None if s.default_setup => crate::tl!("Drawing default").to_string(),
            None => s.setup.clone(),
        };
        let rows: [(&str, String); 7] = [
            ("Drawing", g.title.clone()),
            ("Folder", folder),
            ("Sheet", format!("{} ({kind})", s.name)),
            ("Device", s.device.clone()),
            ("Paper", paper),
            ("Scale", s.scale.clone()),
            ("Setup", setup),
        ];
        for (k, v) in rows {
            ui.label(RichText::new(crate::i18n::t(k)).color(t.text_faint));
            ui.add(egui::Label::new(v).truncate());
            ui.end_row();
        }
    });
}

fn output_tab(ui: &mut Ui, groups: &[Group], st: &mut PublishState, act: &mut Actions) {
    let t = Tokens::get();
    let n = picked(groups, st).len();
    section(ui, "Send to");
    let selected = match st.to {
        To::Pdf => 1,
        To::Svg => 2,
        To::Png => 3,
    };
    // Sending to the page setup's plotter isn't available yet; files are.
    if let Some(i) = segmented(ui, "cc_publish_to", &["Page setup", "PDF", "SVG", "PNG"], selected, &[false, true, true, true]) {
        st.to = match i {
            2 => To::Svg,
            3 => To::Png,
            _ => To::Pdf,
        };
    }
    let about = match st.to {
        To::Pdf => crate::tl!("Vector PDF. Text stays selectable."),
        To::Svg => crate::tl!("Vector SVG, one file per sheet."),
        To::Png => crate::tl!("PNG image, one file per sheet."),
    };
    ui.label(RichText::new(about).font(crate::theme::small()).color(t.text_faint));

    ui.add_space(6.0);
    egui::Grid::new("cc_publish_pdf").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        if st.to == To::Png {
            ui.label(RichText::new(crate::tl!("Quality")).color(t.text_dim));
            ui.add_enabled_ui(false, |ui| {
                egui::ComboBox::from_id_salt("cc_publish_quality").width(200.0).selected_text(crate::tl!("Standard — 300 dpi")).show_ui(ui, |_| {})
            })
            .response
            .on_disabled_hover_text(crate::tl!("Not available yet"));
            ui.end_row();
        }
        ui.label(RichText::new(crate::tl!("Overlaps")).color(t.text_dim));
        ui.add_enabled_ui(false, |ui| {
            egui::ComboBox::from_id_salt("cc_publish_overlap").width(200.0).selected_text(crate::tl!("Last drawn on top")).show_ui(ui, |_| {})
        })
        .response
        .on_disabled_hover_text(crate::tl!("Not available yet"));
        ui.end_row();
    });
    if st.to == To::Pdf {
        let mut layers = !st.drop_layers;
        if ui.checkbox(&mut layers, crate::tl!("Keep layers (viewers can switch them on and off)")).changed() {
            st.drop_layers = !layers;
        }
    }

    section(ui, "Files");
    let pdf = st.to == To::Pdf;
    if let Some(i) = segmented(ui, "cc_publish_files", &["One per sheet", "Combine into one"], usize::from(combining(st)), &[true, pdf]) {
        st.combine = i == 1;
    }
    if !pdf {
        ui.label(RichText::new(crate::tl!("Only PDFs can be combined.")).font(crate::theme::small()).color(t.text_faint));
    }

    section(ui, "Save to");
    let mut folder = default_folder(groups, st);
    ui.horizontal(|ui| {
        let w = ui.available_width() - 80.0;
        if ui
            .add(egui::TextEdit::singleline(&mut folder).font(crate::theme::mono()).desired_width(w).hint_text(crate::tl!("Choose a folder")))
            .changed()
        {
            st.folder = Some(folder.clone());
        }
        if ui.button(crate::tl!("Choose...")).clicked() {
            act.choose = true;
        }
    });

    section(ui, "Names");
    ui.checkbox(&mut st.ask_name, crate::tl!("Ask for a name when publishing"));
    // The engine's names (from the last check), so the list shows exactly the files a run writes.
    let files: Vec<String> = match &st.checked {
        _ if n == 0 => Vec::new(),
        Some(c) if combining(st) => {
            let pages = n.saturating_sub(will_fail(groups, st));
            c.files.iter().map(|f| crate::tf!("{name} ({n} pages)", name = f, n = pages)).collect()
        }
        Some(c) => c.files.clone(),
        None => Vec::new(),
    };
    egui::Frame::NONE.fill(t.list_bg).stroke(Stroke::new(1.0, t.border)).corner_radius(4.0).inner_margin(egui::Margin::symmetric(8, 6)).show(
        ui,
        |ui| {
            ui.set_width(ui.available_width());
            egui::ScrollArea::vertical().id_salt("cc_publish_names").max_height(96.0).show(ui, |ui| {
                if n == 0 {
                    ui.label(RichText::new(crate::tl!("Select at least one sheet.")).color(t.text_faint));
                } else if files.is_empty() {
                    ui.label(RichText::new(crate::tl!("Checking...")).color(t.text_faint));
                }
                for f in files {
                    ui.add(egui::Label::new(RichText::new(f).font(crate::theme::mono()).color(t.text_dim)).truncate());
                }
            });
        },
    );
}

fn footer_row(ui: &mut Ui, groups: &[Group], st: &PublishState, act: &mut Actions) {
    let t = Tokens::get();
    let n = picked(groups, st).len();
    let mut summary = if n == 0 {
        crate::tl!("Nothing selected").to_string()
    } else if combining(st) {
        // Sheets that will fail aren't pages.
        crate::tf!("1 PDF with {n} pages", n = n.saturating_sub(will_fail(groups, st)))
    } else {
        crate::i18n::count(n as u64, "{n} file", "{n} files")
    };
    let failing = will_fail(groups, st);
    ui.horizontal(|ui| {
        if ui.button(" ? ").on_hover_text(crate::tl!("Help")).clicked() {
            act.help = true;
        }
        if failing > 0 {
            summary = format!("{summary} · {}", crate::tf!("{n} will fail", n = failing));
        }
        ui.label(RichText::new(summary).color(if failing > 0 { t.warn } else { t.text_dim }));
        if let Some(r) = &st.report {
            ui.add(egui::Label::new(RichText::new(r).font(crate::theme::small()).color(t.text_faint)).truncate());
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let label =
                if n == 0 { crate::tl!("Publish").to_string() } else { crate::i18n::count(n as u64, "Publish {n} Sheet", "Publish {n} Sheets") };
            let button = egui::Button::new(RichText::new(label).color(if n == 0 { t.text_faint } else { t.paper }).strong())
                .fill(if n == 0 { t.control } else { t.accent })
                .min_size(vec2(120.0, 26.0));
            let resp = ui.add_enabled(n > 0 && !act.busy, button);
            if act.busy {
                resp.on_disabled_hover_text(crate::tl!("Publishing is already running."));
            } else if resp.clicked() {
                act.publish = true;
            }
            if ui.add(egui::Button::new(crate::tl!("Cancel")).min_size(vec2(80.0, 26.0))).clicked() {
                act.close = true;
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratios_read_like_drawing_scales() {
        assert_eq!(ratio(0.02), "1:50");
        assert_eq!(ratio(2.0), "2:1");
        assert_eq!(ratio(1.0), "1:1");
        assert_eq!(ratio(0.0), "—");
        assert_eq!(ratio(f64::NAN), "—");
    }
}
