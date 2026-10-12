//! vendor/winit patch (6); provenance: `vendor/patches/winit/06-macos-ime-commit.md`.

#[test]
fn winit_commits_macos_compositions_that_end_without_marked_text() {
    // winit 0.30.13 treats `insertText` as an IME commit only while marked text is left. Input
    // methods that end a composition with `unmarkText` first lost the committed text, and Apple
    // Korean, which clears its marked text before inserting the key typed after a syllable (`한`
    // then `5`), lost that key. vendor/winit commits when a composition was started
    // (`pending_commit`, rust-windowing/winit#4650), and a commit outside a key press no longer
    // swallows the next key (#4748). A re-vendored copy without the patch would silently bring the
    // bug back: re-apply the patch, or drop the copy once a winit 0.30 release has the fix.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let view = std::fs::read_to_string(root.join("vendor/winit/src/platform_impl/macos/view.rs")).expect("vendored winit macOS view source");

    assert!(view.contains("pending_commit: Cell<bool>"), "composition commit state is missing");
    assert!(view.contains("self.ivars().pending_commit.set(true);"), "setMarkedText no longer records a pending composition commit");
    assert!(
        view.contains("if pending_commit && ime_enabled && !is_control {"),
        "insertText no longer commits only pending, enabled, non-control text"
    );
    assert!(view.contains("self.ivars().in_key_down.get()"), "the commit path does not distinguish keyDown from external insertion");
    assert!(view.contains("self.ivars().pending_commit.set(false);"), "consumed or disabled composition state is not cleared");
    assert!(
        view.contains("self.ivars().in_key_down.set(true);\n                unsafe { self.interpretKeyEvents(&events_for_nsview) };\n                self.ivars().in_key_down.set(false);"),
        "keyDown does not bound the in_key_down state around interpretKeyEvents"
    );
    assert!(
        view.contains("NSString::from_str(\"NSUnderline\")") && view.contains("NSString::from_str(\"NSMarkedClauseSegment\")"),
        "validAttributesForMarkedText does not expose the reviewed attributes"
    );
}
