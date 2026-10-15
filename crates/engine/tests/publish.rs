//! PUBLISH with a `sheets` list: sheets from open drawings and from drawing files (read for the
//! run only), a page setup override per sheet that leaves the drawing as it was, file names from a
//! pattern, one result per sheet, and a run that keeps going when one sheet fails.

use std::sync::atomic::{AtomicUsize, Ordering};

use cadcraft_engine::Session;
use cadcraft_engine::cmd::file::{IoHooks, base64_decode, set_io};
use serde_json::{Value, json};

/// Files this test's reader decoded.
static READS: AtomicUsize = AtomicUsize::new(0);

/// A plotter that echoes what it was asked to plot.
fn echo_plot(_: &cadcraft_doc::Drawing, space: &cadcraft_doc::Space, opts: &Value) -> Result<Vec<u8>, String> {
    Ok(format!("%PDF-1.4 {space:?} {opts}").into_bytes())
}

/// A reader for `*.sheets` files (a metric drawing with Layout1 and Layout2); anything else is
/// unreadable.
fn read(bytes: &[u8], name: &str) -> Result<cadcraft_doc::Drawing, String> {
    if name.ends_with(".sheets") && bytes == b"sheets" {
        READS.fetch_add(1, Ordering::SeqCst);
        Ok(cadcraft_doc::Drawing::new_metric())
    } else {
        Err("not a drawing".into())
    }
}

fn session() -> Session {
    set_io(IoHooks { read, write: |_, _| Err("no writer in this test".into()), plot: Some(echo_plot) });
    let mut s = Session::new();
    s.execute("line", &json!({"points": [[0, 0], [100, 0]]})).unwrap();
    s
}

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("cadcraft-publish-{}-{name}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn plotted(sheet: &Value) -> String {
    String::from_utf8(base64_decode(sheet["data"].as_str().unwrap()).unwrap()).unwrap()
}

fn files(r: &Value) -> Vec<String> {
    r["sheets"].as_array().unwrap().iter().filter_map(|s| s["file"].as_str().map(str::to_string)).collect()
}

#[test]
fn sheets_are_named_from_the_pattern_and_written_to_the_folder() {
    let mut s = session();
    let dir = temp("named");
    let r = s
        .execute(
            "publish",
            &json!({"sheets": [{"layout": "layout1"}, {"layout": "MODEL"}], "folder": dir.to_string_lossy(), "name": "{n} {drawing} {sheet}"}),
        )
        .unwrap();
    assert_eq!(files(&r), ["1 Drawing1 Layout1.pdf", "2 Drawing1 Model.pdf"]);
    assert_eq!((r["published"].as_u64(), r["failed"].as_u64()), (Some(2), Some(0)));
    for sheet in r["sheets"].as_array().unwrap() {
        let path = sheet["path"].as_str().unwrap();
        assert!(std::fs::read_to_string(path).unwrap().starts_with("%PDF"), "{path} was written");
    }
    // SVG and PNG take their own extension; without a folder each sheet comes back as data.
    let r = s.execute("publish", &json!({"sheets": [{"layout": "Layout2"}], "to": "SVG"})).unwrap();
    assert_eq!(files(&r), ["Drawing1-Layout2.svg"]);
    assert!(plotted(&r["sheets"][0]).contains(r#""output":"svg""#));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_failing_sheet_is_reported_and_the_rest_still_publish() {
    let mut s = session();
    let bad_file = temp("unreadable").join("broken.dxf");
    std::fs::write(&bad_file, b"not a drawing").unwrap();
    let r = s
        .execute(
            "publish",
            &json!({"sheets": [
                {"layout": "Layout1"},
                {"layout": "Nope"},
                {"drawing": 7, "layout": "Layout1"},
                {"drawing": bad_file.to_string_lossy(), "layout": "Layout1"},
                {"drawing": "/no/such/folder/plan.sheets", "layout": "Layout1"},
                {"layout": "Model"},
            ]}),
        )
        .unwrap();
    let ok: Vec<bool> = r["sheets"].as_array().unwrap().iter().map(|s| s["ok"].as_bool().unwrap()).collect();
    assert_eq!(ok, [true, false, false, false, false, true]);
    assert_eq!((r["published"].as_u64(), r["failed"].as_u64()), (Some(2), Some(4)));
    assert!(r["sheets"][1]["error"].as_str().unwrap().contains("no layout `Nope`"));
    assert!(r["sheets"][2]["error"].as_str().unwrap().contains("no open drawing 7"));
    assert!(r["sheets"][3]["error"].as_str().unwrap().contains("not a drawing"));
    assert!(r["message"].as_str().unwrap().contains("4 failed"));
    // When no sheet publishes, the command fails and says why.
    let e = s.execute("publish", &json!({"sheets": [{"layout": "Nope"}, {"layout": "Gone"}]})).unwrap_err().to_string();
    assert!(e.contains("no sheet was published") && e.contains("Nope") && e.contains("Gone"), "{e}");
    let _ = std::fs::remove_file(bad_file);
}

#[test]
fn a_page_setup_override_is_for_this_run_only() {
    let mut s = session();
    let before = s.doc().unwrap().layout("Layout1").unwrap().page.clone();
    let (revision, undo) = (s.state().unwrap().revision, s.state().unwrap().undo.len());
    let r = s
        .execute(
            "publish",
            &json!({"sheets": [{"layout": "Layout1", "pageSetup": {"paper": "A3", "landscape": true, "scale": "1:50", "margins": [5, 5, 5, 5]}}]}),
        )
        .unwrap();
    let opts = plotted(&r["sheets"][0]);
    for part in [r#""paper":"A3""#, r#""landscape":true"#, r#""scale":0.02"#, r#""margins":[5.0,5.0,5.0,5.0]"#] {
        assert!(opts.contains(part), "the plotter gets {part}: {opts}");
    }
    assert_eq!(s.doc().unwrap().layout("Layout1").unwrap().page, before, "the layout's page setup is unchanged");
    assert_eq!(s.state().unwrap().revision, revision, "the drawing isn't edited");
    assert_eq!(s.state().unwrap().undo.len(), undo, "no undo step is added");
}

#[test]
fn sheets_come_from_open_drawings_and_files_without_opening_them() {
    let mut s = session();
    s.execute("new", &json!({})).unwrap();
    assert_eq!((s.docs.len(), s.active), (2, 1));
    let file = temp("files").join("site plan.sheets");
    std::fs::write(&file, b"sheets").unwrap();
    let path = file.to_string_lossy().to_string();
    let reads = READS.load(Ordering::SeqCst);
    let r = s
        .execute(
            "publish",
            &json!({"sheets": [
                {"drawing": 0, "layout": "Model"},
                {"drawing": path, "layout": "Layout1"},
                {"drawing": path, "layout": "Layout2"},
            ]}),
        )
        .unwrap();
    assert_eq!(files(&r), ["Drawing1-Model.pdf", "site plan-Layout1.pdf", "site plan-Layout2.pdf"]);
    assert_eq!(r["sheets"][1]["drawing"], path);
    assert_eq!(READS.load(Ordering::SeqCst) - reads, 1, "a file is read once per run");
    assert_eq!((s.docs.len(), s.active), (2, 1), "no drawing is opened and the current one stays current");
    let _ = std::fs::remove_file(file);
}

#[test]
fn two_sheets_with_one_name_do_not_overwrite_each_other() {
    let mut s = session();
    let dir = temp("same");
    let r = s.execute("publish", &json!({"sheets": [{"layout": "Layout1"}, {"layout": "Layout1"}], "folder": dir.to_string_lossy()})).unwrap();
    assert_eq!(files(&r), ["Drawing1-Layout1.pdf", "Drawing1-Layout1 (2).pdf"]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_older_path_and_layouts_form_still_works() {
    let mut s = session();
    let dir = temp("legacy");
    let base = dir.join("set{1}.pdf");
    let r = s.execute("publish", &json!({"path": base.to_string_lossy(), "layouts": ["layout2", "MODEL"]})).unwrap();
    assert_eq!(files(&r), ["set{1}-Layout2.pdf", "set{1}-Model.pdf"], "braces in the old NAME stay braces");
    assert!(r["sheets"][0]["path"].as_str().unwrap().ends_with("set{1}-Layout2.pdf"));
    // Unknown layouts are still refused before anything is written.
    assert!(s.execute("publish", &json!({"layouts": ["Nope"]})).is_err());
    // No parameters: Model (it has objects), then every layout.
    let r = s.execute("publish", &json!({})).unwrap();
    let layouts: Vec<&str> = r["sheets"].as_array().unwrap().iter().filter_map(|x| x["layout"].as_str()).collect();
    assert_eq!(layouts, ["Model", "Layout1", "Layout2"]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn bad_parameters_are_refused_before_anything_is_written() {
    let mut s = session();
    let many: Vec<Value> = (0..257).map(|_| json!({"layout": "Layout1"})).collect();
    let cases = [
        json!({"sheets": [{"layout": "Layout1"}], "widht": 5}),
        json!({"sheets": [{"layout": "Layout1"}], "path": "x.pdf"}),
        json!({"sheets": "Layout1"}),
        json!({"sheets": []}),
        json!({"sheets": many}),
        json!({"sheets": [{"layout": "Layout1", "colour": 1}]}),
        json!({"sheets": [{"drawing": -1, "layout": "Layout1"}]}),
        json!({"sheets": [{"drawing": 1.5, "layout": "Layout1"}]}),
        json!({"sheets": [{"drawing": ""}]}),
        json!({"sheets": [{}]}),
        json!({"sheets": [{"layout": "Layout1", "pageSetup": {"paper": "Z9"}}]}),
        json!({"sheets": [{"layout": "Layout1", "pageSetup": {"width": 100}}]}),
        json!({"sheets": [{"layout": "Layout1", "pageSetup": {"width": 1e308, "height": 5}}]}),
        json!({"sheets": [{"layout": "Layout1", "pageSetup": {"margins": [1, 2, 3]}}]}),
        json!({"sheets": [{"layout": "Layout1", "pageSetup": {"scale": -1}}]}),
        json!({"sheets": [{"layout": "Layout1", "pageSetup": {"landscape": "yes"}}]}),
        json!({"sheets": [{"layout": "Layout1", "pageSetup": {"plotArea": "display"}}]}),
        json!({"sheets": [{"layout": "Layout1"}], "to": "dwg"}),
        json!({"sheets": [{"layout": "Layout1"}], "to": 3}),
        json!({"sheets": [{"layout": "Layout1"}], "name": "../{sheet}"}),
        json!({"sheets": [{"layout": "Layout1"}], "name": "{foo}"}),
        json!({"sheets": [{"layout": "Layout1"}], "name": "a}"}),
        json!({"sheets": [{"layout": "Layout1"}], "name": ".."}),
        json!({"sheets": [{"layout": "Layout1"}], "name": ""}),
        json!({"sheets": [{"layout": "Layout1"}], "folder": 4}),
        json!({"sheets": [{"layout": "Layout1"}], "pdf": {"layers": "on"}}),
        json!({"sheets": [{"layout": "Layout1"}], "pdf": {"text": "outline"}}),
        json!({"sheets": [{"layout": "Layout1"}], "pdf": {"dpi": 600}}),
        json!({"sheets": [{"layout": "Layout1"}], "copies": 0}),
        json!({"sheets": [{"layout": "Layout1"}], "copies": 101}),
        json!({"sheets": [{"layout": "Layout1"}], "stamp": true}),
        json!([1, 2]),
    ];
    for p in cases {
        assert!(s.execute("publish", &p).is_err(), "refused: {p}");
    }
    // Accepted: copies, PDF settings and an escaped brace.
    let r = s
        .execute(
            "publish",
            &json!({"sheets": [{"layout": "Layout1"}], "copies": 3, "pdf": {"layers": false, "text": "geometry"}, "name": "{{{sheet}}}"}),
        )
        .unwrap();
    assert_eq!(r["copies"], 3);
    assert_eq!(files(&r), ["{Layout1}.pdf"]);
    let opts = plotted(&r["sheets"][0]);
    assert!(opts.contains(r#""layers":false"#) && opts.contains(r#""text":"geometry""#), "{opts}");
}
