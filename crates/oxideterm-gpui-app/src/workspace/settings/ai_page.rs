use super::*;

pub(in crate::workspace) const AI_CONFIRM_DIALOG_WIDTH: f32 = 448.0; // Tauri DialogContent max-w-md.
pub(in crate::workspace) const AI_KEY_REMOVE_DIALOG_WIDTH: f32 = 384.0; // Tauri useConfirm max-w-sm.
pub(in crate::workspace) const AI_CONFIRM_BULLET_SIZE: f32 = 4.0; // Tauri w-1 h-1.
pub(in crate::workspace) const AI_CONFIRM_ICON_WRAP: f32 = 48.0; // Tauri useConfirm w-12 h-12.
pub(in crate::workspace) const AI_CONFIRM_ICON: f32 = 24.0; // Tauri useConfirm w-6 h-6.
pub(in crate::workspace) const AI_TEXT_EDITOR_MODAL_WIDTH: f32 = 760.0;
pub(in crate::workspace) const AI_TEXT_EDITOR_MODAL_HEIGHT: f32 = 640.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::workspace) enum AiTextEditorDialog {
    Memory,
}

#[path = "ai/dialogs.rs"]
mod dialogs;
#[path = "ai/helpers.rs"]
mod helpers;
#[path = "ai/mcp.rs"]
mod mcp;
#[path = "ai/provider_actions.rs"]
mod provider_actions;
#[path = "ai/provider_card.rs"]
mod provider_card;
#[path = "ai/provider_keys.rs"]
mod provider_keys;
#[path = "ai/sections.rs"]
mod sections;
#[path = "ai/surface.rs"]
mod surface;
