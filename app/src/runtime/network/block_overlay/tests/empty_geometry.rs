use super::*;

/// A cubeless geometry draws nothing: packs pair it with a null texture to
/// hide a state, and the client must not paint it diagnostic purple.
#[test]
fn empty_geometry_is_invisible_without_gaps() {
    let view = view_with_geometry(
        br#"{"format_version":"1.12.0","minecraft:geometry":[{
        "description":{"identifier":"geometry.empty","texture_width":16,"texture_height":16}
    }]}"#,
    );
    let hidden = block(
        "test:hidden",
        1,
        CustomBlockVisuals {
            base: CustomVisualComponents {
                geometry: Some("geometry.empty".into()),
                materials: materials("lucky"),
                ..CustomVisualComponents::default()
            },
            ..CustomBlockVisuals::default()
        },
    );
    let blocks = CustomBlocks {
        blocks: vec![hidden].into(),
        ..CustomBlocks::default()
    };
    for hashed in [false, true] {
        let compiled = compile_block_overlay(&view, &blocks, hashed, None).expect("overlay");
        assert_eq!(
            compiled.gaps,
            OverlayGaps::default(),
            "hiding a state counts no gap"
        );
        assert_eq!(compiled.overlay.visuals.len(), 1);
        assert_eq!(
            compiled.overlay.visuals[0].kind,
            VisualKind::Invisible,
            "a cubeless geometry draws nothing"
        );
        RuntimeAssets::diagnostic()
            .with_block_overlay(1, &compiled.overlay)
            .expect("invisible visuals apply");
    }
}
