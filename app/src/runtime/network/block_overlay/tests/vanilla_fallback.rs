use super::*;
use protocol::{CustomBlockVisuals, CustomVisualComponents};

/// A full-block furniture piece naming one texture on every face.
fn chair(name: &str, texture: &str) -> protocol::CustomBlock {
    block(
        name,
        1,
        CustomBlockVisuals {
            base: CustomVisualComponents {
                geometry: Some("minecraft:geometry.full_block".into()),
                materials: materials(texture),
                ..CustomVisualComponents::default()
            },
            ..CustomBlockVisuals::default()
        },
    )
}

/// Base carrier materials behind the vanilla keys: oak and lucky live on the
/// base page, so the overlay can borrow them when the server ships no raster.
/// Layer zero matches the diagnostic session base the tests apply onto.
fn base_materials() -> Vec<assets::Material> {
    vec![
        assets::Material::unvaried(),
        assets::Material {
            texture: assets::TextureRef::new(0, 0).unwrap(),
            ..assets::Material::unvaried()
        },
        assets::Material {
            texture: assets::TextureRef::new(0, 0).unwrap(),
            ..assets::Material::unvaried()
        },
    ]
}

fn oak_keys() -> assets::MaterialKeys {
    assets::MaterialKeys::from_entries([(1, "oak_planks")])
        .with_aliases([("oak_planks", "textures/blocks/oak_planks")])
}

fn keys() -> assets::MaterialKeys {
    assets::MaterialKeys::from_entries([(1, "oak_planks"), (2, "lucky")]).with_aliases([
        ("oak_planks", "textures/blocks/oak_planks"),
        ("lucky", "textures/blocks/lucky"),
    ])
}

fn compile(
    blocks: &CustomBlocks,
    keys: Option<&assets::MaterialKeys>,
) -> super::super::CompiledBlockOverlay {
    let base = base_materials();
    super::super::compile_block_overlay(
        &view(),
        blocks,
        false,
        keys,
        Some(&base),
        assets::MAX_TEXTURE_LAYERS as u32,
    )
    .expect("overlay")
}

// Furniture naming vanilla planks the server stack does not ship renders
// from the base page instead of degrading to the diagnostic look.
#[test]
fn vanilla_faces_reuse_the_base_page_without_missing_textures() {
    let keys = oak_keys();
    let blocks = CustomBlocks {
        blocks: vec![chair("test:chair", "oak_planks")].into(),
        ..Default::default()
    };
    let compiled = compile(&blocks, Some(&keys));
    assert_eq!(compiled.gaps.missing_textures, 0);
    assert_eq!(compiled.overlay.visuals.len(), 1);
    let visual = compiled.overlay.visuals[0];
    assert_eq!(visual.kind, VisualKind::Cube);
    let material = compiled.overlay.materials[visual.faces[BlockFace::Up as usize] as usize];
    assert_eq!(
        (material.texture.page(), material.texture.layer()),
        (0, 0),
        "the chair face reuses the base oak layer"
    );
    assert_eq!(
        compiled.overlay.material_overrides.len(),
        0,
        "the vanilla fallback itself claims no override"
    );
    let session = RuntimeAssets::diagnostic()
        .with_block_overlay(1, &compiled.overlay)
        .expect("base-page materials apply");
    let admitted = session.resolve(NetworkIdMode::Sequential, 1);
    assert!(admitted.is_known());
    assert_eq!(admitted.kind(), VisualKind::Cube);
}

// Without base materials the same chair keeps the old diagnostic behavior.
#[test]
fn vanilla_faces_without_a_base_stay_diagnostic() {
    let blocks = CustomBlocks {
        blocks: vec![chair("test:chair", "oak_planks")].into(),
        ..Default::default()
    };
    let compiled = super::compile_block_overlay(&view(), &blocks, false, None).expect("overlay");
    assert_eq!(compiled.overlay.visuals[0].kind, VisualKind::Diagnostic);
    assert_eq!(compiled.gaps.missing_textures, 1);
}

// A server raster wins over the base page for the same key.
#[test]
fn server_rasters_win_over_the_base_page() {
    let keys = keys();
    let blocks = CustomBlocks {
        blocks: vec![chair("test:lucky_chair", "lucky")].into(),
        ..Default::default()
    };
    let compiled = compile(&blocks, Some(&keys));
    assert_eq!(compiled.gaps.missing_textures, 0);
    let visual = compiled.overlay.visuals[0];
    let material = compiled.overlay.materials[visual.faces[BlockFace::Up as usize] as usize];
    assert_eq!(
        material.texture.page(),
        1,
        "the shipped server raster stays on the overlay page"
    );
    assert_eq!(
        compiled.overlay.material_overrides.len(),
        1,
        "only the shipped lucky raster repoints a base material"
    );
    let replacement = &compiled.overlay.material_overrides[0];
    assert_eq!(replacement.material, 2);
    assert_eq!(replacement.texture.page(), 1);
}

// Keys the base carrier never compiled keep degrading instead of borrowing.
#[test]
fn unknown_keys_stay_diagnostic_with_a_base() {
    let keys = keys();
    let blocks = CustomBlocks {
        blocks: vec![chair("test:stranger", "absent")].into(),
        ..Default::default()
    };
    let compiled = compile(&blocks, Some(&keys));
    assert_eq!(compiled.overlay.visuals[0].kind, VisualKind::Diagnostic);
    assert_eq!(compiled.gaps.missing_textures, 1);
}
