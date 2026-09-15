//! Project-wide compile: parse → check → extract → lower → MINCB + functions.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::ast::{CompilationUnit, Item};
use crate::config::MincConfig;
use crate::diagnostic::Diagnostic;
use crate::extract::{extract_binary_info_with_revision, BinaryInfo};
use crate::isa;
use crate::layout::{apply_prefer, place_commands, CbInstance};
use crate::lower::{self, Lowered};
use crate::mincb::{self, CblkRec, FuncRec, SymbRec};
use crate::place;
use crate::sema;

#[derive(Debug, Clone)]
pub struct ParsedFile {
    pub path: PathBuf,
    pub source: String,
    pub unit: CompilationUnit,
}

#[derive(Debug, Clone)]
pub struct Artifacts {
    pub info: BinaryInfo,
    pub lowered: Lowered,
    pub mincb: Vec<u8>,
    pub functions: BTreeMap<String, String>,
    pub tick_json: Option<String>,
    pub load_json: Option<String>,
    pub command_blocks: Vec<CbInstance>,
    pub inspect: String,
    pub pack_files: BTreeMap<PathBuf, String>,
    pub warnings: Vec<String>,
    pub image: mincb::MincbImage,
}

pub fn merge_units(files: &[ParsedFile]) -> Result<CompilationUnit, Vec<Diagnostic>> {
    if files.is_empty() {
        return Err(vec![Diagnostic::new(0..0, "no source files")]);
    }
    let mut pack = files[0].unit.pack.clone();
    let mut imports = Vec::new();
    let mut items = Vec::new();
    for file in files {
        let name = file.path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let stem = file.path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        if name == "controller.mcs" {
            let worlds = file
                .unit
                .items
                .iter()
                .filter(|i| matches!(i, Item::World(_)))
                .count();
            if worlds != 1 {
                return Err(vec![Diagnostic::new(
                    file.unit.pack_span.clone(),
                    format!(
                        "{}: controller must contain exactly one world",
                        file.path.display()
                    ),
                )]);
            }
        }
        if name.ends_with(".chain.mcs") || stem.ends_with(".chain") {
            let chains: Vec<_> = file
                .unit
                .items
                .iter()
                .filter_map(|i| match i {
                    Item::Chain(c) => Some(c.name.as_str()),
                    _ => None,
                })
                .collect();
            let expect = stem.trim_end_matches(".chain");
            if chains.len() != 1 || chains[0] != expect {
                return Err(vec![Diagnostic::new(
                    file.unit.pack_span.clone(),
                    format!(
                        "{}: expected exactly one `chain {expect}`",
                        file.path.display()
                    ),
                )]);
            }
            if file.unit.items.iter().any(|i| matches!(i, Item::Class(_))) {
                return Err(vec![Diagnostic::new(
                    file.unit.pack_span.clone(),
                    format!(
                        "{}: chain files cannot declare classes",
                        file.path.display()
                    ),
                )]);
            }
        }
        if file.unit.pack.dotted().len() > pack.dotted().len()
            && !file.unit.pack.dotted().starts_with(&pack.dotted())
        {
            pack = file.unit.pack.clone();
        }
        imports.extend(file.unit.imports.clone());
        items.extend(file.unit.items.clone());
    }
    Ok(CompilationUnit {
        pack: files[0].unit.pack.clone(),
        pack_span: files[0].unit.pack_span.clone(),
        file: files[0].unit.file.clone(),
        imports,
        items,
    })
}

pub fn compile_unit(
    unit: &CompilationUnit,
    config: &MincConfig,
) -> Result<Artifacts, Vec<Diagnostic>> {
    let errors = sema::check_with_config(unit, Some(config));
    if !errors.is_empty() {
        return Err(errors);
    }
    let info = extract_binary_info_with_revision(unit, config.score_revision);
    let lowered = lower::lower_project(unit, config, &info)?;
    let mut warnings = Vec::new();
    let mut command_blocks = Vec::new();
    let mut chain_lengths: Vec<(String, u16)> = Vec::new();
    let host = info.host_tick.as_deref().unwrap_or("functions");
    let functions_host = host.contains("function") || host == "both" || host.is_empty();

    for chain in &lowered.chains {
        let (origin, facing, layout) = lower::resolve_chain_origin(&info, config, &chain.name);
        let clock = info
            .chains
            .iter()
            .find(|c| c.name == chain.name)
            .map(|c| c.is_clock)
            .unwrap_or(false)
            || config.clock.as_deref() == Some(chain.name.as_str());
        let bound = info
            .chains
            .iter()
            .find(|c| c.name == chain.name)
            .and_then(|c| c.bound)
            .map(|b| [b[0] as i32, b[1] as i32, b[2] as i32]);
        let pack_mode = info
            .chains
            .iter()
            .find(|c| c.name == chain.name)
            .and_then(|c| c.pack_mode.clone())
            .unwrap_or_else(|| config.chain_pack.clone());
        let cmds_for_cb: Vec<(String, crate::layout::ChainCmdMeta)> = if pack_mode == "function" {
            let path = chain_fn_path(config, &chain.name);
            let meta = chain
                .commands
                .first()
                .map(|(_, m)| {
                    let mut m = m.clone();
                    m.repeat = clock;
                    m
                })
                .unwrap_or_default();
            vec![(format!("function {path}"), meta)]
        } else {
            apply_prefer(&config.prefer, chain.commands.clone())
        };
        let placed = place_commands(
            origin,
            &layout,
            facing,
            config.max_span,
            bound,
            &cmds_for_cb,
            clock,
        )
        .map_err(|e| vec![Diagnostic::new(unit.pack_span.clone(), e)])?;
        if placed.len() > 100 {
            warnings.push(format!(
                "chain `{}` length {} > 100 (hard to see in Creative)",
                chain.name,
                placed.len()
            ));
        }
        if chain.commands.len() as u32 > config.function_command_limit {
            return Err(vec![Diagnostic::new(
                unit.pack_span.clone(),
                format!(
                    "chain `{}` exceeds limits.function_command_limit",
                    chain.name
                ),
            )]);
        }
        chain_lengths.push((chain.name.clone(), placed.len() as u16));
        command_blocks.extend(placed);
    }

    let mut functions = BTreeMap::new();
    for f in &lowered.functions {
        functions.insert(f.path.clone(), join_cmds(&f.commands));
    }
    for chain in &lowered.chains {
        let path = chain_fn_path(config, &chain.name);
        let body = chain
            .commands
            .iter()
            .map(|(c, _)| c.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        functions.insert(path, body);
    }

    let mut tick_json = None;
    let mut load_json = if lowered.load_paths.is_empty() {
        None
    } else {
        Some(tick_json_body(config, &lowered.load_paths))
    };
    let mut meta_ticks: Vec<String> = Vec::new();

    if config.emit_functions {
        match config.edition {
            crate::config::Edition::Bedrock => {
                let boot = format!("{}/_boot/tick", config.bedrock_fn_prefix());
                let onload_path = format!("{}/_boot/onload", config.bedrock_fn_prefix());
                let mut onload = lowered.setup.clone();
                for p in &lowered.load_paths {
                    onload.push(format!("function {p}"));
                }
                functions.insert(onload_path.clone(), join_cmds(&onload));
                let mut tick_body = vec![format!(
                    "execute unless score {} mi matches 1.. run function {onload_path}",
                    config.edition.world_holder()
                )];
                tick_body.push(format!(
                    "scoreboard players set {} mi 1",
                    config.edition.world_holder()
                ));
                for p in &lowered.tick_paths {
                    tick_body.push(format!("function {p}"));
                }
                if functions_host {
                    if let Some(clock) = info.clock.as_ref().or(config.clock.as_ref()) {
                        let p = chain_fn_path(config, clock);
                        tick_body.push(format!("function {p}"));
                    }
                }
                functions.insert(boot.clone(), join_cmds(&tick_body));
                meta_ticks.push(boot.clone());
                tick_json = Some(tick_json_body(config, &[boot]));
                load_json = Some(tick_json_body(
                    config,
                    &[format!("{}/_boot/onload", config.bedrock_fn_prefix())],
                ));
            }
            crate::config::Edition::Java => {
                let mut values = lowered.tick_paths.clone();
                if functions_host {
                    if let Some(clock) = info.clock.as_ref().or(config.clock.as_ref()) {
                        let p = chain_fn_path(config, clock);
                        if !values.contains(&p) {
                            values.push(p);
                        }
                    }
                }
                meta_ticks = values.clone();
                if !values.is_empty() {
                    tick_json = Some(tick_json_body(config, &values));
                }
            }
        }
    }

    let mut image = mincb::image_from_extract(&info, config.edition, &config.game_version);
    if let Some(world) = info.origin {
        image.origin = [world[0] as i32, world[1] as i32, world[2] as i32];
    } else {
        image.origin = config.origin;
    }
    intern_fn_symbols(&mut image, &functions);
    let func_recs: Vec<FuncRec> = functions
        .iter()
        .map(|(path, body)| {
            let symb = image
                .symbols
                .iter()
                .find(|s| s.qualified.ends_with(path) || s.short == *path)
                .map(|s| s.id)
                .unwrap_or(0);
            FuncRec {
                symb,
                path: path.clone(),
                body: body.clone(),
            }
        })
        .collect();
    image.functions = func_recs;
    image.command_blocks = command_blocks
        .iter()
        .map(|b| CblkRec {
            x: b.x,
            y: b.y,
            z: b.z,
            facing: b.facing.as_u8(),
            mode: b.mode,
            flags: b.flags,
            delay_ticks: b.delay_ticks,
            command: b.command.clone(),
        })
        .collect();
    let mut cursor = 0u16;
    for (name, len) in &chain_lengths {
        let want = image
            .symbols
            .iter()
            .find(|s| s.qualified.ends_with(&format!(".chain.{name}")) || s.short == *name)
            .map(|s| s.id);
        let pack_mode = info
            .chains
            .iter()
            .find(|c| c.name == *name)
            .and_then(|c| c.pack_mode.as_deref())
            .unwrap_or(config.chain_pack.as_str());
        if let Some(id) = want {
            if let Some(ch) = image.chains.iter_mut().find(|c| c.name_symb == id) {
                ch.length = *len;
                ch.pack_mode = u8::from(pack_mode == "function");
            }
        }
        cursor = cursor.saturating_add(*len);
    }
    let _ = cursor;

    let install = place::install_functions(&image);
    for (path, body) in &install {
        functions.insert(path.clone(), body.clone());
    }
    intern_fn_symbols(&mut image, &functions);
    let func_recs: Vec<FuncRec> = functions
        .iter()
        .map(|(path, body)| FuncRec {
            symb: image
                .symbols
                .iter()
                .find(|s| s.qualified.ends_with(path) || s.short == *path)
                .map(|s| s.id)
                .unwrap_or(0),
            path: path.clone(),
            body: body.clone(),
        })
        .collect();
    image.functions = func_recs;
    image.tick_values = meta_ticks
        .iter()
        .filter_map(|p| {
            image
                .symbols
                .iter()
                .find(|s| s.short == *p || s.qualified.ends_with(p))
                .map(|s| s.id)
        })
        .collect();

    let mut isa_errors = Vec::new();
    for body in functions.values() {
        for line in body.lines() {
            if let Err(e) = isa::check_command(config.edition, line) {
                isa_errors.push(e);
            }
        }
    }
    for b in &command_blocks {
        if let Err(e) = isa::check_command(config.edition, &b.command) {
            isa_errors.push(e);
        }
    }
    isa_errors.sort();
    isa_errors.dedup();
    if !isa_errors.is_empty() {
        return Err(isa_errors
            .into_iter()
            .map(|e| Diagnostic::new(unit.pack_span.clone(), e))
            .collect());
    }

    let mincb_bytes = mincb::encode_image(&image);
    let inspect = mincb::inspect_text(&mincb_bytes, Some(&info), &command_blocks);
    let pack_files = emit_pack_files(
        config,
        &functions,
        tick_json.as_deref(),
        load_json.as_deref(),
    );

    Ok(Artifacts {
        info,
        lowered,
        mincb: mincb_bytes,
        functions,
        tick_json,
        load_json,
        command_blocks,
        inspect,
        pack_files,
        warnings,
        image,
    })
}

fn chain_fn_path(config: &MincConfig, name: &str) -> String {
    match config.edition {
        crate::config::Edition::Bedrock => {
            format!(
                "{}/chain/{}",
                config.bedrock_fn_prefix(),
                name.to_lowercase()
            )
        }
        crate::config::Edition::Java => {
            format!("{}:chain/{}", config.java_namespace(), name.to_lowercase())
        }
    }
}

fn intern_fn_symbols(image: &mut mincb::MincbImage, functions: &BTreeMap<String, String>) {
    let mut next = image.symbols.iter().map(|s| s.id).max().unwrap_or(0);
    for path in functions.keys() {
        if image
            .symbols
            .iter()
            .any(|s| s.qualified.ends_with(path) || s.short == *path)
        {
            continue;
        }
        next += 1;
        image.symbols.push(SymbRec {
            id: next,
            kind: 3,
            short: path.clone(),
            qualified: format!("{}.{}", image.pack, path),
        });
    }
}

fn join_cmds(cmds: &[String]) -> String {
    let mut s = cmds.join("\n");
    if !s.is_empty() && !s.ends_with('\n') {
        s.push('\n');
    }
    s
}

fn tick_json_body(config: &MincConfig, values: &[String]) -> String {
    let quoted: Vec<String> = values.iter().map(|v| format!("\"{v}\"")).collect();
    match config.edition {
        crate::config::Edition::Bedrock => {
            format!("{{\n  \"values\": [{}]\n}}\n", quoted.join(", "))
        }
        crate::config::Edition::Java => format!("{{\n  \"values\": [{}]\n}}\n", quoted.join(", ")),
    }
}

fn emit_pack_files(
    config: &MincConfig,
    functions: &BTreeMap<String, String>,
    tick: Option<&str>,
    load: Option<&str>,
) -> BTreeMap<PathBuf, String> {
    let mut files = BTreeMap::new();
    if !config.emit_pack && !config.emit_functions {
        return files;
    }
    match config.edition {
        crate::config::Edition::Bedrock => {
            files.insert(PathBuf::from("manifest.json"), bedrock_manifest(config));
            for (path, body) in functions {
                let rel = format!("functions/{path}.mcfunction");
                files.insert(PathBuf::from(rel), body.clone());
            }
            if let Some(tick) = tick {
                files.insert(PathBuf::from("functions/tick.json"), tick.to_string());
            }
            if let Some(load) = load {
                files.insert(PathBuf::from("functions/load.json"), load.to_string());
            }
        }
        crate::config::Edition::Java => {
            files.insert(
                PathBuf::from("pack.mcmeta"),
                format!(
                    "{{\n  \"pack\": {{\n    \"pack_format\": 48,\n    \"description\": \"{}\"\n  }}\n}}\n",
                    config.name
                ),
            );
            for (path, body) in functions {
                let (ns, rest) = path
                    .split_once(':')
                    .unwrap_or((&config.pack, path.as_str()));
                let rel = format!("data/{ns}/functions/{rest}.mcfunction");
                files.insert(PathBuf::from(rel), body.clone());
            }
            if let Some(tick) = tick {
                files.insert(
                    PathBuf::from("data/minecraft/tags/functions/tick.json"),
                    tick.to_string(),
                );
            }
            if let Some(load) = load {
                files.insert(
                    PathBuf::from("data/minecraft/tags/functions/load.json"),
                    load.to_string(),
                );
            }
        }
    }
    files
}

fn bedrock_manifest(config: &MincConfig) -> String {
    let (maj, min, pat) =
        crate::config::parse_game_version(&config.game_version).unwrap_or((1, 21, 0));
    let u1 = uuid_like(&config.pack, 1);
    let u2 = uuid_like(&config.pack, 2);
    format!(
        r#"{{
  "format_version": 2,
  "header": {{
    "name": "{}",
    "description": "MincScript",
    "uuid": "{u1}",
    "version": [1, 0, 0],
    "min_engine_version": [{maj}, {min}, {pat}]
  }},
  "modules": [
    {{
      "type": "data",
      "uuid": "{u2}",
      "version": [1, 0, 0]
    }}
  ]
}}
"#,
        config.name
    )
}

fn uuid_like(s: &str, salt: u64) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    salt.hash(&mut h);
    let n = h.finish();
    let a = n & 0xffff_ffff;
    let b = (n >> 32) & 0xffff;
    let c = 0x4000 | ((n >> 48) & 0x0fff);
    let d = 0x8000 | (salt & 0x3fff);
    let e = n.wrapping_mul(0x9e3779b97f4a7c15);
    format!("{a:08x}-{b:04x}-{c:04x}-{d:04x}-{e:012x}")
}

pub fn write_artifacts(
    root: &Path,
    out_dir: &Path,
    config: &MincConfig,
    art: &Artifacts,
) -> Result<(), String> {
    std::fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    if config.emit_mincb {
        let path = out_dir.join(format!("{}.mincb", config.name));
        std::fs::write(&path, &art.mincb).map_err(|e| e.to_string())?;
    }
    if config.emit_functions {
        let fn_root = out_dir.join("functions");
        for (path, body) in &art.functions {
            let file = match config.edition {
                crate::config::Edition::Bedrock => fn_root.join(format!("{path}.mcfunction")),
                crate::config::Edition::Java => {
                    let (ns, rest) = path
                        .split_once(':')
                        .unwrap_or((&config.pack, path.as_str()));
                    out_dir
                        .join("datapack")
                        .join(format!("data/{ns}/functions/{rest}.mcfunction"))
                }
            };
            if let Some(parent) = file.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(file, body).map_err(|e| e.to_string())?;
        }
        if let Some(tick) = &art.tick_json {
            match config.edition {
                crate::config::Edition::Bedrock => {
                    std::fs::create_dir_all(fn_root.parent().unwrap_or(out_dir)).ok();
                    std::fs::create_dir_all(&fn_root).map_err(|e| e.to_string())?;
                    std::fs::write(fn_root.join("tick.json"), tick).map_err(|e| e.to_string())?;
                }
                crate::config::Edition::Java => {
                    let p = out_dir.join("datapack/data/minecraft/tags/functions");
                    std::fs::create_dir_all(&p).map_err(|e| e.to_string())?;
                    std::fs::write(p.join("tick.json"), tick).map_err(|e| e.to_string())?;
                }
            }
        }
    }
    if config.emit_pack {
        let pack_root = match config.edition {
            crate::config::Edition::Bedrock => out_dir.join("behavior_pack"),
            crate::config::Edition::Java => out_dir.join("datapack"),
        };
        for (rel, body) in &art.pack_files {
            let file = pack_root.join(rel);
            if let Some(parent) = file.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(file, body).map_err(|e| e.to_string())?;
        }
    }
    crate::structure::write_structure_files(out_dir, &art.image)?;
    let _ = root;
    Ok(())
}
