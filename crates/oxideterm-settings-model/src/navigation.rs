// Copyright (C) 2026 AnalyseDeCircuit
// SPDX-License-Identifier: GPL-3.0-only

//! Settings page navigation and virtual-section model rules.
//!
//! The GPUI app supplies runtime counts for data-backed pages, while this
//! module owns the invariant section-count math shared by the settings page.

use std::collections::HashSet;

use crate::{SettingsTab, TerminalSettingsPage};

pub const SETTINGS_SECTION_HEADER_ITEM_COUNT: usize = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettingsNavigationLayout {
    groups: Vec<Vec<SettingsTab>>,
}

impl Default for SettingsNavigationLayout {
    fn default() -> Self {
        Self::from_groups(SettingsTab::groups())
    }
}

impl SettingsNavigationLayout {
    pub fn from_persisted_groups(persisted_groups: &[Vec<String>]) -> Self {
        if persisted_groups.is_empty() {
            return Self::default();
        }

        let mut groups = Vec::with_capacity(persisted_groups.len());
        let mut seen = HashSet::new();

        for persisted_group in persisted_groups {
            let mut group = Vec::with_capacity(persisted_group.len());
            for id in persisted_group {
                if let Some(tab) = SettingsTab::from_id(id)
                    && seen.insert(tab)
                {
                    group.push(tab);
                }
            }
            if !group.is_empty() {
                groups.push(group);
            }
        }

        if groups.is_empty() {
            return Self::default();
        }

        // New settings pages remain reachable when an older custom layout is loaded.
        for tab in SettingsTab::all() {
            if seen.insert(*tab) {
                groups
                    .last_mut()
                    .expect("validated navigation layout has a group")
                    .push(*tab);
            }
        }

        Self { groups }
    }

    pub fn groups(&self) -> &[Vec<SettingsTab>] {
        &self.groups
    }

    pub fn group_count(&self) -> usize {
        self.groups.len()
    }

    pub fn add_group(&mut self) {
        self.groups.push(Vec::new());
    }

    pub fn remove_empty_group(&mut self, group_index: usize) -> bool {
        if self.groups.len() <= 1
            || self
                .groups
                .get(group_index)
                .is_none_or(|group| !group.is_empty())
        {
            return false;
        }
        self.groups.remove(group_index);
        true
    }

    pub fn move_tab_to_position(&mut self, tab: SettingsTab, target: SettingsTab) -> bool {
        if tab == target {
            return false;
        }
        let Some((target_group_index, target_index)) = self.tab_position(target) else {
            return false;
        };
        let Some(tab) = self.remove_tab(tab) else {
            return false;
        };
        let insertion_index = target_index.min(self.groups[target_group_index].len());
        self.groups[target_group_index].insert(insertion_index, tab);
        true
    }

    pub fn move_tab_to_group_start(&mut self, tab: SettingsTab, group_index: usize) -> bool {
        if group_index >= self.groups.len() {
            return false;
        }
        let Some(tab) = self.remove_tab(tab) else {
            return false;
        };
        self.groups[group_index].insert(0, tab);
        true
    }

    pub fn move_tab_to_group_end(&mut self, tab: SettingsTab, group_index: usize) -> bool {
        if group_index >= self.groups.len() {
            return false;
        }
        let Some(tab) = self.remove_tab(tab) else {
            return false;
        };
        self.groups[group_index].push(tab);
        true
    }

    pub fn move_group_to_position(&mut self, source_index: usize, target_index: usize) -> bool {
        if source_index == target_index
            || source_index >= self.groups.len()
            || target_index >= self.groups.len()
        {
            return false;
        }
        let group = self.groups.remove(source_index);
        let insertion_index = target_index.min(self.groups.len());
        self.groups.insert(insertion_index, group);
        true
    }

    pub fn move_group_to_end(&mut self, source_index: usize) -> bool {
        if source_index + 1 >= self.groups.len() {
            return false;
        }
        let group = self.groups.remove(source_index);
        self.groups.push(group);
        true
    }

    pub fn to_persisted_groups(&self) -> Vec<Vec<String>> {
        self.groups
            .iter()
            .filter(|group| !group.is_empty())
            .map(|group| group.iter().map(|tab| tab.id().to_string()).collect())
            .collect()
    }

    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }

    fn from_groups(groups: &[&[SettingsTab]]) -> Self {
        Self {
            groups: groups.iter().map(|group| group.to_vec()).collect(),
        }
    }

    fn tab_position(&self, tab: SettingsTab) -> Option<(usize, usize)> {
        self.groups
            .iter()
            .enumerate()
            .find_map(|(group_index, group)| {
                group
                    .iter()
                    .position(|candidate| *candidate == tab)
                    .map(|tab_index| (group_index, tab_index))
            })
    }

    fn remove_tab(&mut self, tab: SettingsTab) -> Option<SettingsTab> {
        let (group_index, tab_index) = self.tab_position(tab)?;
        Some(self.groups[group_index].remove(tab_index))
    }
}

pub fn settings_section_list_item_count(
    tab: SettingsTab,
    dynamic: SettingsDynamicSectionCounts,
) -> usize {
    SETTINGS_SECTION_HEADER_ITEM_COUNT + settings_tab_section_count(tab, dynamic)
}

pub fn settings_tab_section_count(
    tab: SettingsTab,
    dynamic: SettingsDynamicSectionCounts,
) -> usize {
    match tab {
        // The application-lock card was retired with the workspace lock. The
        // General page is now: language, launch at login, window behavior
        // (desktop only), data directory, CLI companion.
        SettingsTab::General => {
            if cfg!(any(target_os = "windows", target_os = "macos")) {
                5
            } else {
                4
            }
        }
        SettingsTab::Terminal => terminal_settings_section_count(dynamic.terminal_page),
        SettingsTab::Appearance => 4,
        // Reconnect controls share one card and therefore one virtual section.
        SettingsTab::Connections => 5,
        SettingsTab::Sftp => 3,
        SettingsTab::Keybindings => {
            keybinding_settings_section_count(dynamic.visible_keybinding_scope_count)
        }
        // The Help page only owns the diagnostics card. The version, tech-stack,
        // resources, and safety cards went with the update feature, and the
        // copyright/legal footer now lives in the onboarding disclaimer only.
        SettingsTab::Help => 1,
    }
}

pub fn terminal_settings_section_count(page: TerminalSettingsPage) -> usize {
    let page_cards = match page {
        TerminalSettingsPage::Display => 5,
        TerminalSettingsPage::Input => 3,
        // The dedicated keybindings page owns shortcut discovery and editing.
        TerminalSettingsPage::Local => 3,
        TerminalSettingsPage::CommandBar => 3,
        TerminalSettingsPage::Awareness => 3,
        TerminalSettingsPage::Transfer => 1,
        TerminalSettingsPage::Highlight => 1,
    };
    1 + page_cards
}

pub fn keybinding_settings_section_count(visible_scope_count: usize) -> usize {
    1 + visible_scope_count.max(1)
}

pub fn settings_section_list_identity(
    tab: SettingsTab,
    terminal_page: TerminalSettingsPage,
) -> String {
    // Keybinding filters update row signatures rather than replacing the list;
    // this keeps the toolbar-mounted selection animation alive.
    format!("{tab:?}:{terminal_page:?}")
}

#[derive(Clone, Copy, Debug)]
pub struct SettingsDynamicSectionCounts {
    pub terminal_page: TerminalSettingsPage,
    pub visible_keybinding_scope_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn general_section_count_matches_the_cards_that_still_render() {
        // The application-lock card was retired. The renderer now maps
        // language, launch-at-login, window behavior (desktop only), data
        // directory and CLI companion onto consecutive indices, so a stale
        // count would leave a blank card between two real ones.
        let desktop = settings_tab_section_count(
            SettingsTab::General,
            SettingsDynamicSectionCounts {
                terminal_page: TerminalSettingsPage::Display,
                visible_keybinding_scope_count: 0,
            },
        );
        let expected = if cfg!(any(target_os = "windows", target_os = "macos")) {
            5
        } else {
            4
        };
        assert_eq!(
            desktop, expected,
            "General must not render an empty section"
        );
    }

    #[test]
    fn persisted_navigation_layout_ignores_invalid_entries_and_appends_new_tabs() {
        let persisted = vec![
            vec!["terminal".to_string(), "unknown".to_string()],
            vec!["general".to_string(), "terminal".to_string()],
        ];

        let layout = SettingsNavigationLayout::from_persisted_groups(&persisted);

        assert_eq!(layout.groups()[0], [SettingsTab::Terminal]);
        assert_eq!(layout.groups()[1][..1], [SettingsTab::General]);
        assert_eq!(
            layout.groups().iter().map(Vec::len).sum::<usize>(),
            SettingsTab::all().len()
        );
        assert_eq!(layout.groups().len(), 2);
    }

    #[test]
    fn navigation_pages_can_move_within_and_across_groups() {
        let mut layout = SettingsNavigationLayout::default();

        assert!(layout.move_tab_to_position(SettingsTab::Terminal, SettingsTab::Appearance));
        assert_eq!(
            layout.groups()[0][..3],
            [
                SettingsTab::General,
                SettingsTab::Terminal,
                SettingsTab::Appearance,
            ]
        );
        assert!(layout.move_tab_to_group_end(SettingsTab::General, 1));
        assert_eq!(layout.groups()[1].last(), Some(&SettingsTab::General));
    }

    #[test]
    fn navigation_groups_can_be_added_removed_and_reordered() {
        let mut layout = SettingsNavigationLayout::default();
        let original_first_group = layout.groups()[0].clone();
        // The default grouping is a product decision that moves whenever a
        // settings tab is added or removed, so assert the deltas this test is
        // actually about instead of an absolute count that goes stale.
        let baseline = layout.group_count();

        layout.add_group();
        assert_eq!(layout.group_count(), baseline + 1);
        assert!(layout.move_group_to_end(0));
        assert_eq!(layout.groups().last(), Some(&original_first_group));
        let empty = layout
            .groups()
            .iter()
            .position(|group| group.is_empty())
            .expect("the group just added");
        assert!(layout.remove_empty_group(empty));
        assert_eq!(layout.group_count(), baseline);
    }
}
