//! `minc` CLI: new / check / build / inspect / dump / layout.

use std::fs;
use std::path::{Path, PathBuf};

use crate::compile::{compile_unit, write_artifacts};
use crate::config::parse_minc_toml;
use crate::diagnostic::eprint_diagnostics;
use crate::mincb::{self, decode_header};
use crate::project::{find_root, load_and_merge, load_project, rewrite_layout, scaffold};

pub fn run(args: Vec<String>) -> i32 {
    let mut it = args.into_iter();
    let _bin = it.next();
    let Some(cmd) = it.next() else {
        print_usage();
        return 2;
    };
    match cmd.as_str() {
        "new" => cmd_new(&mut it),
        "check" => cmd_check(&mut it),
        "build" => cmd_build(&mut it),
        "inspect" => cmd_inspect(&mut it),
        "dump" => cmd_dump(&mut it),
        "layout" => cmd_layout(&mut it),
        "-h" | "--help" | "help" => {
            print_usage();
            0
        }
        other if other.ends_with(".mcs") => {
            // One-file parse dump (syntax validation).
            cmd_parse_file(Path::new(other))
        }
        _ => {
            eprintln!("unknown command `{cmd}`");
            print_usage();
            2
        }
    }
}

fn print_usage() {
    eprintln!(
        "\
minc — MincScript compiler

  minc new <dir> --edition bedrock|java --version <ver>
  minc check [dir]
  minc build [--out dist/] [--emit functions-only]
  minc inspect <file.mincb>
  minc dump <file.mincb|--project> --commands
  minc layout <chain> --layout stack --origin x y z --facing up
"
    );
}

fn cmd_new(it: &mut impl Iterator<Item = String>) -> i32 {
    let mut dir = None;
    let mut edition = "bedrock".to_string();
    let mut version = "1.21.70".to_string();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--edition" => {
                if let Some(v) = it.next() {
                    edition = v;
                }
            }
            "--version" => {
                if let Some(v) = it.next() {
                    version = v;
                }
            }
            s if !s.starts_with('-') => dir = Some(a),
            other => {
                eprintln!("unknown flag `{other}`");
                return 2;
            }
        }
    }
    let Some(dir) = dir else {
        eprintln!("minc new <dir> --edition bedrock|java --version <ver>");
        return 2;
    };
    let path = PathBuf::from(&dir);
    if let Err(e) = scaffold(&path, &edition, &version) {
        eprintln!("{e}");
        return 1;
    }
    println!("created {}", path.display());
    0
}

fn cmd_check(it: &mut impl Iterator<Item = String>) -> i32 {
    let dir = it
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    if dir.is_file() && dir.extension().and_then(|s| s.to_str()) == Some("mcs") {
        return cmd_parse_file(&dir);
    }
    let root = if dir.join("minc.toml").is_file() {
        dir
    } else {
        match find_root(&dir) {
            Some(r) => r,
            None => {
                eprintln!("missing minc.toml (exit 2)");
                return 2;
            }
        }
    };
    match load_and_merge(&root) {
        Ok((config, unit)) => match compile_unit(&unit, &config) {
            Ok(_) => {
                println!(
                    "ok pack={} edition={}",
                    config.pack,
                    config.edition.as_str()
                );
                0
            }
            Err(diags) => {
                print_project_errors(&root, &diags);
                1
            }
        },
        Err(diags) => {
            print_project_errors(&root, &diags);
            1
        }
    }
}

fn cmd_build(it: &mut impl Iterator<Item = String>) -> i32 {
    let mut out = PathBuf::from("dist");
    let mut functions_only = false;
    let mut root = PathBuf::from(".");
    while let Some(a) = it.next() {
        match a.as_str() {
            "--out" => {
                if let Some(v) = it.next() {
                    out = PathBuf::from(v);
                }
            }
            "--emit" => {
                if let Some(v) = it.next() {
                    functions_only = v == "functions-only";
                }
            }
            s if !s.starts_with('-') => root = PathBuf::from(a),
            other => {
                eprintln!("unknown flag `{other}`");
                return 2;
            }
        }
    }
    let root = if root.join("minc.toml").is_file() {
        root
    } else {
        match find_root(&root) {
            Some(r) => r,
            None => {
                eprintln!("missing minc.toml");
                return 2;
            }
        }
    };
    match load_and_merge(&root) {
        Ok((mut config, unit)) => {
            if functions_only {
                config.emit_mincb = false;
                config.emit_pack = false;
                config.emit_functions = true;
            }
            match compile_unit(&unit, &config) {
                Ok(art) => {
                    let out_dir = if out.is_absolute() {
                        out
                    } else {
                        root.join(&out)
                    };
                    if let Err(e) = write_artifacts(&root, &out_dir, &config, &art) {
                        eprintln!("{e}");
                        return 2;
                    }
                    println!("wrote {}", out_dir.display());
                    0
                }
                Err(diags) => {
                    print_project_errors(&root, &diags);
                    1
                }
            }
        }
        Err(diags) => {
            print_project_errors(&root, &diags);
            1
        }
    }
}

fn cmd_inspect(it: &mut impl Iterator<Item = String>) -> i32 {
    let Some(path) = it.next() else {
        eprintln!("minc inspect <file.mincb>");
        return 2;
    };
    let bytes = match fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    if let Some(h) = decode_header(&bytes) {
        if let Ok(toml) = fs::read_to_string("minc.toml") {
            if let Ok(cfg) = parse_minc_toml(&toml) {
                let want = cfg.edition.byte();
                if h.edition != want {
                    eprintln!(
                        "MINCB edition {} does not match minc.toml {}",
                        h.edition,
                        cfg.edition.as_str()
                    );
                    return 3;
                }
            }
        }
    }
    print!("{}", mincb::inspect_text(&bytes, None, &[]));
    0
}

fn cmd_dump(it: &mut impl Iterator<Item = String>) -> i32 {
    let mut commands = false;
    let mut path: Option<PathBuf> = None;
    for a in it {
        match a.as_str() {
            "--commands" => commands = true,
            "--project" => path = None,
            s if !s.starts_with('-') => path = Some(PathBuf::from(a)),
            _ => {}
        }
    }
    if !commands {
        eprintln!("minc dump --commands");
        return 2;
    }
    if let Some(p) = path {
        if p.extension().and_then(|s| s.to_str()) == Some("mincb") {
            eprintln!("binary dump of command strings requires a project; compiling cwd");
        }
    }
    let root = match find_root(&PathBuf::from(".")) {
        Some(r) => r,
        None => {
            eprintln!("missing minc.toml");
            return 2;
        }
    };
    match load_and_merge(&root) {
        Ok((config, unit)) => match compile_unit(&unit, &config) {
            Ok(art) => {
                print!("{}", crate::lower::dump_commands(&art.lowered));
                0
            }
            Err(diags) => {
                print_project_errors(&root, &diags);
                1
            }
        },
        Err(diags) => {
            print_project_errors(&root, &diags);
            1
        }
    }
}

fn cmd_layout(it: &mut impl Iterator<Item = String>) -> i32 {
    let Some(chain) = it.next() else {
        eprintln!("minc layout <chain> --layout stack --origin x y z --facing up");
        return 2;
    };
    let mut layout = "stack".to_string();
    let mut origin = [0, 64, 0];
    let mut facing = "up".to_string();
    let args: Vec<String> = it.collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--layout" => {
                i += 1;
                if i < args.len() {
                    layout = args[i].clone();
                }
            }
            "--facing" => {
                i += 1;
                if i < args.len() {
                    facing = args[i].clone();
                }
            }
            "--origin" => {
                origin = [0, 0, 0].map(|_| {
                    i += 1;
                    args.get(i).and_then(|s| s.parse().ok()).unwrap_or(0)
                });
            }
            "--relative" => {}
            _ => {}
        }
        i += 1;
    }
    let root = match find_root(&PathBuf::from(".")) {
        Some(r) => r,
        None => {
            eprintln!("missing minc.toml");
            return 2;
        }
    };
    let controller = root.join("src/controller.mcs");
    let src = match fs::read_to_string(&controller) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    let next = rewrite_layout(&src, &chain, &layout, origin, &facing);
    if let Err(e) = fs::write(&controller, next) {
        eprintln!("{e}");
        return 2;
    }
    println!("updated {}", controller.display());
    0
}

fn cmd_parse_file(path: &Path) -> i32 {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    match crate::parse(&source) {
        Ok(unit) => {
            print!("{unit}");
            let info = crate::extract_binary_info(&unit);
            print!("{}", info.summary());
            0
        }
        Err(diags) => {
            eprint_diagnostics(&path.display().to_string(), &source, &diags);
            1
        }
    }
}

fn print_project_errors(root: &Path, diags: &[crate::Diagnostic]) {
    let loaded = load_project(root);
    match loaded {
        Ok(proj) => {
            for d in diags {
                if let Some(file) = &d.file {
                    if let Some(f) = proj
                        .files
                        .iter()
                        .find(|f| f.path.display().to_string() == *file)
                    {
                        eprint_diagnostics(file, &f.source, std::slice::from_ref(d));
                        continue;
                    }
                }
                eprint_diagnostics(&root.display().to_string(), "", std::slice::from_ref(d));
            }
        }
        Err(_) => {
            for d in diags {
                let name = d.file.as_deref().unwrap_or("minc");
                eprint_diagnostics(name, "", std::slice::from_ref(d));
            }
        }
    }
}
