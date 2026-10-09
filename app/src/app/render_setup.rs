//! Renderer setup shared by direct and launcher sessions.

use std::{ffi::OsStr, num::NonZeroU32};

use bevy::{
    render::{
        RenderPlugin,
        settings::{Backends, RenderCreation, WgpuSettings},
    },
    window::{PresentMode, Window},
};

/// One queued frame: input-to-photon drops a refresh against the default two.
const PRIMARY_FRAME_LATENCY: NonZeroU32 = NonZeroU32::MIN;

pub(super) fn primary_window(title: String, present_mode: PresentMode) -> Window {
    Window {
        title,
        present_mode,
        desired_maximum_frame_latency: Some(PRIMARY_FRAME_LATENCY),
        ..Default::default()
    }
}

pub(super) fn render_plugin() -> RenderPlugin {
    let mut settings = WgpuSettings::default();
    settings.limits.max_storage_buffers_per_shader_stage = settings
        .limits
        .max_storage_buffers_per_shader_stage
        .max(render::required_vertex_storage_buffers());
    if let Some(backends) = preferred_render_backends(std::env::var_os("WGPU_BACKEND").as_deref()) {
        settings.backends = Some(backends);
    }
    #[cfg(all(feature = "enhanced", target_os = "windows"))]
    render::configure_enhanced_shader_compiler(&mut settings);
    RenderPlugin {
        render_creation: RenderCreation::Automatic(settings),
        ..Default::default()
    }
}

/// Prefers capable Windows backends unless the operator selected one explicitly.
pub(crate) fn preferred_render_backends(explicit: Option<&OsStr>) -> Option<Backends> {
    if explicit.is_some() {
        return None;
    }
    #[cfg(target_os = "windows")]
    {
        // Prefer Vulkan for GPU terrain culling, with DX12 as the fallback.
        // An explicit WGPU_BACKEND retains operator control.
        Some(Backends::VULKAN | Backends::DX12)
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_window_queues_a_single_frame() {
        for present_mode in [PresentMode::Fifo, PresentMode::AutoNoVsync] {
            let window = primary_window(String::new(), present_mode);
            assert_eq!(
                window.desired_maximum_frame_latency.map(NonZeroU32::get),
                Some(1)
            );
            assert_eq!(window.present_mode, present_mode);
        }
    }
}
