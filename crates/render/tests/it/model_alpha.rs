//! Opaque model promotion must retain cube alpha semantics.
use crate::{
    gpu_snapshot::{Draw, Gpu, SNAPSHOT_SIDE},
    material_shader, shader_source,
};

#[test]
fn displaced_models_preserve_opaque_texels_and_cutout_thresholds() {
    let Some(gpu) = Gpu::for_fixture("model opaque and cutout alpha") else {
        return;
    };
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("opaque/cutout source alpha"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 4,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let pixels: Vec<_> = [0, 127, 128, 255]
        .into_iter()
        .flat_map(|alpha| [80, 120, 160, alpha])
        .collect();
    gpu.queue.write_texture(
        texture.as_image_copy(),
        &pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: Some(1),
        },
        texture.size(),
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    });
    let sampler = gpu
        .device
        .create_sampler(&material_shader::native_leaf_sampler_descriptor());
    let texture_refs = gpu.words(
        &vec![u32::MAX; assets::MAX_TEXTURE_PAGES * assets::MAX_TEXTURE_LAYERS],
        wgpu::BufferUsages::STORAGE,
    );
    let view_uniform = gpu.buffer(&[0.0; 104], wgpu::BufferUsages::UNIFORM);
    let frame = gpu.buffer(&[0.0; 256], wgpu::BufferUsages::UNIFORM);
    let resources = alpha_resources(&view, &sampler, &texture_refs, &view_uniform, &frame);
    for (enhanced, shadow) in [(false, false), (true, false), (false, true)] {
        let definitions: &[&str] = if shadow {
            &["OPAQUE_OVERDRAW", "ENHANCED_SHADOW"]
        } else if enhanced {
            &["OPAQUE_OVERDRAW", "ENHANCED"]
        } else {
            &["OPAQUE_OVERDRAW"]
        };
        let mut production =
            shader_source::standalone(include_str!("../../src/model.wgsl"), definitions);
        if shadow {
            production = production.replace("@fragment\nfn fragment_shadow(", "fn sampled_shadow(")
                .replace("@builtin(front_facing) front: bool", "front: bool");
            production.push_str("\n@fragment fn shadow_colour(in: VertexOutput) -> @location(0) vec4<f32> { sampled_shadow(in, true); return vec4(1.0); }\n");
        }
        let source = format!(
            "{}\n{}",
            production,
            VERTEX.replace(
                "ALPHA_FLAG",
                &format!("{}u", assets::MATERIAL_FLAG_ALPHA_CUTOUT)
            )
        );
        let source = source.replace(
            "@group(2) @binding(0)",
            &format!("@group(0) @binding({FRAME_BINDING})"),
        );
        let fragment = if shadow { "shadow_colour" } else { "fragment" };
        let bindings = used_alpha_resources(&source, fragment, &resources);
        let actual = gpu.render(
            &source,
            "alpha_vertex",
            &[Draw {
                fragment,
                vertices: 0..48,
                bindings: &bindings,
                blend: None,
                write_depth: true,
            }],
        );
        let clear_offset = ((SNAPSHOT_SIDE as usize / 2) * SNAPSHOT_SIDE as usize
            + 9 * SNAPSHOT_SIDE as usize / 16)
            * 4;
        let clear = &actual[clear_offset..clear_offset + 3];
        assert_ne!(
            clear, &[255; 3],
            "cutout alpha0 leaves the clear colour untouched"
        );
        for case in 0..8usize {
            let offset = ((SNAPSHOT_SIDE as usize / 2) * SNAPSHOT_SIDE as usize
                + (case * 2 + 1) * SNAPSHOT_SIDE as usize / 16)
                * 4;
            assert_eq!(
                &actual[offset..offset + 3],
                if case == 4 || case == 5 {
                    clear
                } else {
                    &[255; 3]
                },
                "Enhanced={enhanced} shadow={shadow} case={case}: opaque alpha always draws; cutout discards only below128"
            );
        }
    }
    // Isolate colour conversion while retaining production sampling, discard and output alpha.
    let mut production = shader_source::standalone(include_str!("../../src/model.wgsl"), &[])
        .replace(
            "fn ordinary_world_model_colour(",
            "fn original_world_model_colour(",
        );
    production.push_str("\nfn ordinary_world_model_colour(in: VertexOutput, sampled: vec4<f32>) -> vec4<f32> { return sampled; }\n");
    let source = format!(
        "{production}\n{}",
        VERTEX.replace(
            "ALPHA_FLAG",
            &format!("{}u", assets::MATERIAL_FLAG_ALPHA_CUTOUT)
        )
    );
    let bindings = used_alpha_resources(&source, "fragment", &resources);
    let actual = gpu.render(
        &source,
        "alpha_vertex",
        &[Draw {
            fragment: "fragment",
            vertices: 0..48,
            bindings: &bindings,
            blend: None,
            write_depth: true,
        }],
    );
    for case in [0, 1, 2, 3, 6, 7] {
        let offset = ((SNAPSHOT_SIDE as usize / 2) * SNAPSHOT_SIDE as usize
            + (case * 2 + 1) * SNAPSHOT_SIDE as usize / 16)
            * 4;
        assert_eq!(&actual[offset..offset + 3], &[80, 120, 160]);
        assert_eq!(
            actual[offset + 3],
            if case == 6 { 128 } else { 255 },
            "opaque model writes cube opacity, while the cutout threshold keeps its admitted alpha"
        );
    }
}

const FRAME_BINDING: u32 = material_shader::ENHANCED_TEXTURE_REF_BINDING + 1;

/// Supplies neutral material resources while the fixture varies only source alpha.
fn alpha_resources<'a>(
    view: &'a wgpu::TextureView,
    sampler: &'a wgpu::Sampler,
    texture_refs: &'a wgpu::Buffer,
    view_uniform: &'a wgpu::Buffer,
    frame: &'a wgpu::Buffer,
) -> Vec<wgpu::BindGroupEntry<'a>> {
    let mut resources = vec![
        wgpu::BindGroupEntry {
            binding: 0,
            resource: view_uniform.as_entire_binding(),
        },
        wgpu::BindGroupEntry {
            binding: FRAME_BINDING,
            resource: frame.as_entire_binding(),
        },
        wgpu::BindGroupEntry {
            binding: material_shader::ENHANCED_TEXTURE_REF_BINDING,
            resource: texture_refs.as_entire_binding(),
        },
    ];
    for binding in [4, 5]
        .into_iter()
        .chain(material_shader::NATIVE_LEAF_TEXTURE_BINDINGS)
        .chain(material_shader::PBR_NORMAL_TEXTURE_BINDINGS)
        .chain(material_shader::PBR_MER_TEXTURE_BINDINGS)
        .chain(material_shader::ENHANCED_COLOR_TEXTURE_BINDINGS)
        .chain(material_shader::ENHANCED_NORMAL_TEXTURE_BINDINGS)
        .chain(material_shader::ENHANCED_MER_TEXTURE_BINDINGS)
    {
        resources.push(wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(view),
        });
    }
    for binding in [
        6,
        material_shader::NATIVE_LEAF_SAMPLER_BINDING,
        material_shader::PBR_SAMPLER_BINDING,
        material_shader::ENHANCED_SAMPLER_BINDING,
    ] {
        resources.push(wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::Sampler(sampler),
        });
    }
    resources
}

/// Selects the bindings reached by the fixture's real vertex and fragment entry points.
fn used_alpha_resources<'a>(
    source: &str,
    fragment: &str,
    resources: &[wgpu::BindGroupEntry<'a>],
) -> Vec<wgpu::BindGroupEntry<'a>> {
    let module = naga::front::wgsl::parse_str(source).unwrap();
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap();
    let used: std::collections::BTreeSet<_> = module
        .global_variables
        .iter()
        .filter_map(|(handle, global)| {
            let binding = global.binding.as_ref()?;
            let used = module
                .entry_points
                .iter()
                .enumerate()
                .any(|(index, entry)| {
                    (entry.name == "alpha_vertex" || entry.name == fragment)
                        && !info.get_entry_point(index)[handle].is_empty()
                });
            (used && binding.group == 0).then_some(binding.binding)
        })
        .collect();
    let selected: Vec<_> = resources
        .iter()
        .filter(|resource| used.contains(&resource.binding))
        .cloned()
        .collect();
    assert_eq!(
        selected.len(),
        used.len(),
        "alpha fixture must supply every active terrain binding"
    );
    selected
}

const VERTEX: &str = r#"
@vertex fn alpha_vertex(@builtin(vertex_index) index: u32) -> VertexOutput {
    let indices = array<u32,6>(0u,1u,2u,0u,2u,3u);
    let corners = array(vec2(0.0,0.0),vec2(1.0,0.0),vec2(1.0,1.0),vec2(0.0,1.0));
    let cell = index / 6u;
    let corner = corners[indices[index % 6u]];
    var out: VertexOutput;
    out.clip_position = vec4((vec2(f32(cell),0.0)+corner) / vec2(8.0,1.0)*vec2(2.0,-2.0)+vec2(-1.0,1.0),0.5,1.0);
    out.uv = vec2(0.5);
    out.current_texture = 0x80000000u | (cell % 4u);
    out.next_texture = out.current_texture;
    out.visible = 1u;
    out.two_sided = 1u;
    out.material_flags = select(0u, ALPHA_FLAG, cell >= 4u);
    return out;
}
"#;
