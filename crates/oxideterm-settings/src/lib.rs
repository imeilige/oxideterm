// Copyright (C) 2026 AnalyseDeCircuit
// SPDX-License-Identifier: GPL-3.0-only

mod model;
mod normalize;
mod oxide_snapshot;
mod store;

pub use model::*;
pub use normalize::{SanitizedSettings, sanitize_settings_value};
pub use oxide_snapshot::{
    ALL_OXIDE_SETTINGS_SECTIONS, DEFAULT_OXIDE_SETTINGS_SECTIONS, OXIDE_SETTINGS_FORMAT,
    OXIDE_SETTINGS_VERSION, export_oxide_settings_snapshot_json, merge_oxide_settings_snapshot,
};
pub use oxideterm_portable_runtime as portable_runtime;
pub use store::{
    DataDirectoryCheck, DataDirectoryInfo, SETTINGS_FILENAME, SettingsLoadResult,
    SettingsSaveResult, SettingsStore, SettingsStoreCheckpoint, check_data_directory,
    data_directory_info, default_settings_path, reset_data_directory, save_settings_to_path,
    set_data_directory,
};
