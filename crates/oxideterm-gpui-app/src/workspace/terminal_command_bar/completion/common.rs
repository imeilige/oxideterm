use super::*;

pub(super) fn normalize_terminal_command_suggestions(
    suggestions: Vec<TerminalCommandSuggestion>,
) -> Vec<TerminalCommandSuggestion> {
    let mut by_key: HashMap<String, TerminalCommandSuggestion> = HashMap::new();
    for suggestion in suggestions {
        let key = format!(
            "{}:{}:{}:{}:{}",
            suggestion.source_label_key,
            terminal_command_suggestion_kind_key(suggestion.kind),
            suggestion.insert_text,
            suggestion.replacement.start,
            suggestion.replacement.end
        );
        if by_key
            .get(&key)
            .is_none_or(|existing| suggestion.score > existing.score)
        {
            by_key.insert(key, suggestion);
        }
    }
    let mut ranked = by_key.into_values().collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.label.cmp(&right.label))
    });
    ranked.truncate(24);
    ranked
}

fn terminal_command_suggestion_kind_key(kind: TerminalCommandSuggestionKind) -> &'static str {
    match kind {
        TerminalCommandSuggestionKind::History => "history",
        TerminalCommandSuggestionKind::Command => "command",
        TerminalCommandSuggestionKind::Subcommand => "subcommand",
        TerminalCommandSuggestionKind::Option => "option",
        TerminalCommandSuggestionKind::File => "file",
        TerminalCommandSuggestionKind::Directory => "directory",
    }
}

pub(super) fn terminal_command_risk_score_penalty(risk: Option<&'static str>) -> f64 {
    match risk {
        Some("high") => 900.0,
        Some("medium") => 250.0,
        _ => 0.0,
    }
}

// Remote identity detection backs path completion only: a completed path must resolve
// against the remote host shown in the prompt rather than the local machine.
pub(super) fn terminal_cwd_looks_remote(cwd: &str) -> bool {
    cwd.starts_with("/home/")
        || cwd.starts_with("/root/")
        || cwd.starts_with("/srv/")
        || cwd.starts_with("/var/www/")
}

pub(super) fn infer_terminal_ssh_identity_from_buffer(buffer: &str) -> Option<String> {
    let tail_start = buffer
        .char_indices()
        .rev()
        .nth(8000)
        .map(|(index, _)| index)
        .unwrap_or(0);
    buffer[tail_start..]
        .split_whitespace()
        .filter_map(terminal_ssh_identity_candidate)
        .next_back()
}

fn terminal_ssh_identity_candidate(token: &str) -> Option<String> {
    if token.contains('=') {
        return None;
    }
    let end = token
        .char_indices()
        .find_map(|(index, ch)| {
            (matches!(ch, ':' | '~' | '#' | '$' | '>') && token[..index].contains('@'))
                .then_some(index)
        })
        .unwrap_or(token.len());
    let candidate = token[..end].trim_matches(|ch: char| {
        !(ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-' | '@'))
    });
    let (user, host) = candidate.split_once('@')?;
    if !(1..=64).contains(&user.len()) || !(1..=128).contains(&host.len()) {
        return None;
    }
    if !user
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
    {
        return None;
    }
    let mut host_chars = host.chars();
    if !host_chars
        .next()
        .is_some_and(|ch| ch.is_ascii_alphanumeric())
    {
        return None;
    }
    if !host_chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-')) {
        return None;
    }
    Some(format!("{user}@{host}"))
}

#[cfg(test)]
mod terminal_ssh_identity_tests {
    use super::{infer_terminal_ssh_identity_from_buffer, terminal_ssh_identity_candidate};

    #[test]
    fn infers_last_ssh_identity_from_terminal_buffer() {
        let buffer = "Last login\nuser@example.com:~$ ssh deploy@prod-box\n\
            deploy@prod-box:/srv/app$ ";

        assert_eq!(
            infer_terminal_ssh_identity_from_buffer(buffer),
            Some("deploy@prod-box".to_string())
        );
    }

    #[test]
    fn rejects_secret_like_or_malformed_identity_tokens() {
        assert_eq!(
            terminal_ssh_identity_candidate("token@example.com=abc"),
            None
        );
        assert_eq!(terminal_ssh_identity_candidate("@example.com:~$"), None);
        assert_eq!(
            terminal_ssh_identity_candidate("user@example.com:~$"),
            Some("user@example.com".to_string())
        );
    }
}
