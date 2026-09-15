//! `minc.toml` — edition, game version, and emit options (`docs/language/01-project-and-cli.md`).

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edition {
    Java,
    Bedrock,
}

impl Edition {
    pub fn as_str(self) -> &'static str {
        match self {
            Edition::Java => "java",
            Edition::Bedrock => "bedrock",
        }
    }

    pub fn byte(self) -> u8 {
        match self {
            Edition::Java => 1,
            Edition::Bedrock => 2,
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "java" => Some(Edition::Java),
            "bedrock" => Some(Edition::Bedrock),
            _ => None,
        }
    }

    /// World-register fake player used for `static int` fields.
    pub fn world_holder(self) -> &'static str {
        match self {
            Edition::Java => "#mcs",
            Edition::Bedrock => "mcs",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MincConfig {
    pub name: String,
    pub pack: String,
    pub score_revision: u32,
    pub edition: Edition,
    pub game_version: String,
    pub emit_mincb: bool,
    pub emit_functions: bool,
    pub emit_pack: bool,
    pub dimension: String,
    pub origin: [i32; 3],
    pub default_layout: String,
    pub default_facing: String,
    pub max_span: u32,
    pub clock: Option<String>,
    /// `"one"` (default) or `"function"`.
    pub chain_pack: String,
    pub prefer: String,
    pub default_cb_type: String,
    pub default_redstone: String,
    pub default_conditional: bool,
    pub track_output: bool,
    pub function_command_limit: u32,
    pub strict_raw: bool,
}

impl Default for MincConfig {
    fn default() -> Self {
        Self {
            name: "unnamed".into(),
            pack: "game".into(),
            score_revision: 1,
            edition: Edition::Bedrock,
            game_version: "1.21.70".into(),
            emit_mincb: true,
            emit_functions: true,
            emit_pack: true,
            dimension: "overworld".into(),
            origin: [0, 64, 0],
            default_layout: "stack".into(),
            default_facing: "up".into(),
            max_span: 32,
            clock: None,
            chain_pack: "one".into(),
            prefer: "execute".into(),
            default_cb_type: "chain".into(),
            default_redstone: "always_active".into(),
            default_conditional: false,
            track_output: false,
            function_command_limit: 10_000,
            strict_raw: false,
        }
    }
}

impl MincConfig {
    pub fn validate(&self) -> Result<(), String> {
        let (maj, min, pat) = parse_game_version(&self.game_version)?;
        match self.edition {
            Edition::Bedrock => {
                if (maj, min, pat) < (1, 19, 50) {
                    return Err(format!(
                        "Bedrock {} is below 1.19.50 (new /execute is required)",
                        self.game_version
                    ));
                }
            }
            Edition::Java => {
                if (maj, min, pat) < (1, 13, 0) {
                    return Err(format!(
                        "Java {} is below 1.13 (brigadier /execute is required)",
                        self.game_version
                    ));
                }
            }
        }
        if self.pack.is_empty() {
            return Err("project.pack is required".into());
        }
        Ok(())
    }

    pub fn bedrock_fn_prefix(&self) -> String {
        self.pack.replace('.', "/")
    }

    pub fn java_namespace(&self) -> &str {
        &self.pack
    }
}

pub fn parse_game_version(s: &str) -> Result<(u32, u32, u32), String> {
    let mut parts = s.split('.');
    let maj = parts
        .next()
        .and_then(|p| p.parse().ok())
        .ok_or_else(|| format!("invalid game_version `{s}`"))?;
    let min = parts
        .next()
        .and_then(|p| p.parse().ok())
        .ok_or_else(|| format!("invalid game_version `{s}`"))?;
    let pat = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    Ok((maj, min, pat))
}

pub fn parse_minc_toml(text: &str) -> Result<MincConfig, String> {
    let mut cfg = MincConfig::default();
    let mut section = String::new();
    let mut seen_edition = false;
    let mut seen_version = false;
    for (i, raw) in text.lines().enumerate() {
        let line = strip_comment(raw).trim().to_string();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].to_string();
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            return Err(format!("line {}: expected key = value", i + 1));
        };
        let key = k.trim();
        let val = v.trim();
        match (section.as_str(), key) {
            ("project", "name") => cfg.name = parse_string(val)?,
            ("project", "pack") => cfg.pack = parse_string(val)?,
            ("project", "score_revision") => cfg.score_revision = parse_u32(val)?,
            ("target", "edition") => {
                let s = parse_string(val)?;
                cfg.edition = Edition::parse(&s)
                    .ok_or_else(|| format!("unknown edition `{s}` (want java|bedrock)"))?;
                seen_edition = true;
            }
            ("target", "game_version") => {
                cfg.game_version = parse_string(val)?;
                seen_version = true;
            }
            ("emit", "mincb") => cfg.emit_mincb = parse_bool(val)?,
            ("emit", "functions") => cfg.emit_functions = parse_bool(val)?,
            ("emit", "datapack_or_behavior_pack") => cfg.emit_pack = parse_bool(val)?,
            ("world", "dimension") => cfg.dimension = parse_string(val)?,
            ("world", "origin") => cfg.origin = parse_i32_3(val)?,
            ("chains", "default_layout") => cfg.default_layout = parse_string(val)?,
            ("chains", "default_facing") => cfg.default_facing = parse_string(val)?,
            ("chains", "max_span") => cfg.max_span = parse_u32(val)?,
            ("chains", "clock") => cfg.clock = Some(parse_string(val)?),
            ("chains", "pack") => cfg.chain_pack = parse_string(val)?,
            ("chains", "prefer") => cfg.prefer = parse_string(val)?,
            ("command_block", "default_type") => cfg.default_cb_type = parse_string(val)?,
            ("command_block", "default_redstone") => cfg.default_redstone = parse_string(val)?,
            ("command_block", "default_conditional") => {
                cfg.default_conditional = parse_bool(val)?;
            }
            ("command_block", "track_output") => cfg.track_output = parse_bool(val)?,
            ("limits", "function_command_limit") => {
                cfg.function_command_limit = parse_u32(val)?;
            }
            _ => {
                return Err(format!("line {}: unknown key `{section}.{key}`", i + 1));
            }
        }
    }
    if !seen_edition {
        return Err("target.edition is required".into());
    }
    if !seen_version {
        return Err("target.game_version is required".into());
    }
    cfg.validate()?;
    Ok(cfg)
}

pub fn render_minc_toml(cfg: &MincConfig) -> String {
    format!(
        r#"[project]
name = "{name}"
pack = "{pack}"
score_revision = {rev}

[target]
edition = "{edition}"
game_version = "{ver}"

[emit]
mincb = {mincb}
functions = {funcs}
datapack_or_behavior_pack = {pack_emit}

[world]
dimension = "{dim}"
origin = [{ox}, {oy}, {oz}]

[chains]
default_layout = "{layout}"
default_facing = "{facing}"
max_span = {span}
{clock}pack = "{cpack}"
prefer = "{prefer}"
[command_block]
default_type = "{cb}"
default_redstone = "{red}"
default_conditional = {cond}
track_output = {track}

[limits]
function_command_limit = {limit}
"#,
        name = cfg.name,
        pack = cfg.pack,
        rev = cfg.score_revision,
        edition = cfg.edition.as_str(),
        ver = cfg.game_version,
        mincb = cfg.emit_mincb,
        funcs = cfg.emit_functions,
        pack_emit = cfg.emit_pack,
        dim = cfg.dimension,
        ox = cfg.origin[0],
        oy = cfg.origin[1],
        oz = cfg.origin[2],
        layout = cfg.default_layout,
        facing = cfg.default_facing,
        span = cfg.max_span,
        clock = cfg
            .clock
            .as_ref()
            .map(|c| format!("clock = \"{c}\"\n"))
            .unwrap_or_default(),
        cpack = cfg.chain_pack,
        prefer = cfg.prefer,
        cb = cfg.default_cb_type,
        red = cfg.default_redstone,
        cond = cfg.default_conditional,
        track = cfg.track_output,
        limit = cfg.function_command_limit,
    )
}

fn strip_comment(line: &str) -> String {
    let mut out = String::new();
    let mut in_str = false;
    for c in line.chars() {
        if c == '"' {
            in_str = !in_str;
            out.push(c);
        } else if c == '#' && !in_str {
            break;
        } else {
            out.push(c);
        }
    }
    out
}

fn parse_string(v: &str) -> Result<String, String> {
    if v.starts_with('"') && v.ends_with('"') && v.len() >= 2 {
        Ok(v[1..v.len() - 1].to_string())
    } else if v
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-')
    {
        Ok(v.to_string())
    } else {
        Err(format!("expected string, got `{v}`"))
    }
}

fn parse_bool(v: &str) -> Result<bool, String> {
    match v {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("expected bool, got `{v}`")),
    }
}

fn parse_u32(v: &str) -> Result<u32, String> {
    v.parse()
        .map_err(|_| format!("expected integer, got `{v}`"))
}

fn parse_i32_3(v: &str) -> Result<[i32; 3], String> {
    let v = v.trim();
    if !v.starts_with('[') || !v.ends_with(']') {
        return Err(format!("expected [x, y, z], got `{v}`"));
    }
    let inner = &v[1..v.len() - 1];
    let nums: Vec<i32> = inner
        .split(',')
        .map(|s| s.trim().parse::<i32>())
        .collect::<Result<_, _>>()
        .map_err(|_| format!("expected [x, y, z], got `{v}`"))?;
    if nums.len() != 3 {
        return Err(format!("expected 3 numbers, got `{v}`"));
    }
    Ok([nums[0], nums[1], nums[2]])
}

/// Tiny TOML table dump used by tests.
pub fn keys_in(text: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut section = String::new();
    for raw in text.lines() {
        let line = strip_comment(raw).trim().to_string();
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].to_string();
        } else if let Some((k, v)) = line.split_once('=') {
            out.insert(format!("{section}.{}", k.trim()), v.trim().to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_metro_toml() {
        let text = r#"
[project]
name = "metro-escape"
pack = "metro.escape"
score_revision = 1

[target]
edition = "bedrock"
game_version = "1.21.70"

[world]
origin = [0, 64, 0]
"#;
        let cfg = parse_minc_toml(text).expect("toml");
        assert_eq!(cfg.edition, Edition::Bedrock);
        assert_eq!(cfg.pack, "metro.escape");
        assert_eq!(cfg.origin, [0, 64, 0]);
    }

    #[test]
    fn rejects_old_bedrock() {
        let text = r#"
[target]
edition = "bedrock"
game_version = "1.18.0"
"#;
        assert!(parse_minc_toml(text).is_err());
    }
}
