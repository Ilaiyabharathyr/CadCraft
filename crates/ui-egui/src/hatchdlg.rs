//! The Hatch and Gradient dialog (HATCH, GRADIENT) and, on existing hatches, the Hatch Edit
//! dialog (HATCHEDIT, double-click): one window in two modes. It never edits the drawing itself:
//! picks go through HATCHPICK, and OK runs one HATCH, GRADIENT or HATCHEDIT call.

use cadcraft_color::Color;
use cadcraft_doc::{Drawing, EntityKind, Handle};
use cadcraft_engine::{HatchPicks, PickMode};
use serde_json::{Value, json};

use crate::CadApp;

/// The gradient shapes the dialog offers; each can also be inverted (`INV` + name).
pub(crate) const SHAPES: [&str; 5] = ["LINEAR", "CYLINDER", "CURVED", "SPHERICAL", "HEMISPHERICAL"];

/// HPORIGINMODE 1–5, by the names HATCH's `originMode` takes.
pub(crate) const ORIGIN_MODES: [&str; 5] = ["bottomLeft", "bottomRight", "topRight", "topLeft", "center"];

/// The fill and options the dialog shows.
#[derive(Clone, Debug, PartialEq)]
pub struct Fill {
    pub gradient: bool,
    pub pattern: String,
    pub scale: f64,
    /// Pattern or gradient angle, degrees.
    pub angle: f64,
    /// One of [`SHAPES`].
    pub shape: String,
    pub inverted: bool,
    pub color1: Color,
    pub color2: Color,
    pub centered: bool,
    /// "Specified origin" rather than "Use current origin".
    pub specified: bool,
    /// A clicked origin; when `None` a specified origin is `origin_mode` of the extents.
    pub origin: Option<[f64; 2]>,
    /// One of [`ORIGIN_MODES`].
    pub origin_mode: String,
    pub associative: bool,
    /// Island detection: 0 normal, 1 outer, 2 ignore.
    pub style: u8,
}

/// The dialog while open (and while a pick it started runs).
#[derive(Clone, Debug, PartialEq)]
pub struct HatchDlgState {
    pub fill: Fill,
    /// The fill as the dialog opened: edit mode sends only what changed.
    pub opened: Fill,
    /// The hatches being edited; empty in create mode.
    pub edit: Vec<Handle>,
    pub separate: bool,
    /// Internal points and boundary objects picked so far (create mode).
    pub points: Vec<[f64; 2]>,
    pub handles: Vec<Handle>,
    /// A HATCHPICK the dialog started that hasn't ended yet.
    pub picking: Option<PickMode>,
    /// The last command error, shown in the dialog.
    pub error: Option<String>,
    /// The drawing the dialog belongs to (`DocState::uid`).
    pub uid: u64,
}

/// Create mode, from the drawing's HATCH settings (HPNAME, HPSCALE, HPANG, HPASSOC, HPSEPARATE,
/// HPORIGINMODE, HPISLANDDETECTION); values a file got wrong fall back to the defaults.
pub fn create_state(d: &Drawing, uid: u64, gradient: bool) -> HatchDlgState {
    let h = &d.header;
    let scale = Some(h.f64("HPSCALE", 1.0)).filter(|v| v.is_finite() && *v > 0.0).unwrap_or(1.0);
    let angle = Some(h.f64("HPANG", 0.0).to_degrees()).filter(|v| v.is_finite()).unwrap_or(0.0);
    let mode = h.i64("HPORIGINMODE", 0);
    let origin_mode = mode.checked_sub(1).and_then(|i| usize::try_from(i).ok()).and_then(|i| ORIGIN_MODES.get(i)).copied().unwrap_or("bottomLeft");
    let fill = Fill {
        gradient,
        pattern: h.str("HPNAME", "ANSI31"),
        scale,
        angle,
        shape: "LINEAR".into(),
        inverted: false,
        color1: Color::Index(5),
        color2: Color::Index(7),
        centered: true,
        specified: mode != 0,
        origin: None,
        origin_mode: origin_mode.into(),
        associative: h.i64("HPASSOC", 1) != 0,
        style: u8::try_from(h.i64("HPISLANDDETECTION", 0).clamp(0, 2)).unwrap_or(0),
    };
    HatchDlgState {
        opened: fill.clone(),
        fill,
        edit: Vec::new(),
        separate: h.i64("HPSEPARATE", 0) != 0,
        points: Vec::new(),
        handles: Vec::new(),
        picking: None,
        error: None,
        uid,
    }
}

/// Edit mode on the hatches among `handles`, showing the first one; `None` when there are none.
pub fn edit_state(d: &Drawing, uid: u64, handles: Vec<Handle>) -> Option<HatchDlgState> {
    let hatches: Vec<Handle> = handles.into_iter().filter(|h| matches!(d.entity(*h).map(|e| &e.kind), Some(EntityKind::Hatch(_)))).collect();
    let first = hatches.first().and_then(|h| match &d.entity(*h)?.kind {
        EntityKind::Hatch(x) => Some(x.clone()),
        _ => None,
    })?;
    let mut st = create_state(d, uid, first.gradient.is_some());
    let f = &mut st.fill;
    f.pattern = first.pattern.clone();
    f.scale = if first.scale.is_finite() && first.scale > 0.0 { first.scale } else { 1.0 };
    f.angle = Some(first.angle.to_degrees()).filter(|a| a.is_finite()).unwrap_or(0.0);
    f.associative = first.associative;
    f.style = first.style.min(2);
    f.specified = false;
    f.origin = None;
    if let Some(g) = &first.gradient {
        let up = g.name.trim().to_ascii_uppercase();
        let (inverted, base) = match up.strip_prefix("INV") {
            Some(b) => (true, b.to_string()),
            None => (false, up.clone()),
        };
        f.shape = if SHAPES.contains(&base.as_str()) { base } else { "LINEAR".into() };
        f.inverted = inverted;
        f.color1 = g.color1;
        f.color2 = g.color2;
        f.angle = Some(g.angle.to_degrees()).filter(|a| a.is_finite()).unwrap_or(0.0);
        f.centered = g.centered;
    }
    st.opened = st.fill.clone();
    st.edit = hatches;
    st.separate = false;
    Some(st)
}

fn style_name(s: u8) -> &'static str {
    match s {
        1 => "outer",
        2 => "ignore",
        _ => "normal",
    }
}

fn gradient_name(f: &Fill) -> String {
    format!("{}{}", if f.inverted { "INV" } else { "" }, f.shape)
}

fn hexes(hs: &[Handle]) -> Value {
    json!(hs.iter().map(|h| h.hex()).collect::<Vec<_>>())
}

/// The command OK runs: HATCH or GRADIENT with the picks (create mode), or HATCHEDIT with what
/// changed (edit mode). `None` when there is nothing to run.
pub fn ok_call(st: &HatchDlgState) -> Option<(&'static str, Value)> {
    let f = &st.fill;
    let mut p = serde_json::Map::new();
    if st.edit.is_empty() {
        if st.points.is_empty() && st.handles.is_empty() {
            return None;
        }
        if !st.points.is_empty() {
            p.insert("points".into(), json!(st.points));
        }
        if !st.handles.is_empty() {
            p.insert("handles".into(), hexes(&st.handles));
        }
        if f.gradient {
            p.insert("name".into(), json!(gradient_name(f)));
            p.insert("color1".into(), json!(f.color1.name()));
            p.insert("color2".into(), json!(f.color2.name()));
            p.insert("centered".into(), json!(f.centered));
        } else {
            p.insert("pattern".into(), json!(f.pattern));
            p.insert("scale".into(), json!(f.scale));
        }
        p.insert("angle".into(), json!(f.angle));
        p.insert("associative".into(), json!(f.associative));
        p.insert("separate".into(), json!(st.separate));
        p.insert("style".into(), json!(style_name(f.style)));
        match (f.specified, f.origin) {
            (true, Some(o)) => p.insert("origin".into(), json!(o)),
            (true, None) => p.insert("originMode".into(), json!(f.origin_mode)),
            (false, _) => p.insert("originMode".into(), json!("current")),
        };
        return Some((if f.gradient { "gradient" } else { "hatch" }, Value::Object(p)));
    }
    let o = &st.opened;
    if f.gradient {
        let changed = !o.gradient
            || f.shape != o.shape
            || f.inverted != o.inverted
            || f.color1 != o.color1
            || f.color2 != o.color2
            || f.angle != o.angle
            || f.centered != o.centered;
        if changed {
            p.insert(
                "gradient".into(),
                json!({ "name": gradient_name(f), "color1": f.color1.name(), "color2": f.color2.name(), "angle": f.angle, "centered": f.centered }),
            );
        }
    } else {
        if o.gradient || f.pattern != o.pattern {
            p.insert("pattern".into(), json!(f.pattern));
        }
        if f.scale != o.scale {
            p.insert("scale".into(), json!(f.scale));
        }
        if f.angle != o.angle {
            p.insert("angle".into(), json!(f.angle));
        }
    }
    if f.associative != o.associative {
        p.insert("associative".into(), json!(f.associative));
    }
    if f.style != o.style {
        p.insert("style".into(), json!(style_name(f.style)));
    }
    if f.specified {
        match f.origin {
            Some(q) => p.insert("origin".into(), json!(q)),
            None => p.insert("originMode".into(), json!(f.origin_mode)),
        };
    }
    if p.is_empty() {
        return None;
    }
    p.insert("handles".into(), hexes(&st.edit));
    Some(("hatchedit", Value::Object(p)))
}

/// Take a HATCHPICK's picks into the dialog (Hatch picks are taken by `resume`).
pub fn merge_picks(st: &mut HatchDlgState, picks: HatchPicks) {
    match picks.mode {
        PickMode::Points | PickMode::Objects => {
            st.points.extend(picks.points.iter().map(|q| [q.x, q.y]));
            for h in picks.handles {
                if !st.handles.contains(&h) {
                    st.handles.push(h);
                }
            }
        }
        PickMode::Origin => {
            if let Some(q) = picks.origin {
                st.fill.origin = Some([q.x, q.y]);
                st.fill.specified = true;
            }
        }
        PickMode::Hatch => {}
    }
    st.error = None;
}

/// Hide the dialog and run HATCHPICK in `mode`; [`resume`] brings the dialog back.
pub fn start_pick(app: &mut CadApp, mode: PickMode) {
    let Some(st) = app.hatch_dlg.as_mut() else { return };
    st.picking = Some(mode);
    st.error = None;
    app.session.hatch_picks = None;
    app.ui.dialog = None;
    app.start("hatchpick");
    let kw = match mode {
        PickMode::Points => "Points",
        PickMode::Objects => "Objects",
        PickMode::Origin => "oRigin",
        PickMode::Hatch => "Hatch",
    };
    if app.session.running.is_some()
        && let Err(e) = app.session.input(cadcraft_engine::Input::Keyword(kw.into()))
    {
        app.session.echo(e.to_string());
    }
}

/// Every frame: once a HATCHPICK the dialog started has ended, take its picks and bring the
/// dialog back (unchanged when the pick was cancelled). A different drawing by then drops the
/// dialog and its picks; a Hatch pick that found none closes it.
pub fn resume(app: &mut CadApp) {
    let Some(mode) = app.hatch_dlg.as_ref().and_then(|s| s.picking) else { return };
    if app.session.running.is_some() {
        return;
    }
    let picks = app.session.hatch_picks.take();
    let uid = app.session.state().ok().map(|s| s.uid);
    let Some(mut st) = app.hatch_dlg.take() else { return };
    if uid != Some(st.uid) {
        return;
    }
    st.picking = None;
    if mode == PickMode::Hatch {
        let next = picks.and_then(|p| app.session.doc().ok().and_then(|d| edit_state(d, st.uid, p.handles)));
        let Some(next) = next else { return };
        st = next;
    } else if let Some(p) = picks {
        merge_picks(&mut st, p);
    }
    app.hatch_dlg = Some(st);
    app.ui.dialog = Some("hatch".into());
}

#[cfg(test)]
mod tests {
    use cadcraft_color::Color;
    use cadcraft_doc::Handle;
    use cadcraft_engine::geom::Vec2;
    use cadcraft_engine::{HatchPicks, PickMode, Session};
    use serde_json::json;

    use super::*;

    fn session_with_rect() -> (Session, Handle) {
        let mut s = Session::new();
        let r = s.execute("rectang", &json!({ "p1": [0, 0], "p2": [10, 10] })).unwrap();
        (s, Handle::parse_hex(r["handle"].as_str().unwrap()).unwrap())
    }

    use crate::{CadApp, Services};

    fn app_with_rect() -> (CadApp, Handle) {
        let (s, r) = session_with_rect();
        (CadApp::new(s, Services::default()), r)
    }

    fn open_create(app: &mut CadApp) {
        let st = app.session.state().unwrap();
        app.hatch_dlg = Some(create_state(&st.doc, st.uid, false));
        app.ui.dialog = Some("hatch".into());
    }

    #[test]
    fn pick_points_hides_the_dialog_and_brings_it_back_with_the_points() {
        let (mut app, _) = app_with_rect();
        open_create(&mut app);
        start_pick(&mut app, PickMode::Points);
        assert_eq!(app.ui.dialog, None, "hidden while picking");
        assert!(app.session.prompt_text().contains("Pick internal point"));
        resume(&mut app);
        assert_eq!(app.ui.dialog, None, "still picking");
        app.session.input(cadcraft_engine::Input::Point(Vec2::new(5.0, 5.0))).unwrap();
        app.session.input(cadcraft_engine::Input::Enter).unwrap();
        resume(&mut app);
        assert_eq!(app.ui.dialog.as_deref(), Some("hatch"));
        let st = app.hatch_dlg.clone().unwrap();
        assert_eq!((st.points, st.picking), (vec![[5.0, 5.0]], None));
    }

    #[test]
    fn escape_brings_the_dialog_back_unchanged() {
        let (mut app, r) = app_with_rect();
        open_create(&mut app);
        if let Some(st) = app.hatch_dlg.as_mut() {
            st.handles.push(r);
        }
        let before = app.hatch_dlg.clone();
        start_pick(&mut app, PickMode::Points);
        app.session.input(cadcraft_engine::Input::Point(Vec2::new(5.0, 5.0))).unwrap();
        app.session.cancel();
        resume(&mut app);
        assert_eq!(app.ui.dialog.as_deref(), Some("hatch"));
        assert_eq!(app.hatch_dlg, before);
    }

    #[test]
    fn switching_drawings_drops_the_dialog() {
        let (mut app, _) = app_with_rect();
        open_create(&mut app);
        start_pick(&mut app, PickMode::Points);
        app.session.input(cadcraft_engine::Input::Point(Vec2::new(5.0, 5.0))).unwrap();
        app.session.input(cadcraft_engine::Input::Enter).unwrap();
        app.session.new_drawing(false);
        resume(&mut app);
        assert_eq!(app.ui.dialog, None);
        assert_eq!(app.hatch_dlg, None);
    }

    #[test]
    fn hatch_pick_opens_edit_mode_or_nothing() {
        let (mut app, _) = app_with_rect();
        let hx = app.session.execute("hatch", &json!({ "points": [[5, 5]] })).unwrap()["handle"].as_str().unwrap().to_string();
        open_create(&mut app);
        start_pick(&mut app, PickMode::Hatch);
        app.session.input(cadcraft_engine::Input::Pick(vec![Handle::parse_hex(&hx).unwrap()])).unwrap();
        resume(&mut app);
        assert_eq!(app.ui.dialog.as_deref(), Some("hatch"));
        assert_eq!(app.hatch_dlg.clone().unwrap().edit, vec![Handle::parse_hex(&hx).unwrap()]);
        // Nothing picked: no dialog.
        start_pick(&mut app, PickMode::Hatch);
        app.session.input(cadcraft_engine::Input::Enter).unwrap();
        resume(&mut app);
        assert_eq!((app.ui.dialog.clone(), app.hatch_dlg.clone()), (None, None));
    }

    #[test]
    fn create_state_reads_the_header_and_survives_bad_values() {
        let (mut s, _) = session_with_rect();
        for (k, v) in [("HPNAME", json!("BRICK")), ("HPSCALE", json!(2.5)), ("HPANG", json!(30)), ("HPSEPARATE", json!(1)), ("HPASSOC", json!(0))] {
            s.execute("setvar", &json!({ "name": k, "value": v })).unwrap();
        }
        let st = create_state(s.doc().unwrap(), 7, false);
        let f = &st.fill;
        assert_eq!((f.pattern.as_str(), f.scale, f.associative, st.separate, st.uid), ("BRICK", 2.5, false, true, 7));
        assert!((f.angle - 30.0).abs() < 1e-9);
        assert!(st.edit.is_empty() && st.points.is_empty() && st.handles.is_empty());
        assert_eq!(st.fill, st.opened);
        // Hostile header values from a file.
        let h = &mut s.doc_mut().unwrap().header;
        h.set_f64("HPSCALE", f64::NAN);
        h.set_f64("HPANG", f64::INFINITY);
        h.set_i64("HPORIGINMODE", i64::MIN);
        h.set_i64("HPISLANDDETECTION", -5);
        let st = create_state(s.doc().unwrap(), 7, true);
        assert_eq!((st.fill.scale, st.fill.angle, st.fill.style, st.fill.specified), (1.0, 0.0, 0, true));
        assert!(ORIGIN_MODES.contains(&st.fill.origin_mode.as_str()));
        assert!(st.fill.gradient);
        s.doc_mut().unwrap().header.set_f64("HPSCALE", 0.0);
        s.doc_mut().unwrap().header.set_i64("HPORIGINMODE", 99);
        let st = create_state(s.doc().unwrap(), 7, false);
        assert_eq!(st.fill.scale, 1.0);
        assert!(ORIGIN_MODES.contains(&st.fill.origin_mode.as_str()));
    }

    #[test]
    fn create_ok_hatches_points_and_objects_in_one_call() {
        let (s, r) = session_with_rect();
        let mut st = create_state(s.doc().unwrap(), 1, false);
        assert_eq!(ok_call(&st), None, "no boundary yet");
        st.points.push([5.0, 5.0]);
        st.handles.push(r);
        st.fill.pattern = "ANSI37".into();
        st.fill.scale = 2.0;
        st.fill.angle = 15.0;
        st.fill.style = 1;
        let (cmd, p) = ok_call(&st).unwrap();
        assert_eq!(cmd, "hatch");
        assert_eq!(p["points"], json!([[5.0, 5.0]]));
        assert_eq!(p["handles"], json!([r.hex()]));
        assert_eq!((p["pattern"].as_str(), p["scale"].as_f64(), p["angle"].as_f64()), (Some("ANSI37"), Some(2.0), Some(15.0)));
        assert_eq!((p["style"].as_str(), p["originMode"].as_str()), (Some("outer"), Some("current")));
        // A clicked origin, then a corner of the extents.
        st.fill.specified = true;
        st.fill.origin = Some([1.0, 2.0]);
        assert_eq!(ok_call(&st).unwrap().1["origin"], json!([1.0, 2.0]));
        st.fill.origin = None;
        st.fill.origin_mode = "topRight".into();
        assert_eq!(ok_call(&st).unwrap().1["originMode"], json!("topRight"));
        // Gradient.
        st.fill.gradient = true;
        st.fill.shape = "SPHERICAL".into();
        st.fill.inverted = true;
        st.fill.color1 = Color::Index(1);
        st.fill.centered = false;
        let (cmd, p) = ok_call(&st).unwrap();
        assert_eq!(cmd, "gradient");
        assert_eq!((p["name"].as_str(), p["color1"].as_str(), p["centered"].as_bool()), (Some("INVSPHERICAL"), Some("Red"), Some(false)));
        assert!(p.get("pattern").is_none());
    }

    #[test]
    fn create_ok_runs_and_makes_one_hatch() {
        let (mut s, _) = session_with_rect();
        s.execute("circle", &json!({ "center": [30, 5], "radius": 3 })).unwrap();
        let c = s.doc().unwrap().model.iter().last().unwrap().handle;
        let mut st = create_state(s.doc().unwrap(), 1, false);
        st.points.push([5.0, 5.0]);
        st.handles.push(c);
        let (cmd, p) = ok_call(&st).unwrap();
        s.execute(cmd, &p).unwrap();
        let n = s.doc().unwrap().model.iter().filter(|e| matches!(e.kind, cadcraft_doc::EntityKind::Hatch(_))).count();
        assert_eq!(n, 1);
    }

    #[test]
    fn edit_ok_sends_only_what_changed() {
        let (mut s, _) = session_with_rect();
        let h = s.execute("hatch", &json!({ "points": [[5, 5]], "pattern": "ANSI31", "scale": 2 })).unwrap()["handle"].as_str().unwrap().to_string();
        let h = Handle::parse_hex(&h).unwrap();
        let mut st = edit_state(s.doc().unwrap(), 1, vec![h]).unwrap();
        assert_eq!((st.fill.pattern.as_str(), st.fill.scale, st.edit.clone()), ("ANSI31", 2.0, vec![h]));
        assert_eq!(ok_call(&st), None, "nothing changed: OK runs nothing");
        st.fill.scale = 3.0;
        let (cmd, p) = ok_call(&st).unwrap();
        assert_eq!(cmd, "hatchedit");
        assert_eq!(p, json!({ "handles": [h.hex()], "scale": 3.0 }));
        // To a gradient: the whole gradient, no pattern fields.
        st.fill.scale = 2.0;
        st.fill.gradient = true;
        let (_, p) = ok_call(&st).unwrap();
        assert_eq!(p["gradient"]["name"], json!("LINEAR"));
        assert!(p.get("pattern").is_none() && p.get("scale").is_none());
        s.execute("hatchedit", &p).unwrap();
        // Back from a gradient hatch: the pattern is sent even if its name didn't change.
        let mut st = edit_state(s.doc().unwrap(), 1, vec![h]).unwrap();
        assert!(st.fill.gradient);
        st.fill.gradient = false;
        st.fill.pattern = "ANSI31".into();
        assert_eq!(ok_call(&st).unwrap().1["pattern"], json!("ANSI31"));
        // Not a hatch: no edit state.
        let line = s.execute("line", &json!({ "points": [[20, 0], [30, 0]] })).unwrap()["handles"][0].as_str().unwrap().to_string();
        assert!(edit_state(s.doc().unwrap(), 1, vec![Handle::parse_hex(&line).unwrap()]).is_none());
    }

    #[test]
    fn picks_merge_into_the_state() {
        let (s, r) = session_with_rect();
        let mut st = create_state(s.doc().unwrap(), 1, false);
        st.error = Some("old".into());
        merge_picks(&mut st, HatchPicks { mode: PickMode::Points, points: vec![Vec2::new(5.0, 5.0)], ..HatchPicks::default() });
        merge_picks(&mut st, HatchPicks { mode: PickMode::Objects, handles: vec![r, r], ..HatchPicks::default() });
        merge_picks(&mut st, HatchPicks { mode: PickMode::Objects, handles: vec![r], ..HatchPicks::default() });
        assert_eq!((st.points.clone(), st.handles.clone(), st.error.clone()), (vec![[5.0, 5.0]], vec![r], None));
        merge_picks(&mut st, HatchPicks { mode: PickMode::Origin, origin: Some(Vec2::new(1.0, 2.0)), ..HatchPicks::default() });
        assert_eq!((st.fill.specified, st.fill.origin), (true, Some([1.0, 2.0])));
    }
}
