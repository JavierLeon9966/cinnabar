//! Offline face-attachment fixture: a slab block whose only per-face signal is
//! `minecraft:transformation`, as `owlscubed_pob:backrooms_wallpaper_slab`
//! authors it on the wire. It rasterizes the production selection outline
//! geometry for both the broken and fixed shapes, so a face change is visible
//! without a live server.

use image::{ExtendedColorType, ImageEncoder, Rgba, RgbaImage, codecs::png::PngEncoder};

use render::{
    BlockEntityVertex, BlockSelectionFrame, BlockSelectionTarget, CrackShape,
};

/// Authored half turns per `minecraft:block_face`, in the wire's palette order.
/// The default face is `up`, so it carries no turn of its own.
const FACES: [(&str, [i32; 3]); 6] = [
    ("down", [2, 0, 0]),
    ("up", [0, 0, 0]),
    ("north", [1, 2, 0]),
    ("south", [1, 0, 0]),
    ("west", [2, 1, 3]),
    ("east", [0, 1, 3]),
];

fn rotate_quarter([x, y, z]: [f32; 3], axis: usize) -> [f32; 3] {
    match axis {
        0 => [x, -z, y],
        1 => [z, y, -x],
        _ => [-y, x, z],
    }
}

/// Mirrors the overlay's block-centre transform, which the fix shares.
fn transform_point(value: [f32; 3], rotation: [i32; 3]) -> [f32; 3] {
    let mut point = value.map(|coordinate| coordinate * 16.0 - 8.0);
    for (axis, turns) in rotation.into_iter().enumerate() {
        for _ in 0..turns.rem_euclid(4) {
            point = rotate_quarter(point, axis);
        }
    }
    point.map(|coordinate| (coordinate + 8.0) / 16.0)
}

/// The base `up` slab: the top half of the block.
fn base_box() -> ([f32; 3], [f32; 3]) {
    ([0.0, 0.5, 0.0], [1.0, 1.0, 1.0])
}

/// The selection bounds for one face, before the fix and after it.
fn bounds_for(face: usize, fixed: bool) -> ([f32; 3], [f32; 3]) {
    let base = base_box();
    let Some(rotation) = fixed.then(|| FACES[face].1) else {
        return base;
    };
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for bits in 0..8_u32 {
        let corner = [
            if bits & 1 != 0 { base.1[0] } else { base.0[0] },
            if bits & 2 != 0 { base.1[1] } else { base.0[1] },
            if bits & 4 != 0 { base.1[2] } else { base.0[2] },
        ];
        let point = transform_point(corner, rotation);
        for axis in 0..3 {
            min[axis] = min[axis].min(point[axis]);
            max[axis] = max[axis].max(point[axis]);
        }
    }
    (min, max)
}

/// Isometric projection of a block-space point into pixel coordinates.
fn project(point: [f32; 3], origin: (f32, f32), scale: f32) -> (f32, f32) {
    let [x, y, z] = point;
    (
        origin.0 + (x - z) * scale,
        origin.1 + (x + z) * scale * 0.5 - y * scale,
    )
}

fn draw_line(image: &mut RgbaImage, from: (f32, f32), to: (f32, f32), color: Rgba<u8>) {
    let steps = (from.0 - to.0).abs().max((from.1 - to.1).abs()).ceil().max(1.0) as i32;
    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        let x = from.0 + (to.0 - from.0) * t;
        let y = from.1 + (to.1 - from.1) * t;
        if x < 0.0 || y < 0.0 || x >= image.width() as f32 || y >= image.height() as f32 {
            continue;
        }
        image.put_pixel(x as u32, y as u32, color);
    }
}

/// Draws one face's panel: the block cube in grey and its selection box in black.
fn draw_panel(
    image: &mut RgbaImage,
    bounds: ([f32; 3], [f32; 3]),
    origin: (f32, f32),
    scale: f32,
) {
    let grey = Rgba([150, 150, 150, 255]);
    let black = Rgba([0, 0, 0, 255]);
    // The unit cube as a wireframe so the panel reads as a block.
    for box_bounds in [([0.0; 3], [1.0; 3])] {
        let (box_min, box_max) = box_bounds;
        for bits in 0..8_u32 {
            for axis in 0..3 {
                if bits & (1 << axis) != 0 {
                    continue;
                }
                let from = [
                    if bits & 1 != 0 { box_max[0] } else { box_min[0] },
                    if bits & 2 != 0 { box_max[1] } else { box_min[1] },
                    if bits & 4 != 0 { box_max[2] } else { box_min[2] },
                ];
                let mut to = from;
                to[axis] = box_max[axis];
                draw_line(
                    image,
                    project(from, origin, scale),
                    project(to, origin, scale),
                    grey,
                );
            }
        }
    }
    // The production selection outline geometry.
    let target = BlockSelectionTarget {
        block: bounds.0.map(|value| value.floor() as i32),
        bounds: [bounds.0, bounds.1],
        shape: CrackShape::Cube,
    };
    let mut frame = BlockSelectionFrame::default();
    frame.update(Some(&target), true);
    assert_eq!(
        frame.outline.len(),
        12 * 2,
        "the production outline draws every box edge"
    );
    for pair in frame.outline.chunks_exact(2) {
        draw_line(
            image,
            project(vertex_point(&pair[0]), origin, scale),
            project(vertex_point(&pair[1]), origin, scale),
            black,
        );
    }
}

fn vertex_point(vertex: &BlockEntityVertex) -> [f32; 3] {
    vertex.position
}

fn render(fixed: bool) -> RgbaImage {
    const PANEL: u32 = 220;
    const COLS: u32 = 3;
    let mut image = RgbaImage::from_pixel(PANEL * COLS + 16, PANEL * 2 + 16, Rgba([255, 255, 255, 255]));
    for face in 0..FACES.len() {
        let column = u32::try_from(face % 3).unwrap();
        let row = u32::try_from(face / 3).unwrap();
        let origin = (
            40.0 + column as f32 * PANEL as f32 + 70.0,
            40.0 + row as f32 * PANEL as f32 + 20.0,
        );
        draw_panel(&mut image, bounds_for(face, fixed), origin, 90.0);
    }
    image
}

#[test]
fn face_attached_selection_outline_follows_the_authored_turn() {
    // The outline geometry must land on the authored face, not the default one.
    let expected: [(usize, [f32; 3], [f32; 3]); 6] = [
        (0, [0.0, 0.0, 0.0], [1.0, 0.5, 1.0]),
        (1, [0.0, 0.5, 0.0], [1.0, 1.0, 1.0]),
        (2, [0.0, 0.0, 0.0], [1.0, 1.0, 0.5]),
        (3, [0.0, 0.0, 0.5], [1.0, 1.0, 1.0]),
        (4, [0.0, 0.0, 0.0], [0.5, 1.0, 1.0]),
        (5, [0.5, 0.0, 0.0], [1.0, 1.0, 1.0]),
    ];
    for (face, min, max) in expected {
        for (axis, coordinate) in min.iter().enumerate() {
            assert!(
                bounds_for(face, true).0[axis] <= coordinate + 1e-6
                    && bounds_for(face, true).1[axis] >= max[axis] - 1e-6,
                "face {face} axis {axis}"
            );
        }
        assert_eq!(bounds_for(face, true), (min, max), "face {face}");
        // Before the fix every face kept the default `up` box.
        assert_eq!(bounds_for(face, false), base_box(), "face {face} before");
    }
}

/// Writes the before/after fixture panels when `FACE_FIXTURE_OUT` names a directory.
#[test]
fn write_face_attachment_fixture() {
    let Ok(directory) = std::env::var("FACE_FIXTURE_OUT") else {
        eprintln!("skipped: set FACE_FIXTURE_OUT to a directory to write the fixture PNGs");
        return;
    };
    let directory = std::path::Path::new(&directory);
    std::fs::create_dir_all(directory).unwrap();
    for (fixed, name) in [(false, "before"), (true, "after")] {
        let image = render(fixed);
        let mut png = Vec::new();
        PngEncoder::new(&mut png)
            .write_image(
                image.as_raw(),
                image.width(),
                image.height(),
                ExtendedColorType::Rgba8,
            )
            .unwrap();
        let path = directory.join(format!("face-attachment-{name}.png"));
        std::fs::write(&path, png).unwrap();
        eprintln!("wrote {}", path.display());
    }
}
