//! Macro presses consume one control even when the chat queue cannot accept a send.
use bevy::prelude::MouseButton;
use client_ui::ui_runtime::UiRuntime;
use semantic_input::PhysicalControl;

use crate::{menu::MenuRuntime, semantic_controls::SemanticInputRuntime};

/// Sends the matching gameplay macro and retires only its sampled control.
pub(super) fn dispatch(
    menu: Option<&MenuRuntime>,
    runtime: &mut UiRuntime,
    alt: bool,
    control: Option<PhysicalControl>,
    now_millis: u64,
    semantic: Option<&mut SemanticInputRuntime>,
) -> bool {
    if !alt || runtime.chat_focused() || runtime.inventory_open() || runtime.local_sleeping() {
        return false;
    }
    let Some(control) = control else {
        return false;
    };
    let Some(command) =
        crate::menu::settings_options::control_bindings::binding_macro_command(menu, control)
    else {
        return false;
    };
    let _ = runtime.queue_chat_message(&command, now_millis);
    if let Some(semantic) = semantic {
        semantic.consume_control(control);
    }
    true
}

/// Matches pointer presses using the same binding codes as capture and gameplay.
pub(super) fn dispatch_mouse(
    menu: Option<&MenuRuntime>,
    runtime: &mut UiRuntime,
    alt: bool,
    button: MouseButton,
    now_millis: u64,
    semantic: Option<&mut SemanticInputRuntime>,
) -> bool {
    dispatch(
        menu,
        runtime,
        alt,
        crate::semantic_controls::physical::mouse_button_code(button)
            .map(PhysicalControl::MouseButton),
        now_millis,
        semantic,
    )
}
