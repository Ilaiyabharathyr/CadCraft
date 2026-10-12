//! Standard SPATIAL_FILTER data must survive save, reopen, and removal.

use std::sync::Arc;

use cadcraft_doc::{Block, ClipBoundary, Common, Drawing, Entity, EntityKind, Insert, Line, Space};
use cadcraft_geom::{Vec2, Vec3};
use cadcraft_io::{read, read_dxf, write, write_dxf};

fn drawing() -> Drawing {
    let mut d = Drawing::new_metric();
    let mut block = Block::new("Plan");
    block.entities.push(Entity::new(d.new_handle(), EntityKind::Line(Line { a: Vec3::new(-10.0, 0.0, 0.0), b: Vec3::new(10.0, 0.0, 0.0) })));
    d.blocks.insert(block.name.clone(), Arc::new(block));
    d.add(
        &Space::Model,
        Common::default(),
        EntityKind::Insert(Insert {
            block: "Plan".into(),
            insert: Vec3::new(10.0, 20.0, 0.0),
            scale: Vec3::new(2.0, 1.0, 1.0),
            rotation: 0.3,
            attribs: vec![],
            cols: 1,
            rows: 1,
            col_spacing: 0.0,
            row_spacing: 0.0,
            clip: Some(ClipBoundary { points: vec![Vec2::new(-2.0, -1.0), Vec2::new(2.0, 1.0)], enabled: true, inverted: false, deleted: false }),
        }),
    )
    .unwrap();
    d
}

fn clip(d: &Drawing) -> Option<&ClipBoundary> {
    match &d.model.iter().find(|e| matches!(e.kind, EntityKind::Insert(_)))?.kind {
        EntityKind::Insert(i) => i.clip.as_ref(),
        _ => None,
    }
}

#[test]
fn spatial_filter_round_trips_without_duplication() {
    let d = drawing();
    let text = write_dxf(&d);
    assert!(text.contains("ACAD_FILTER"), "missing extension dictionary");
    assert!(text.contains("SPATIAL_FILTER"), "missing spatial filter object");
    let back = read_dxf(text.as_bytes()).unwrap();
    assert_eq!(clip(&back), clip(&d));
    let again = write_dxf(&back);
    assert_eq!(again.matches("SPATIAL_FILTER").count(), 1, "duplicate filter");
    let twice = read_dxf(again.as_bytes()).unwrap();
    assert_eq!(clip(&twice), clip(&d));
}

#[test]
fn off_and_delete_survive_reopen() {
    let mut d = read_dxf(write_dxf(&drawing()).as_bytes()).unwrap();
    let h = d.model.iter().find(|e| matches!(e.kind, EntityKind::Insert(_))).unwrap().handle;
    d.modify_entity(h, |e| {
        if let EntityKind::Insert(i) = &mut e.kind {
            i.clip.as_mut().unwrap().enabled = false;
        }
    })
    .unwrap();
    let off = read_dxf(write_dxf(&d).as_bytes()).unwrap();
    assert_eq!(clip(&off).map(|c| c.enabled), Some(false));

    d.modify_entity(h, |e| {
        if let EntityKind::Insert(i) = &mut e.kind {
            i.clip = Some(ClipBoundary { points: vec![], enabled: false, inverted: false, deleted: true });
        }
    })
    .unwrap();
    let deleted = write_dxf(&d);
    assert!(!deleted.contains("SPATIAL_FILTER"), "old filter remained in DXF");
    assert!(clip(&read_dxf(deleted.as_bytes()).unwrap()).is_none());
}

#[test]
fn spatial_filter_survives_dwg_bridge() {
    let d = drawing();
    let bytes = write(&d, "x.dwg").unwrap();
    let back = read(&bytes, "x.dwg").unwrap();
    assert_eq!(clip(&back), clip(&d));
}
