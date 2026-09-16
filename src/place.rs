//! Placer: install MINCB into a datapack / behavior pack in spec order.
//!
//! 1. objectives (`OBJT`)
//! 2. `forceload` / `tickingarea` (`META`)
//! 3. `WBLK` then `CONT` then `CBLK` (command blocks last)
//! 4. copy `FUNC` into the pack folder (handled by `compile::write_artifacts`)
//! 5. `tick.json` / datapack tick tag
//! 6. do not run gameplay chains until load / `@OnLoad`

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::config::Edition;
use crate::mincb::{facing_name, CblkRec, MincbImage};

/// Relative pack paths → function bodies for the installer.
pub fn install_functions(image: &MincbImage) -> BTreeMap<String, String> {
    let prefix = match image.edition {
        Edition::Bedrock => image.pack.replace('.', "/"),
        Edition::Java => image.pack.clone(),
    };
    let path = |name: &str| -> String {
        match image.edition {
            Edition::Bedrock => format!("{prefix}/install/{name}"),
            Edition::Java => format!("{prefix}:install/{name}"),
        }
    };

    let obj = join(&objectives(image));
    let chunks = join(&chunks(image));
    let world = join(&world_blocks(image));
    let cont = join(&containers(image));
    let cblk = join(&command_blocks(image));

    let install_body = match image.edition {
        Edition::Bedrock => format!(
            "function {}\nfunction {}\nfunction {}\nfunction {}\nfunction {}\n",
            path("01_objectives"),
            path("02_chunks"),
            path("03_world"),
            path("04_containers"),
            path("05_command_blocks"),
        ),
        Edition::Java => format!(
            "function {}\nfunction {}\nfunction {}\nfunction {}\nfunction {}\n",
            path("01_objectives"),
            path("02_chunks"),
            path("03_world"),
            path("04_containers"),
            path("05_command_blocks"),
        ),
    };

    let mut files = BTreeMap::new();
    files.insert(path("01_objectives"), obj);
    files.insert(path("02_chunks"), chunks);
    files.insert(path("03_world"), world);
    files.insert(path("04_containers"), cont);
    files.insert(path("05_command_blocks"), cblk);
    files.insert(
        match image.edition {
            Edition::Bedrock => format!("{prefix}/install"),
            Edition::Java => format!("{prefix}:install"),
        },
        install_body,
    );
    files
}

pub fn write_install_tree(
    out_dir: &std::path::Path,
    image: &MincbImage,
) -> Result<BTreeMap<PathBuf, String>, String> {
    let fns = install_functions(image);
    let mut written = BTreeMap::new();
    for (path, body) in &fns {
        let file = match image.edition {
            Edition::Bedrock => out_dir.join(format!("functions/{path}.mcfunction")),
            Edition::Java => {
                let (ns, rest) = path
                    .split_once(':')
                    .unwrap_or((image.pack.as_str(), path.as_str()));
                out_dir.join(format!("data/{ns}/functions/{rest}.mcfunction"))
            }
        };
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&file, body).map_err(|e| e.to_string())?;
        written.insert(file, body.clone());
    }
    crate::structure::write_structure_files(out_dir, image)?;
    Ok(written)
}

fn join(cmds: &[String]) -> String {
    let mut s = cmds.join("\n");
    if !s.is_empty() && !s.ends_with('\n') {
        s.push('\n');
    }
    s
}

fn objectives(image: &MincbImage) -> Vec<String> {
    let mut out = Vec::new();
    for o in &image.objectives {
        let name = image
            .symbols
            .iter()
            .find(|s| s.id == o.symb)
            .map(|s| s.short.as_str())
            .unwrap_or("obj");
        out.push(format!("scoreboard objectives add {name} dummy"));
    }
    out
}

fn chunks(image: &MincbImage) -> Vec<String> {
    let mut out = Vec::new();
    for area in parse_ticking_areas(&image.meta_json) {
        match image.edition {
            Edition::Bedrock => {
                let mut cmd = format!(
                    "tickingarea add circle {} {} {} {} {}",
                    area.center[0], area.center[1], area.center[2], area.radius, area.name
                );
                if area.preload {
                    cmd.push_str(" true");
                }
                out.push(cmd);
            }
            Edition::Java => {
                let r = area.radius.max(0);
                let min_cx = (area.center[0] - r) >> 4;
                let max_cx = (area.center[0] + r) >> 4;
                let min_cz = (area.center[2] - r) >> 4;
                let max_cz = (area.center[2] + r) >> 4;
                out.push(format!("forceload add {min_cx} {min_cz} {max_cx} {max_cz}"));
            }
        }
    }
    out
}

struct Area {
    name: String,
    center: [i64; 3],
    radius: i64,
    preload: bool,
}

fn parse_ticking_areas(meta: &str) -> Vec<Area> {
    let mut out = Vec::new();
    let Some(idx) = meta.find("\"ticking_areas\"") else {
        return out;
    };
    let rest = &meta[idx..];
    let Some(start) = rest.find('[') else {
        return out;
    };
    let rest = &rest[start + 1..];
    for obj in rest.split("},{") {
        let name = json_str(obj, "name").unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        let center = json_i3(obj, "center").unwrap_or([0, 0, 0]);
        let radius = json_i(obj, "radius").unwrap_or(0);
        let preload = obj.contains("\"preload\":true");
        out.push(Area {
            name,
            center,
            radius,
            preload,
        });
    }
    out
}

struct Fill {
    from: [i64; 3],
    to: [i64; 3],
    block: String,
    replace: String,
}

fn parse_fills(meta: &str) -> Vec<Fill> {
    let Some(idx) = meta.find("\"fills\"") else {
        return Vec::new();
    };
    let rest = &meta[idx..];
    let Some(start) = rest.find('[') else {
        return Vec::new();
    };
    let mut depth = 0i32;
    let mut end = None;
    for (i, ch) in rest.char_indices().skip(start) {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    let Some(end) = end else {
        return Vec::new();
    };
    let arr = &rest[start..=end];
    let mut out = Vec::new();
    let chars: Vec<char> = arr.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '{' {
            let mut d = 1;
            let from = i;
            i += 1;
            while i < chars.len() && d > 0 {
                if chars[i] == '{' {
                    d += 1;
                } else if chars[i] == '}' {
                    d -= 1;
                }
                i += 1;
            }
            let obj: String = chars[from..i].iter().collect();
            let block = json_str(&obj, "block").unwrap_or_default();
            if !block.is_empty() {
                out.push(Fill {
                    from: json_i3(&obj, "from").unwrap_or([0, 0, 0]),
                    to: json_i3(&obj, "to").unwrap_or([0, 0, 0]),
                    block,
                    replace: json_str(&obj, "replace").unwrap_or_default(),
                });
            }
        } else {
            i += 1;
        }
    }
    out
}

fn json_str(obj: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\":\"");
    let i = obj.find(&pat)?;
    let rest = &obj[i + pat.len()..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn json_i(obj: &str, key: &str) -> Option<i64> {
    let pat = format!("\"{key}\":");
    let i = obj.find(&pat)?;
    let rest = obj[i + pat.len()..].trim_start();
    let num: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '-')
        .collect();
    num.parse().ok()
}

fn json_i3(obj: &str, key: &str) -> Option<[i64; 3]> {
    let pat = format!("\"{key}\":[");
    let i = obj.find(&pat)?;
    let rest = &obj[i + pat.len()..];
    let end = rest.find(']')?;
    let nums: Vec<i64> = rest[..end]
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    if nums.len() == 3 {
        Some([nums[0], nums[1], nums[2]])
    } else {
        None
    }
}

fn world_blocks(image: &MincbImage) -> Vec<String> {
    let mut out = Vec::new();
    for f in parse_fills(&image.meta_json) {
        let mut cmd = format!(
            "fill {} {} {} {} {} {} {}",
            f.from[0], f.from[1], f.from[2], f.to[0], f.to[1], f.to[2], f.block
        );
        if !f.replace.is_empty() {
            cmd.push_str(" replace ");
            cmd.push_str(&f.replace);
        }
        out.push(cmd);
    }
    for b in &image.world_blocks {
        out.push(format!("setblock {} {} {} {}", b.x, b.y, b.z, b.block));
    }
    out
}

fn containers(image: &MincbImage) -> Vec<String> {
    let mut out = Vec::new();
    for c in &image.containers {
        let face = facing_name(c.facing);
        match image.edition {
            Edition::Bedrock => {
                out.push(format!(
                    "setblock {} {} {} {} [\"facing_direction\"={}]",
                    c.x, c.y, c.z, c.block, c.facing
                ));
            }
            Edition::Java => {
                out.push(format!(
                    "setblock {} {} {} {}[facing={face}]",
                    c.x, c.y, c.z, c.block
                ));
            }
        }
        for s in &c.slots {
            match image.edition {
                Edition::Bedrock => {
                    let mut cmd = format!(
                        "replaceitem block {} {} {} slot.container.{} {} {}",
                        c.x, c.y, c.z, s.slot, s.count, s.item
                    );
                    if s.data != 0 {
                        cmd.push(' ');
                        cmd.push_str(&s.data.to_string());
                    }
                    out.push(cmd);
                }
                Edition::Java => {
                    out.push(format!(
                        "item replace block {} {} {} container.{} with {} {}",
                        c.x, c.y, c.z, s.slot, s.item, s.count
                    ));
                }
            }
        }
    }
    out
}

fn command_blocks(image: &MincbImage) -> Vec<String> {
    image
        .command_blocks
        .iter()
        .map(|b| set_command_block(image.edition, b))
        .collect()
}

fn set_command_block(edition: Edition, b: &CblkRec) -> String {
    let kind = match b.mode {
        2 => "repeating_command_block",
        1 => "chain_command_block",
        _ => "command_block",
    };
    let face = facing_name(b.facing);
    let auto = (b.flags & 2) != 0;
    let cond = (b.flags & 1) != 0;
    let cmd = escape_cmd(&b.command);
    match edition {
        Edition::Java => {
            format!(
                "setblock {} {} {} {kind}[facing={face},conditional={}]{{Command:\"{cmd}\",auto:{},TrackOutput:0b}}",
                b.x,
                b.y,
                b.z,
                if cond { "true" } else { "false" },
                if auto { "1b" } else { "0b" },
            )
        }
        Edition::Bedrock => {
            // Bedrock setblock cannot write Command NBT; still place the block type
            // so a later world edit / structure load can attach the MINCB string.
            format!(
                "setblock {} {} {} {kind} [\"facing_direction\"={}]",
                b.x, b.y, b.z, b.facing
            )
        }
    }
}

fn escape_cmd(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
