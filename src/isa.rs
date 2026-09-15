//! Command ISA snapshots from `docs/minecraft-commands/*/data/command-index.json`.
//!
//! Emitted first-words must exist in the edition snapshot. Bedrock must not grow
//! Java `store` / `team` / `nbt=` / `distance=` / item-component `[]` syntax.

use crate::config::Edition;

const BEDROCK_INDEX: &str =
    include_str!("../docs/minecraft-commands/bedrock/data/command-index.json");
const JAVA_INDEX: &str = include_str!("../docs/minecraft-commands/java/data/command-index.json");

fn root_commands(json: &str) -> Vec<String> {
    let Some(key) = json.find("\"root_commands\"") else {
        return Vec::new();
    };
    let rest = &json[key..];
    let Some(bracket) = rest.find('[') else {
        return Vec::new();
    };
    let rest = &rest[bracket + 1..];
    let Some(end) = rest.find(']') else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut chars = rest[..end].chars().peekable();
    while let Some(c) = chars.next() {
        if c == '"' {
            let mut s = String::new();
            while let Some(ch) = chars.next() {
                if ch == '\\' {
                    if let Some(n) = chars.next() {
                        s.push(ch);
                        s.push(n);
                    }
                } else if ch == '"' {
                    break;
                } else {
                    s.push(ch);
                }
            }
            if !s.is_empty() {
                out.push(s);
            }
        }
    }
    out
}

fn snapshot(edition: Edition) -> Vec<String> {
    match edition {
        Edition::Bedrock => root_commands(BEDROCK_INDEX),
        Edition::Java => root_commands(JAVA_INDEX),
    }
}

/// First word of a command line (no leading `/`).
pub fn first_word(cmd: &str) -> &str {
    let cmd = cmd.trim().trim_start_matches('/');
    cmd.split_whitespace().next().unwrap_or("")
}

/// Returns an error message if `cmd` is illegal for `edition`.
pub fn check_command(edition: Edition, cmd: &str) -> Result<(), String> {
    let cmd = cmd.trim();
    if cmd.is_empty() || cmd.starts_with('#') {
        return Ok(());
    }
    let word = first_word(cmd);
    if word.is_empty() {
        return Ok(());
    }
    let allowed = snapshot(edition);
    if !allowed.iter().any(|c| c == word) {
        return Err(format!(
            "command `{word}` is not in the {} command snapshot",
            edition.as_str()
        ));
    }
    match edition {
        Edition::Bedrock => {
            if word == "team" || cmd.split_whitespace().any(|w| w == "team") && word == "execute" {
                return Err("`team` is Java-only".into());
            }
            if cmd.contains("execute store")
                || cmd.contains(" store result ")
                || cmd.contains(" store success ")
            {
                return Err("`execute store` is Java-only".into());
            }
            if cmd.contains("nbt=") {
                return Err("selector `nbt=` is Java-only".into());
            }
            if cmd.contains("distance=") {
                return Err("selector `distance=` is Java-only (use `r=` on Bedrock)".into());
            }
            if cmd.contains("item[") {
                return Err("Java item components `item[...]` are not valid on Bedrock".into());
            }
        }
        Edition::Java => {}
    }
    Ok(())
}

pub fn check_commands(
    edition: Edition,
    cmds: impl IntoIterator<Item = impl AsRef<str>>,
) -> Vec<String> {
    let mut errors = Vec::new();
    for cmd in cmds {
        if let Err(e) = check_command(edition, cmd.as_ref()) {
            errors.push(e);
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bedrock_index_has_execute() {
        assert!(snapshot(Edition::Bedrock).iter().any(|c| c == "execute"));
        assert!(snapshot(Edition::Bedrock).iter().any(|c| c == "scoreboard"));
        assert!(!snapshot(Edition::Bedrock).iter().any(|c| c == "team"));
    }

    #[test]
    fn java_index_has_team() {
        assert!(snapshot(Edition::Java).iter().any(|c| c == "team"));
        assert!(snapshot(Edition::Java).iter().any(|c| c == "return"));
    }

    #[test]
    fn rejects_store_on_bedrock() {
        assert!(check_command(
            Edition::Bedrock,
            "execute store result score @s m00 run data get entity @s Health"
        )
        .is_err());
        assert!(check_command(Edition::Bedrock, "scoreboard players add mcs m00 1").is_ok());
    }
}
