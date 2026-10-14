//! TRIM, FILLET and CHAMFER on rays and xlines (#1235): what is left of a ray with a finite end
//! is a line, what is left of an xline is a ray, as in AutoCAD. EXTEND moves a ray's base (#1236).

use cadcraft_engine::Session;
use cadcraft_engine::doc::EntityKind;

fn r(v: f64) -> f64 {
    (v * 1e6).round() / 1e6 + 0.0
}

/// Every object as text, sorted: lines with their ends in a fixed order, rays and xlines with
/// base and direction.
fn objects(s: &Session) -> Vec<String> {
    let mut v: Vec<String> = s
        .doc()
        .unwrap()
        .model
        .iter()
        .map(|e| match &e.kind {
            EntityKind::Line(l) => {
                let (a, b) = ((r(l.a.x), r(l.a.y)), (r(l.b.x), r(l.b.y)));
                let (a, b) = if a <= b { (a, b) } else { (b, a) };
                format!("LINE {a:?} {b:?}")
            }
            EntityKind::Ray(k) => format!("RAY {:?} {:?}", (r(k.base.x), r(k.base.y)), (r(k.dir.x), r(k.dir.y))),
            EntityKind::XLine(k) => format!("XLINE {:?} {:?}", (r(k.base.x), r(k.base.y)), (r(k.dir.x), r(k.dir.y))),
            EntityKind::Arc(_) => "ARC".to_string(),
            k => format!("{k:?}"),
        })
        .collect();
    v.sort();
    v
}

/// A vertical line at x = 10 and, along y = 0, the ray or xline drawn by `cmd`.
fn session(cmd: &str) -> Session {
    let mut s = Session::new();
    s.cmdline("LINE 10,-2 10,12").unwrap();
    s.cmdline("").unwrap();
    s.script(&format!("{cmd}\n\n")).unwrap();
    assert!(s.running.is_none(), "{}", s.prompt_text());
    s
}

fn run(cmd: &str, then: &str) -> Vec<String> {
    let mut s = session(cmd);
    s.script(then).unwrap();
    assert!(s.running.is_none(), "{cmd} / {then}: {}", s.prompt_text());
    objects(&s)
}

const EDGE: &str = "LINE (10.0, -2.0) (10.0, 12.0)";

/// The expected objects, sorted as `objects` sorts them.
fn want(v: &[&str]) -> Vec<String> {
    let mut v: Vec<String> = v.iter().map(|s| s.to_string()).collect();
    v.sort();
    v
}

#[test]
fn trim_a_ray_leaves_a_line_or_a_ray() {
    // The far side goes: the ray keeps its base and becomes a line.
    assert_eq!(run("RAY 0,0 1,0", "TRIM 15,0\n\n"), want(&[EDGE, "LINE (0.0, 0.0) (10.0, 0.0)"]));
    // The side with the base goes: what is left is still infinite, a ray from the edge.
    assert_eq!(run("RAY 0,0 1,0", "TRIM 5,0\n\n"), want(&[EDGE, "RAY (10.0, 0.0) (1.0, 0.0)"]));
}

#[test]
fn trim_an_xline_leaves_a_ray() {
    assert_eq!(run("XLINE H 0,0", "TRIM 15,0\n\n"), want(&[EDGE, "RAY (10.0, 0.0) (-1.0, 0.0)"]));
    assert_eq!(run("XLINE H 0,0", "TRIM 5,0\n\n"), want(&[EDGE, "RAY (10.0, 0.0) (1.0, 0.0)"]));
}

#[test]
fn trim_between_two_edges_splits_a_ray_and_an_xline() {
    let mut s = session("RAY -5,0 1,0");
    s.cmdline("LINE 0,-2 0,12").unwrap();
    s.cmdline("").unwrap();
    s.script("TRIM 5,0\n\n").unwrap();
    let edges = ["LINE (0.0, -2.0) (0.0, 12.0)", EDGE];
    assert_eq!(objects(&s), want(&[&edges[..], &["LINE (-5.0, 0.0) (0.0, 0.0)", "RAY (10.0, 0.0) (1.0, 0.0)"]].concat()));
    let mut s = session("XLINE H 0,0");
    s.cmdline("LINE 0,-2 0,12").unwrap();
    s.cmdline("").unwrap();
    s.script("TRIM 5,0\n\n").unwrap();
    assert_eq!(objects(&s), want(&[&edges[..], &["RAY (0.0, 0.0) (-1.0, 0.0)", "RAY (10.0, 0.0) (1.0, 0.0)"]].concat()));
}

#[test]
fn fillet_a_ray_or_an_xline_with_a_line() {
    let corner = "LINE (10.0, 0.0) (10.0, 12.0)";
    // The ray's base side kept: it becomes a line to the corner.
    assert_eq!(run("RAY 0,0 1,0", "FILLET R 0 5,0 10,5\n"), want(&["LINE (0.0, 0.0) (10.0, 0.0)", corner]));
    // Its infinite side kept: a ray from the corner.
    assert_eq!(run("RAY 0,0 1,0", "FILLET R 0 15,0 10,5\n"), want(&["RAY (10.0, 0.0) (1.0, 0.0)", corner]));
    // An xline becomes a ray running to the picked side, from the arc's tangent point.
    assert_eq!(run("XLINE H 0,0", "FILLET R 1 5,0 10,5\n"), want(&["ARC", "LINE (10.0, 1.0) (10.0, 12.0)", "RAY (9.0, 0.0) (-1.0, 0.0)"]));
    // Two xlines: two rays.
    let mut s = Session::new();
    s.script("XLINE H 0,0\n\nXLINE V 10,0\n\nFILLET R 0 5,0 10,5\n").unwrap();
    assert!(s.running.is_none(), "{}", s.prompt_text());
    assert_eq!(objects(&s), want(&["RAY (10.0, 0.0) (-1.0, 0.0)", "RAY (10.0, 0.0) (0.0, 1.0)"]));
}

#[test]
fn chamfer_a_ray_and_an_xline() {
    assert_eq!(
        run("RAY 0,0 1,0", "CHAMFER D 2 2 15,0 10,5\n"),
        want(&["LINE (10.0, 2.0) (10.0, 12.0)", "LINE (10.0, 2.0) (12.0, 0.0)", "RAY (12.0, 0.0) (1.0, 0.0)"])
    );
    assert_eq!(
        run("XLINE H 0,0", "CHAMFER D 2 2 5,0 10,5\n"),
        want(&["LINE (10.0, 2.0) (10.0, 12.0)", "LINE (8.0, 0.0) (10.0, 2.0)", "RAY (8.0, 0.0) (-1.0, 0.0)"])
    );
}

#[test]
fn extend_a_ray_moves_its_base_back_to_the_edge() {
    assert_eq!(run("RAY 15,0 16,0", "EXTEND 16,0\n\n"), want(&[EDGE, "RAY (10.0, 0.0) (1.0, 0.0)"]));
    // The edge lies ahead of the base, not behind it: the ray is left alone.
    let mut s = session("RAY 5,0 6,0");
    s.script("EXTEND 6,0\n\n").unwrap();
    assert_eq!(objects(&s), want(&[EDGE, "RAY (5.0, 0.0) (1.0, 0.0)"]));
}
