// Copyright (C) 2026 AnalyseDeCircuit
// SPDX-License-Identifier: GPL-3.0-only

use std::path::Path;


use crate::*;

fn syntax_scope_covers_range(
    spans: &[HighlightSpan],
    scope: SyntaxScope,
    range: std::ops::Range<usize>,
) -> bool {
    // Adjacent grammar tokens may represent one visual delimiter, such as `**`.
    let mut covered_until = range.start;
    for span in spans.iter().filter(|span| span.scope == scope) {
        if span.range.start.0 > covered_until {
            break;
        }
        if span.range.end.0 > covered_until {
            covered_until = span.range.end.0;
        }
        if covered_until >= range.end {
            return true;
        }
    }
    false
}

#[test]
fn detects_shell_language_extensions_and_shebangs() {
    assert_eq!(LanguageId::from_path("install.sh"), Some(LanguageId::Bash));
    assert_eq!(LanguageId::from_path("config.fish"), Some(LanguageId::Fish));
    assert_eq!(
        LanguageId::from_path("profile.ps1"),
        Some(LanguageId::Powershell)
    );
    assert_eq!(LanguageId::from_path(".zshrc"), Some(LanguageId::Zsh));
    assert_eq!(
        LanguageId::from_path("src/main.rs"),
        None,
        "the editor grammars are no longer linked"
    );

    assert_eq!(
        LanguageId::detect(Some(Path::new("run")), "#!/bin/bash\n"),
        Some(LanguageId::Bash)
    );
    assert_eq!(
        LanguageId::detect(Some(Path::new("run")), "#!/usr/bin/env zsh\n"),
        Some(LanguageId::Zsh)
    );
    assert_eq!(
        LanguageId::detect(Some(Path::new("run")), "#!/usr/bin/fish\n"),
        Some(LanguageId::Fish)
    );
    assert_eq!(
        LanguageId::detect(Some(Path::new("run")), "#!/usr/bin/env pwsh\n"),
        Some(LanguageId::Powershell)
    );
}






















