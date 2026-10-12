//! The engine side of the Hatch and Gradient dialog: GRADIENT's name and centred, points and
//! objects in one HATCH call, HATCHEDIT's gradient, HATCHPICK and HPDLGMODE.

use cadcraft_engine::Session;
use cadcraft_engine::doc::{EntityKind, Handle, Hatch};
use serde_json::{Value, json};

fn rect(s: &mut Session, a: [f64; 2], b: [f64; 2]) -> String {
    s.execute("rectang", &json!({ "p1": a, "p2": b })).unwrap()["handle"].as_str().unwrap().to_string()
}

/// The handle of the object added last.
fn last(s: &Session) -> String {
    s.doc().unwrap().model.iter().last().unwrap().handle.hex()
}

fn hatch(s: &Session, h: &str) -> Hatch {
    match &s.doc().unwrap().entity(Handle::parse_hex(h).unwrap()).unwrap().kind {
        EntityKind::Hatch(x) => x.clone(),
        k => panic!("not a hatch: {k:?}"),
    }
}

fn hatches(s: &Session) -> Vec<(String, Hatch)> {
    s.doc().unwrap().model.iter().filter_map(|e| if let EntityKind::Hatch(h) = &e.kind { Some((e.handle.hex(), h.clone())) } else { None }).collect()
}

fn count(s: &Session) -> usize {
    s.doc().unwrap().model.len()
}

fn undo_depth(s: &Session) -> usize {
    s.state().unwrap().undo.len()
}

fn handle(v: &Value) -> String {
    v["handle"].as_str().unwrap().to_string()
}

#[test]
fn gradient_takes_a_name_and_centred() {
    let mut s = Session::new();
    rect(&mut s, [0.0, 0.0], [10.0, 10.0]);
    let h = handle(
        &s.execute("gradient", &json!({ "points": [[5, 5]], "name": "invspherical", "centered": false, "color1": "1", "color2": "3", "angle": 45 }))
            .unwrap(),
    );
    let g = hatch(&s, &h).gradient.unwrap();
    assert_eq!(g.name, "INVSPHERICAL");
    assert!(!g.centered);
    assert!((g.angle - 45f64.to_radians()).abs() < 1e-12);
    // Linear and centred unless told otherwise.
    let h = handle(&s.execute("gradient", &json!({ "points": [[5, 5]] })).unwrap());
    let g = hatch(&s, &h).gradient.unwrap();
    assert_eq!((g.name.as_str(), g.centered), ("LINEAR", true));
    // An unknown name is refused and adds nothing.
    let n = count(&s);
    assert!(s.execute("gradient", &json!({ "points": [[5, 5]], "name": "RAINBOW" })).is_err());
    assert_eq!(count(&s), n);
}

#[test]
fn hatch_takes_points_and_objects_in_one_call() {
    let mut s = Session::new();
    rect(&mut s, [0.0, 0.0], [10.0, 10.0]);
    s.execute("circle", &json!({ "center": [30, 5], "radius": 3 })).unwrap();
    let c = last(&s);
    let depth = undo_depth(&s);
    s.execute("hatch", &json!({ "points": [[5, 5]], "handles": [c], "pattern": "SOLID" })).unwrap();
    let hs = hatches(&s);
    assert_eq!(hs.len(), 1, "one hatch for both picks");
    assert_eq!(hs[0].1.loops.len(), 2);
    assert_eq!(undo_depth(&s), depth + 1, "one undo step");
    // Separate: one hatch per point and one per object.
    s.execute("hatch", &json!({ "points": [[5, 5]], "handles": [c], "separate": true })).unwrap();
    assert_eq!(hatches(&s).len(), 3);
    s.undo().unwrap();
    assert_eq!(hatches(&s).len(), 1, "one undo removes both");
    // Points alone still ignore the selection.
    s.set_selection(vec![Handle::parse_hex(&c).unwrap()]);
    let h = handle(&s.execute("hatch", &json!({ "points": [[5, 5]] })).unwrap());
    assert_eq!(hatch(&s, &h).loops.len(), 1);
}

use cadcraft_engine::doc::color::{Color, Rgb};

#[test]
fn hatchedit_turns_a_pattern_into_a_gradient_and_back() {
    let mut s = Session::new();
    rect(&mut s, [0.0, 0.0], [10.0, 10.0]);
    let h = handle(&s.execute("hatch", &json!({ "points": [[5, 5]], "pattern": "ANSI31" })).unwrap());
    let depth = undo_depth(&s);
    s.execute(
        "hatchedit",
        &json!({ "handles": [h], "gradient": { "name": "cylinder", "color1": "red", "color2": "255,128,0", "angle": 30, "centered": false } }),
    )
    .unwrap();
    let x = hatch(&s, &h);
    let g = x.gradient.clone().unwrap();
    assert_eq!((g.name.as_str(), g.color1, g.color2, g.centered), ("CYLINDER", Color::Index(1), Color::True(Rgb(255, 128, 0)), false));
    assert!((g.angle - 30f64.to_radians()).abs() < 1e-12);
    assert!(x.solid);
    assert_eq!(x.pattern, "SOLID");
    assert_eq!(undo_depth(&s), depth + 1, "one undo step");
    // Only the fields given change.
    s.execute("hatchedit", &json!({ "handles": [h], "gradient": { "color2": "3" } })).unwrap();
    let g = hatch(&s, &h).gradient.unwrap();
    assert_eq!((g.name.as_str(), g.color1, g.color2), ("CYLINDER", Color::Index(1), Color::Index(3)));
    // A pattern turns it back.
    s.execute("hatchedit", &json!({ "handles": [h], "pattern": "ANSI31" })).unwrap();
    let x = hatch(&s, &h);
    assert!(x.gradient.is_none());
    assert!(!x.solid);
    s.undo().unwrap();
    assert!(hatch(&s, &h).gradient.is_some());
}

#[test]
fn hatchedit_refuses_a_bad_gradient_and_changes_nothing() {
    let mut s = Session::new();
    rect(&mut s, [0.0, 0.0], [10.0, 10.0]);
    let h = handle(&s.execute("hatch", &json!({ "points": [[5, 5]], "pattern": "ANSI31" })).unwrap());
    let before = hatch(&s, &h);
    for bad in [
        json!("x"),
        json!({ "name": "RAINBOW" }),
        json!({ "name": 5 }),
        json!({ "color1": "nope" }),
        json!({ "angle": "x" }),
        json!({ "centered": 1 }),
    ] {
        assert!(s.execute("hatchedit", &json!({ "handles": [h], "gradient": bad })).is_err(), "{bad}");
        assert_eq!(hatch(&s, &h), before, "{bad}");
    }
    assert!(s.execute("hatchedit", &json!({ "handles": [h], "pattern": "ANSI31", "gradient": { "name": "LINEAR" } })).is_err());
    assert_eq!(hatch(&s, &h), before);
}
