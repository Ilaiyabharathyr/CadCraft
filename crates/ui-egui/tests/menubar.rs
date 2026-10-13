//! The in-window menu bar and its popups: measured from what the real app paints.

use cadcraft_engine::Session;
use cadcraft_ui_egui::theme::Tokens;
use cadcraft_ui_egui::{CadApp, Services, menus};
use egui::{Rect, Shape, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

/// The height of a menu row and of a button of the bar, which tell them from the toolbar buttons
/// and panel headings that carry the same names.
const ROW: f32 = 26.0;
const BAR_BUTTON: f32 = 24.0;

fn harness(scale: f32) -> Harness<'static, CadApp> {
    let mut app = CadApp::new(Session::default(), Services::default());
    app.run("new", serde_json::json!({})).unwrap();
    app.ui.start_tab = false;
    app.ui.in_window_menu = true;
    app.integrated_titlebar = true;
    let mut h = Harness::builder().with_size(vec2(1200.0, 760.0)).with_pixels_per_point(scale).build_ui_state(
        |ui, app: &mut CadApp| {
            app.logic(&ui.ctx().clone());
            app.ui(ui);
        },
        app,
    );
    h.run_steps(5);
    h
}

/// Save what the harness shows when `CRAFT_UI_VISUAL_DIR` is set (for looking at the result).
fn snapshot(h: &mut Harness<'static, CadApp>, name: &str) {
    if let Ok(directory) = std::env::var("CRAFT_UI_VISUAL_DIR") {
        std::fs::create_dir_all(&directory).unwrap();
        h.render().unwrap().save(format!("{directory}/menubar-{name}.png")).unwrap();
    }
}

/// The rectangle of the widget named `label` that is `height` tall.
fn find(h: &Harness<'static, CadApp>, label: &str, height: f32) -> Option<Rect> {
    h.query_all_by_label(label).map(|n| n.rect()).find(|r| (r.height() - height).abs() < 0.5)
}

fn bar_button(h: &Harness<'static, CadApp>, menu: &str) -> Rect {
    find(h, menu, BAR_BUTTON).unwrap_or_else(|| panic!("no {menu} button in the menu bar"))
}

fn row(h: &Harness<'static, CadApp>, label: &str) -> Rect {
    find(h, label, ROW).unwrap_or_else(|| panic!("no {label} row in an open menu"))
}

fn click(h: &mut Harness<'static, CadApp>, rect: Rect) {
    h.hover_at(rect.center());
    h.run_steps(2);
    h.drag_at(rect.center());
    h.drop_at(rect.center());
    h.run_steps(4);
}

fn open(h: &mut Harness<'static, CadApp>, menu: &str) {
    let button = bar_button(h, menu);
    click(h, button);
}

/// Everything painted, in painting order.
fn painted(h: &Harness<'static, CadApp>) -> Vec<Shape> {
    clipped(h).into_iter().map(|(_, shape)| shape).collect()
}

/// Everything painted with the rectangle it is clipped to.
fn clipped(h: &Harness<'static, CadApp>) -> Vec<(Rect, Shape)> {
    fn flatten(clip: Rect, shape: &Shape, out: &mut Vec<(Rect, Shape)>) {
        match shape {
            Shape::Vec(shapes) => shapes.iter().for_each(|s| flatten(clip, s, out)),
            shape => out.push((clip, shape.clone())),
        }
    }
    let mut out = Vec::new();
    for clipped in &h.output().shapes {
        flatten(clipped.clip_rect, &clipped.shape, &mut out);
    }
    out
}

/// The text that shows, with the part of it that is visible.
fn texts(shapes: &[(Rect, Shape)]) -> Vec<(String, Rect)> {
    shapes
        .iter()
        .filter_map(|(clip, s)| match s {
            Shape::Text(t) => Some((t.galley.text().to_string(), t.visual_bounding_rect().intersect(*clip))).filter(|(_, r)| r.is_positive()),
            _ => None,
        })
        .collect()
}

fn overlaps(texts: &[(String, Rect)]) -> Vec<String> {
    let mut out = Vec::new();
    for (i, (first, a)) in texts.iter().enumerate() {
        for (second, b) in texts.iter().skip(i + 1) {
            if a.intersects(*b) {
                out.push(format!("{first:?} {a:?} overlaps {second:?} {b:?}"));
            }
        }
    }
    out
}

/// The bar: the panel filled with `list_bg` that holds the File button.
fn bar_rect(h: &Harness<'static, CadApp>) -> Rect {
    let file = bar_button(h, "File");
    let fill = Tokens::get().list_bg;
    painted(h)
        .iter()
        .find_map(|s| match s {
            Shape::Rect(r) if r.fill == fill && r.rect.contains_rect(file) => Some(r.rect),
            _ => None,
        })
        .expect("the menu bar paints its background")
}

#[test]
fn menubar_is_30_high() {
    let mut h = harness(1.0);
    snapshot(&mut h, "bar");
    let bar = bar_rect(&h);
    // The bar takes 30 pt of the window: its fill and the 1 pt rule egui draws under a panel.
    let below = painted(&h)
        .iter()
        .filter_map(|s| match s {
            Shape::Rect(r) if r.rect.width() == bar.width() && r.rect.top() > bar.top() => Some(r.rect.top()),
            _ => None,
        })
        .fold(f32::INFINITY, f32::min);
    assert_eq!(below - bar.top(), 30.0, "{bar:?}");
    assert_eq!(bar.height(), 29.0);
    assert_eq!(cadcraft_ui_egui::menus::BAR_HEIGHT, 30.0);
    for menu in menus::MENUS {
        let button = bar_button(&h, menu);
        assert!((button.center().y - bar.center().y).abs() < 0.5, "{menu} is not centred in the bar: {button:?} in {bar:?}");
    }
}

#[test]
fn file_menu_groups_with_separators() {
    let mut h = harness(1.0);
    open(&mut h, "File");
    snapshot(&mut h, "file");
    let (new, template, open_row, close) = (row(&h, "New Drawing..."), row(&h, "New from Template..."), row(&h, "Open..."), row(&h, "Close"));
    // Rows of one group touch; a separator takes 9 pt and paints its line in the `list_row` colour.
    assert!((template.top() - new.bottom()).abs() < 0.1, "{new:?} {template:?}");
    assert!((close.top() - open_row.bottom() - 9.0).abs() < 0.1, "{open_row:?} {close:?}");
    let colour = Tokens::get().list_row;
    let line = painted(&h).iter().any(|s| match s {
        Shape::LineSegment { points, stroke } => {
            stroke.color == colour && points.iter().all(|p| p.y > open_row.bottom() && p.y < close.top() && open_row.x_range().contains(p.x))
        }
        _ => false,
    });
    assert!(line, "no separator line between Open and Close");
}

#[test]
fn submenu_aligns_with_parent() {
    let mut h = harness(1.0);
    open(&mut h, "File");
    let parent = row(&h, "Default Save Format");
    h.hover_at(parent.center());
    h.run_steps(4);
    snapshot(&mut h, "file-saveformat");
    let first = row(&h, "DXF");
    assert!((first.top() - parent.top()).abs() <= 1.0, "the submenu's first row {first:?} is not level with {parent:?}");
    // The submenu overlaps its parent by 4 pt: both popups pad their rows by 5 pt (margin and border).
    assert!((first.left() - (parent.right() + 6.0)).abs() <= 1.0, "the submenu {first:?} does not overlap {parent:?} by 4 pt");
    assert!(first.width() >= 290.0, "{first:?}");
    assert!(parent.width() >= 306.0, "{parent:?}");
}

/// The fill of the dot painted in `row`, or `None` for a ring.
fn radio_fill(h: &Harness<'static, CadApp>, row: Rect) -> Option<egui::Color32> {
    painted(h).iter().rev().find_map(|s| match s {
        Shape::Circle(c) if row.contains(c.center) => Some((c.fill != egui::Color32::TRANSPARENT).then_some(c.fill)),
        _ => None,
    })?
}

#[test]
fn radio_reflects_save_format() {
    let mut h = harness(1.0);
    let show = |h: &mut Harness<'static, CadApp>| {
        open(h, "File");
        let parent = row(h, "Default Save Format");
        h.hover_at(parent.center());
        h.run_steps(4);
        // Rest on a row without a mark: a highlighted row repaints its mark in another colour.
        let rest = row(h, "Version 2000");
        h.hover_at(rest.center());
        h.run_steps(2);
    };
    show(&mut h);
    let accent = Tokens::get().accent;
    assert_eq!(radio_fill(&h, row(&h, "DXF")), Some(accent));
    assert_eq!(radio_fill(&h, row(&h, "DWG")), None);

    let dwg = row(&h, "DWG");
    click(&mut h, dwg);
    assert_eq!(h.state().ui.save_format.as_str(), "dwg");
    assert!(find(&h, "DWG", ROW).is_none(), "choosing a row closes the menu");
    show(&mut h);
    snapshot(&mut h, "file-saveformat-dwg");
    assert_eq!(radio_fill(&h, row(&h, "DWG")), Some(accent));
    assert_eq!(radio_fill(&h, row(&h, "DXF")), None);
}

#[test]
fn disabled_rows_do_not_fire() {
    let mut h = harness(1.0);
    open(&mut h, "Edit");
    snapshot(&mut h, "edit");
    // Nothing is selected in the new drawing, so Clear (ERASE) is unavailable.
    let clear = row(&h, "Clear");
    let before = h.state().session.log.len();
    click(&mut h, clear);
    assert!(h.state().session.running.is_none());
    assert_eq!(h.state().session.log.len(), before);
    assert!(find(&h, "Select All", ROW).is_some(), "a disabled row leaves the menu open");
    let fill = Tokens::get().accent;
    let highlighted = painted(&h).iter().any(|s| matches!(s, Shape::Rect(r) if r.fill == fill && r.rect == clear));
    assert!(!highlighted, "a disabled row is not highlighted");

    // The row above it works: Paste commands aside, Select All runs and closes the menu.
    let select_all = row(&h, "Select All");
    click(&mut h, select_all);
    assert!(find(&h, "Select All", ROW).is_none());
    assert!(h.state().session.log.len() > before, "{:?}", h.state().session.log);
}

#[test]
fn menubar_has_no_overlaps() {
    let mut problems = Vec::new();
    for scale in [1.0, 2.0] {
        let h = harness(scale);
        let bar = bar_rect(&h);
        let in_bar: Vec<_> = texts(&clipped(&h)).into_iter().filter(|(_, r)| bar.contains(r.center())).collect();
        assert!(in_bar.len() > menus::MENUS.len(), "the bar shows the menu names and the drawing name: {in_bar:?}");
        problems.extend(overlaps(&in_bar).into_iter().map(|p| format!("bar @{scale}: {p}")));
        for (i, menu) in menus::MENUS.iter().enumerate() {
            let button = bar_button(&h, menu);
            assert!(bar.contains_rect(button), "{menu} @{scale}: {button:?} leaves the bar {bar:?}");
            if let Some(next) = menus::MENUS.get(i + 1) {
                assert!(button.right() <= bar_button(&h, next).left(), "{menu} overlaps {next} @{scale}");
            }
        }

        for menu in menus::MENUS {
            let mut h = harness(scale);
            open(&mut h, menu);
            let shapes = clipped(&h);
            let fill = Tokens::get().menu_bg;
            let (at, popup) = shapes
                .iter()
                .enumerate()
                .rev()
                .find_map(|(i, (_, s))| match s {
                    Shape::Rect(r) if r.fill == fill => Some((i, r.rect)),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("{menu} @{scale} did not open"));
            assert!(h.ctx.content_rect().contains_rect(popup), "{menu} @{scale}: the popup {popup:?} leaves the window");
            assert!(popup.width() >= 316.0, "{menu} @{scale}: {popup:?}");
            let rows = texts(shapes.get(at..).unwrap_or_default());
            assert!(!rows.is_empty(), "{menu} @{scale} is empty");
            for (text, rect) in &rows {
                assert!(popup.contains_rect(*rect), "{menu} @{scale}: {text:?} {rect:?} leaves the popup {popup:?}");
            }
            problems.extend(overlaps(&rows).into_iter().map(|p| format!("{menu} @{scale}: {p}")));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
