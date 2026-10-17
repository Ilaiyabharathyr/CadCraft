//! One failing line costs that line, not the drawing (#1522): the rest of the script still runs,
//! the drawing is still written, and the exit status still reports the failure.

use std::process::Command;

#[test]
fn a_bad_line_is_reported_and_the_rest_still_runs() {
    let dir = std::env::temp_dir().join(format!("cc-cli-scriptfail-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let script = dir.join("bad.scr");
    std::fs::write(&script, "CIRCLE 5,5 2\nNOT_A_COMMAND\nLINE 0,0 10,0\n").unwrap();
    let drawing = dir.join("out.dxf");
    let out = Command::new(env!("CARGO_BIN_EXE_cadcraft-cli"))
        .args(["run", "--metric", "--script-file", script.to_str().unwrap(), "--save", drawing.to_str().unwrap()])
        .output()
        .expect("run cadcraft-cli");
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "the failure is still reported: {err}");
    assert!(err.contains("script line 2: unknown command `NOT_A_COMMAND`"), "{err}");
    let saved = std::fs::read_to_string(&drawing).expect("the drawing is written even though a line failed");
    assert!(saved.contains("CIRCLE") && saved.contains("LINE"), "both good commands ran: {}", &saved[..saved.len().min(200)]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_fully_good_script_still_exits_zero() {
    let out =
        Command::new(env!("CARGO_BIN_EXE_cadcraft-cli")).args(["run", "--metric", "--script", "CIRCLE 5,5 2\\n"]).output().expect("run cadcraft-cli");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
}
