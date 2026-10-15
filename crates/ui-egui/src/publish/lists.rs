//! Publish lists: opening a `.ccpub` file (its drawings, ticks, order, page setup changes and output
//! settings) and saving the list as it is shown, through the engine's `publishlist`.

use serde_json::{Value, json};

use super::*;
use crate::CadApp;

/// Open a list: its drawings (open ones as they are, others from their files), its ticks, order
/// and page setup changes, and its output settings.
pub(super) fn open_list(app: &mut CadApp, st: &mut PublishState, path: &str) -> Result<(), String> {
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
pub(super) fn save_list(app: &mut CadApp, groups: &[Group], st: &mut PublishState, path: &str) -> String {
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
