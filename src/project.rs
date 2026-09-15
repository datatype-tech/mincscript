//! Load a MincScript project (`minc.toml` + `src/**/*.mcs`).

use std::fs;
use std::path::{Path, PathBuf};

use crate::compile::{merge_units, ParsedFile};
use crate::config::{parse_minc_toml, MincConfig};
use crate::diagnostic::Diagnostic;
use crate::parse;

pub struct LoadedProject {
    pub root: PathBuf,
    pub config: MincConfig,
    pub files: Vec<ParsedFile>,
}

pub fn find_root(start: &Path) -> Option<PathBuf> {
    let mut cur = start.to_path_buf();
    loop {
        if cur.join("minc.toml").is_file() {
            return Some(cur);
        }
        if !cur.pop() {
            return None;
        }
    }
}

pub fn load_project(root: &Path) -> Result<LoadedProject, Vec<Diagnostic>> {
    let toml_path = root.join("minc.toml");
    let text = fs::read_to_string(&toml_path).map_err(|e| {
        vec![Diagnostic::new(
            0..0,
            format!("cannot read {}: {e}", toml_path.display()),
        )]
    })?;
    let config = parse_minc_toml(&text).map_err(|e| vec![Diagnostic::new(0..0, e)])?;
    let src = root.join("src");
    let mut paths = Vec::new();
    collect_mcs(&src, &mut paths);
    paths.sort();
    if paths.is_empty() {
        return Err(vec![Diagnostic::new(
            0..0,
            format!("no .mcs files under {}", src.display()),
        )]);
    }
    let mut files = Vec::new();
    let mut errors = Vec::new();
    for path in paths {
        let source = fs::read_to_string(&path).map_err(|e| {
            vec![Diagnostic::new(
                0..0,
                format!("cannot read {}: {e}", path.display()),
            )]
        })?;
        match parse(&source) {
            Ok(mut unit) => {
                unit.file = Some(path.display().to_string());
                let pack = unit.pack.dotted();
                if pack != config.pack && !pack.starts_with(&format!("{}.", config.pack)) {
                    errors.push(Diagnostic {
                        span: unit.pack_span.clone(),
                        message: format!(
                            "pack `{pack}` must equal `{}` or a subpackage of it",
                            config.pack
                        ),
                        file: Some(path.display().to_string()),
                    });
                }
                files.push(ParsedFile { path, source, unit });
            }
            Err(mut diags) => {
                for d in &mut diags {
                    d.file = Some(path.display().to_string());
                }
                errors.extend(diags);
            }
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(LoadedProject {
        root: root.to_path_buf(),
        config,
        files,
    })
}

pub fn load_and_merge(
    root: &Path,
) -> Result<(MincConfig, crate::ast::CompilationUnit), Vec<Diagnostic>> {
    let loaded = load_project(root)?;
    let unit = merge_units(&loaded.files)?;
    Ok((loaded.config, unit))
}

fn collect_mcs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_mcs(&path, out);
        } else if path.extension().and_then(|s| s.to_str()) == Some("mcs") {
            out.push(path);
        }
    }
}

pub fn scaffold(dir: &Path, edition: &str, version: &str) -> Result<(), String> {
    fs::create_dir_all(dir.join("src/chains")).map_err(|e| e.to_string())?;
    fs::create_dir_all(dir.join("src/logic")).map_err(|e| e.to_string())?;
    fs::create_dir_all(dir.join("src/world")).map_err(|e| e.to_string())?;
    let name = dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("game")
        .to_string();
    let pack = name.replace('-', ".");
    let edition = crate::config::Edition::parse(edition)
        .ok_or_else(|| format!("unknown edition `{edition}`"))?;
    let cfg = MincConfig {
        name: name.clone(),
        pack: pack.clone(),
        edition,
        game_version: version.to_string(),
        clock: Some("Main".into()),
        ..MincConfig::default()
    };
    cfg.validate()?;
    fs::write(dir.join("minc.toml"), crate::config::render_minc_toml(&cfg))
        .map_err(|e| e.to_string())?;
    fs::write(dir.join(".gitignore"), "dist/\n").map_err(|e| e.to_string())?;
    let tick_host = match cfg.edition {
        crate::config::Edition::Bedrock => "functions",
        crate::config::Edition::Java => "functions",
    };
    fs::write(
        dir.join("src/controller.mcs"),
        format!(
            r#"pack {pack};

@Controller
world {world} {{
    origin (0, 64, 0);
    dimension overworld;
    chain Main at (0, 64, 0) layout stack facing up;
    clock Main;
    host tick = {tick_host};
}}
"#,
            world = pascal(&name),
        ),
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        dir.join("src/chains/Main.chain.mcs"),
        format!(
            r#"pack {pack};

@Chain("Main")
@Repeat
@AlwaysActive
public chain Main {{
}}
"#
        ),
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        dir.join("src/logic/World.mcs"),
        format!(
            r#"pack {pack};

public class WorldState {{
    public static int tick;
}}
"#
        ),
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn pascal(name: &str) -> String {
    name.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut c = s.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

pub fn rewrite_layout(
    controller: &str,
    chain: &str,
    layout: &str,
    origin: [i32; 3],
    facing: &str,
) -> String {
    let mut out = String::new();
    let mut replaced = false;
    for line in controller.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("chain ") && trimmed[6..].starts_with(chain) {
            let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();
            out.push_str(&format!(
                "{indent}chain {chain} at ({}, {}, {}) layout {layout} facing {facing};\n",
                origin[0], origin[1], origin[2]
            ));
            replaced = true;
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    if !replaced {
        out.push_str(&format!(
            "    chain {chain} at ({}, {}, {}) layout {layout} facing {facing};\n",
            origin[0], origin[1], origin[2]
        ));
    }
    out
}
