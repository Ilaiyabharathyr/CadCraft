#!/usr/bin/env python3
"""Apply Android hover support to an isolated copy of the locked, published winit crate.

The desktop dependency and the checkout's Cargo.lock stay unchanged. Android packaging uses
the copied workspace and crate, with the same dependency versions apart from winit's source.
Remove this bridge when the winit version used by eframe emits Android hover events itself.
"""

import json
import pathlib
import shutil
import subprocess
import sys
import tomllib

repo = pathlib.Path(__file__).resolve().parents[2]
stage = pathlib.Path(sys.argv[1]).resolve()
metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--locked", "--format-version", "1"], cwd=repo))
package = next(p for p in metadata["packages"] if p["name"] == "winit" and p["version"] == "0.30.13")
source = pathlib.Path(package["manifest_path"]).parent
patched = stage / "winit-hover"
workspace = stage / "workspace"
if patched.exists():
    shutil.rmtree(patched)
shutil.copytree(source, patched)
module = patched / "src/platform_impl/android/mod.rs"
text = module.read_text()
marker = "                let phase = match motion_event.action() {"
if text.count(marker) != 1:
    raise SystemExit("The locked winit input code changed; review the Android hover bridge.")
hover = """                // Android pens report hover separately from touch. Emit pointer movement
                // without synthesizing a button press, so hovering cannot draft a point.
                match motion_event.action() {
                    MotionAction::HoverEnter | MotionAction::HoverMove => {
                        if let Some(pointer) = motion_event.pointers().next() {
                            let (x, y) = (pointer.x(), pointer.y());
                            if x.is_finite() && y.is_finite() {
                                if motion_event.action() == MotionAction::HoverEnter {
                                    callback(event::Event::WindowEvent {
                                        window_id,
                                        event: event::WindowEvent::CursorEntered { device_id },
                                    }, self.window_target());
                                }
                                callback(event::Event::WindowEvent {
                                    window_id,
                                    event: event::WindowEvent::CursorMoved {
                                        device_id,
                                        position: PhysicalPosition { x: x as f64, y: y as f64 },
                                    },
                                }, self.window_target());
                            }
                        }
                        return input_status;
                    },
                    MotionAction::HoverExit => {
                        callback(event::Event::WindowEvent {
                            window_id,
                            event: event::WindowEvent::CursorLeft { device_id },
                        }, self.window_target());
                        return input_status;
                    },
                    _ => {},
                }

"""
module.write_text(text.replace(marker, hover + marker))
paths = subprocess.check_output(["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"], cwd=repo).decode().split("\0")
workspace.mkdir(parents=True, exist_ok=True)
for name in paths:
    if not name or name.startswith(("dist/", "target/", "plan/")):
        continue
    src = repo / name
    if src.is_file():
        dest = workspace / name
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, dest)
config = workspace / ".cargo/config.toml"
config.parent.mkdir(parents=True, exist_ok=True)
original = (repo / ".cargo/config.toml").read_text() if (repo / ".cargo/config.toml").exists() else ""
if "[patch.crates-io]" in original:
    raise SystemExit("Review the existing Cargo patch before applying the Android hover bridge.")
config.write_text(original + '\n[patch.crates-io]\nwinit = { path = ' + json.dumps(str(patched)) + ' }\n')
subprocess.run(["cargo", "metadata", "--offline", "--format-version", "1"], cwd=workspace, check=True, stdout=subprocess.DEVNULL)
before = tomllib.loads((repo / "Cargo.lock").read_text())
after = tomllib.loads((workspace / "Cargo.lock").read_text())
normalize = lambda lock: [p for p in lock["package"] if p["name"] != "winit"]
if normalize(before) != normalize(after):
    raise SystemExit("Android hover patch changed another dependency; refusing an unpinned build.")
print(workspace)
