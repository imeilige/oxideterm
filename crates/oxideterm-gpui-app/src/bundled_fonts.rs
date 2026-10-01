// Copyright (C) 2026 AnalyseDeCircuit
// SPDX-License-Identifier: GPL-3.0-only

//! Bundled terminal font registration for the native GPUI app.
//!
//! MapleMono's CN face is embedded as an independent Zstd frame and decompressed
//! only when registered. GPUI/font-kit still receives the original SFNT bytes.
//! Registration stays lazy: startup and terminal-open paths load only the
//! selected font's critical faces, matching Tauri's fontLoader strategy.

use std::borrow::Cow;
use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};

use anyhow::{Context as _, Result};
use gpui::TextSystem;
use oxideterm_settings::{FontFamily, PersistedSettings};

const JETBRAINS_REGULAR: &[u8] =
    include_bytes!("../resources/fonts/JetBrainsMono/JetBrainsMonoNerdFontMono-Subset-Regular.ttf");
const JETBRAINS_BOLD: &[u8] =
    include_bytes!("../resources/fonts/JetBrainsMono/JetBrainsMonoNerdFontMono-Subset-Bold.ttf");
const JETBRAINS_ITALIC: &[u8] =
    include_bytes!("../resources/fonts/JetBrainsMono/JetBrainsMonoNerdFontMono-Subset-Italic.ttf");
const JETBRAINS_BOLD_ITALIC: &[u8] = include_bytes!(
    "../resources/fonts/JetBrainsMono/JetBrainsMonoNerdFontMono-Subset-BoldItalic.ttf"
);
const MAPLE_REGULAR: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/MapleMono-NF-CN-Subset-Regular.ttf.zst"
));

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum BundledTerminalFace {
    JetBrainsRegular,
    JetBrainsBold,
    JetBrainsItalic,
    JetBrainsBoldItalic,
    MapleRegular,
}

impl BundledTerminalFace {
    fn embedded_bytes(self) -> &'static [u8] {
        match self {
            Self::JetBrainsRegular => JETBRAINS_REGULAR,
            Self::JetBrainsBold => JETBRAINS_BOLD,
            Self::JetBrainsItalic => JETBRAINS_ITALIC,
            Self::JetBrainsBoldItalic => JETBRAINS_BOLD_ITALIC,
            Self::MapleRegular => MAPLE_REGULAR,
        }
    }

    pub(crate) fn load(self) -> Result<Vec<u8>> {
        let bytes = self.embedded_bytes();
        if matches!(self, Self::MapleRegular) {
            // The build script writes frames with their original length, allowing one allocation.
            let size = zstd::zstd_safe::get_frame_content_size(bytes)
                .map_err(|error| anyhow::anyhow!("invalid bundled font {self:?}: {error}"))?
                .context("bundled font frame has no content size")?;
            zstd::bulk::decompress(bytes, usize::try_from(size)?)
                .with_context(|| format!("failed to decompress bundled font {self:?}"))
        } else {
            Ok(bytes.to_vec())
        }
    }
}

#[allow(dead_code)]
const ALL_TERMINAL_FACES: &[BundledTerminalFace] = &[
    BundledTerminalFace::JetBrainsRegular,
    BundledTerminalFace::JetBrainsBold,
    BundledTerminalFace::JetBrainsItalic,
    BundledTerminalFace::JetBrainsBoldItalic,
    BundledTerminalFace::MapleRegular,
];

static LOADED_TERMINAL_FACES: LazyLock<Mutex<HashSet<BundledTerminalFace>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

pub(crate) fn load_terminal_font_open_critical(
    settings: &PersistedSettings,
    text_system: &TextSystem,
) -> Result<()> {
    let faces = critical_faces_for_settings(settings);
    register_faces(text_system, &faces)
}

pub(crate) fn load_terminal_cjk_fallback_regular(
    text_system: &TextSystem,
    cjk_font_family: &str,
) -> Result<()> {
    if !should_load_terminal_cjk_fallback(cjk_font_family) {
        return Ok(());
    }
    register_faces(text_system, &[BundledTerminalFace::MapleRegular])
}

fn critical_faces_for_family(family: FontFamily) -> &'static [BundledTerminalFace] {
    match family {
        // Tauri prepares regular+bold for Latin bundled fonts before open.
        FontFamily::Jetbrains => &[
            BundledTerminalFace::JetBrainsRegular,
            BundledTerminalFace::JetBrainsBold,
        ],
        // Maple is large and only the regular face is embedded; styled CJK
        // runs fall back to the system CJK fonts.
        FontFamily::Maple => &[BundledTerminalFace::MapleRegular],
        FontFamily::Cascadia | FontFamily::Consolas | FontFamily::Menlo | FontFamily::Custom => &[],
    }
}

fn critical_faces_for_settings(settings: &PersistedSettings) -> Vec<BundledTerminalFace> {
    // The editor and Markdown code surfaces always use the bundled JetBrains family,
    // independently of the terminal's selected family.
    let mut faces = vec![BundledTerminalFace::JetBrainsRegular];
    let terminal_faces = critical_faces_for_family(settings.terminal.font_family);
    let terminal_faces = if terminal_faces.is_empty() {
        // System and custom choices still need a bundled monospace fallback if lookup fails.
        critical_faces_for_family(FontFamily::Jetbrains)
    } else {
        terminal_faces
    };
    for face in terminal_faces {
        if !faces.contains(face) {
            faces.push(*face);
        }
    }
    if settings.terminal.cjk_font_family.trim() == oxideterm_settings::MAPLE_MONO_SUBSET_FAMILY
        && !faces.contains(&BundledTerminalFace::MapleRegular)
    {
        // Explicit CJK fallback selection should be ready with the terminal,
        // while Auto keeps the existing delayed fallback warmup path.
        faces.push(BundledTerminalFace::MapleRegular);
    }
    faces
}

fn should_load_terminal_cjk_fallback(cjk_font_family: &str) -> bool {
    let cjk_font_family = cjk_font_family.trim();
    cjk_font_family.is_empty() || cjk_font_family == oxideterm_settings::MAPLE_MONO_SUBSET_FAMILY
}

fn register_faces(text_system: &TextSystem, faces: &[BundledTerminalFace]) -> Result<()> {
    let mut inserted_faces = Vec::new();
    let fonts = {
        let mut loaded = LOADED_TERMINAL_FACES
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut fonts = Vec::new();
        for face in faces {
            if loaded.contains(face) || inserted_faces.contains(face) {
                continue;
            }
            fonts.push(Cow::Owned(face.load()?));
            inserted_faces.push(*face);
        }
        // A decode failure must leave every face retryable, including earlier faces in this batch.
        loaded.extend(inserted_faces.iter().copied());
        fonts
    };
    if fonts.is_empty() {
        return Ok(());
    }
    if let Err(error) = text_system.add_fonts(fonts) {
        let mut loaded = LOADED_TERMINAL_FACES
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for face in inserted_faces {
            loaded.remove(&face);
        }
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_maple_face_preserves_the_complete_original_font() {
        let face = BundledTerminalFace::MapleRegular;
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources/fonts/MapleMono/MapleMono-NF-CN-Subset-Regular.ttf");
        let original = std::fs::read(path).unwrap();
        let decoded = face.load().unwrap();
        assert_eq!(
            decoded, original,
            "{face:?} must preserve every original byte"
        );
        assert!(
            face.embedded_bytes().len() < original.len(),
            "{face:?} must reduce embedded size"
        );
    }

    #[test]
    fn bundled_terminal_faces_use_one_runtime_family_name() {
        for face in ALL_TERMINAL_FACES {
            let expected_family = match face {
                BundledTerminalFace::JetBrainsRegular
                | BundledTerminalFace::JetBrainsBold
                | BundledTerminalFace::JetBrainsItalic
                | BundledTerminalFace::JetBrainsBoldItalic => {
                    oxideterm_settings::JETBRAINS_MONO_SUBSET_FAMILY
                }
                BundledTerminalFace::MapleRegular => oxideterm_settings::MAPLE_MONO_SUBSET_FAMILY,
            };

            let bytes = face.load().unwrap();
            let runtime_family_names = sfnt_runtime_family_names(&bytes);
            assert!(
                !runtime_family_names.is_empty(),
                "{face:?} must declare a runtime family name"
            );
            assert!(
                runtime_family_names
                    .iter()
                    .all(|family| family == expected_family),
                "{face:?} declares conflicting runtime family names: {runtime_family_names:?}"
            );
        }
    }

    #[test]
    fn app_code_font_is_loaded_for_every_terminal_family() {
        for family in [
            FontFamily::Jetbrains,
            FontFamily::Maple,
            FontFamily::Cascadia,
            FontFamily::Consolas,
            FontFamily::Menlo,
            FontFamily::Custom,
        ] {
            let mut settings = PersistedSettings::default();
            settings.terminal.font_family = family;

            assert!(
                critical_faces_for_settings(&settings)
                    .contains(&BundledTerminalFace::JetBrainsRegular),
                "{family:?} must keep the app code font available"
            );
        }
    }

    fn sfnt_runtime_family_names(bytes: &[u8]) -> Vec<String> {
        let Ok(face) = ttf_parser::Face::parse(bytes, 0) else {
            return Vec::new();
        };
        face.names()
            .into_iter()
            .filter(|name| {
                name.name_id == ttf_parser::name_id::FAMILY
                    || name.name_id == ttf_parser::name_id::TYPOGRAPHIC_FAMILY
            })
            .filter_map(|name| name.to_string())
            .collect()
    }
}
