use super::*;

/// Failed faces keep the diagnostic material while their cube still renders.
#[test]
fn cube_keeps_rendering_with_a_failed_face() {
    let instances =
        [("*", "lucky"), ("up", "absent")].map(|(name, texture)| CustomMaterialInstance {
            name: name.into(),
            texture: texture.into(),
            render_method: None,
            tint_method: None,
            ambient_occlusion: None,
            face_dimming: None,
        });
    let chair = block(
        "test:chair",
        1,
        CustomBlockVisuals {
            base: CustomVisualComponents {
                geometry: Some("minecraft:geometry.full_block".into()),
                materials: Some(instances.into()),
                ..CustomVisualComponents::default()
            },
            ..CustomBlockVisuals::default()
        },
    );
    let blocks = CustomBlocks {
        blocks: vec![chair].into(),
        ..CustomBlocks::default()
    };
    let compiled = compile_block_overlay(&view(), &blocks, false, None).expect("overlay");
    assert_eq!(compiled.gaps.missing_textures, 1);
    assert_eq!(compiled.overlay.visuals.len(), 1);
    let visual = compiled.overlay.visuals[0];
    assert_eq!(visual.kind, VisualKind::Cube);
    assert_eq!(
        compiled.overlay.materials[visual.faces[BlockFace::Up as usize] as usize],
        compiled.overlay.materials[assets::DIAGNOSTIC_MATERIAL as usize],
        "only the failed top face degrades"
    );
    assert!(
        compiled.overlay.materials[visual.faces[BlockFace::North as usize] as usize]
            .texture
            .page()
            == 1,
        "healthy faces keep their overlay raster"
    );
    let session = RuntimeAssets::diagnostic()
        .with_block_overlay(1, &compiled.overlay)
        .expect("partial cubes apply");
    assert_eq!(
        session.resolve(NetworkIdMode::Sequential, 1).kind(),
        VisualKind::Cube
    );
}

/// A model whose every texture fails still emits its mesh with diagnostic
/// quads instead of collapsing to the diagnostic look.
#[test]
fn model_keeps_quads_with_failed_textures() {
    let statue = block(
        "test:statue",
        1,
        CustomBlockVisuals {
            base: CustomVisualComponents {
                geometry: Some("geometry.gen".into()),
                materials: materials("absent"),
                ..CustomVisualComponents::default()
            },
            ..CustomBlockVisuals::default()
        },
    );
    let blocks = CustomBlocks {
        blocks: vec![statue].into(),
        ..CustomBlocks::default()
    };
    let compiled = compile_block_overlay(&view(), &blocks, false, None).expect("overlay");
    assert_eq!(compiled.gaps.missing_textures, 1);
    assert_eq!(compiled.overlay.visuals.len(), 1);
    let visual = compiled.overlay.visuals[0];
    assert_eq!(visual.kind, VisualKind::Model);
    let template = compiled.overlay.model_templates[visual.model_template as usize];
    assert!(
        template.quad_count > 0,
        "failed quads stay in the mesh with the diagnostic material"
    );
    assert!(
        compiled.overlay.model_quads[template.quad_start as usize..]
            [..template.quad_count as usize]
            .iter()
            .all(|quad| quad.material == assets::DIAGNOSTIC_MATERIAL)
    );
    RuntimeAssets::diagnostic()
        .with_block_overlay(1, &compiled.overlay)
        .expect("partial models apply");
}
