//! MincScript: Java-shaped language that compiles to Minecraft MINCB.

pub mod ast;
pub mod cli;
pub mod cmds;
pub mod compile;
pub mod config;
pub mod diagnostic;
pub mod extract;
pub mod isa;
pub mod items;
pub mod layout;
pub mod lexer;
pub mod lower;
pub mod mar;
pub mod mincb;
pub mod nbt;
pub mod opt;
pub mod parser;
pub mod place;
pub mod project;
pub mod sema;
pub mod span;
pub mod structure;
pub mod token;

pub use ast::CompilationUnit;
pub use diagnostic::{eprint_diagnostics, Diagnostic};
pub use extract::{extract_binary_info, BinaryInfo};
pub use lexer::tokenize;
pub use token::Token;

/// Lex and parse `source` into a [`CompilationUnit`].
pub fn parse(source: &str) -> Result<CompilationUnit, Vec<Diagnostic>> {
    let (tokens, lex_errors) = tokenize(source);
    if !lex_errors.is_empty() {
        return Err(lex_errors);
    }
    parser::parse_tokens(source, tokens)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Item, Member, TypeRef};
    use crate::extract::SymbolKind;

    #[test]
    fn parses_class_fields() {
        let src = r#"
pack metro.escape;

public class Runner {
    private int keys;
    private boolean extracted;
}
"#;
        let unit = parse(src).expect("should parse");
        assert_eq!(unit.pack.dotted(), "metro.escape");
        let Item::Class(class) = &unit.items[0] else {
            panic!("expected class");
        };
        assert_eq!(class.name, "Runner");
        assert_eq!(class.members.len(), 2);
        match &class.members[0] {
            Member::Field(f) => {
                assert_eq!(f.name, "keys");
                assert_eq!(f.ty, TypeRef::Int);
            }
            _ => panic!("expected field"),
        }
        let info = extract_binary_info(&unit);
        assert!(info
            .symbols
            .iter()
            .any(|s| s.kind == SymbolKind::Objective && s.qualified.ends_with(".keys")));
        assert!(info
            .symbols
            .iter()
            .any(|s| s.kind == SymbolKind::Tag && s.qualified.ends_with(".extracted")));
        let bytes = mincb::encode(&info, 2);
        assert_eq!(&bytes[0..4], b"MINC");
        assert_eq!(mincb::decode_pack(&bytes).as_deref(), Some("metro.escape"));
    }

    #[test]
    fn parses_chain_and_place() {
        let src = r#"
pack metro.escape;

@Chain("Play")
@Repeat
@AlwaysActive
public chain Play {
    if (Match.phase == Phase.PLAY) {
        Match.playTick += 1;
    }
}

@Controller
world MetroEscape {
    origin (0, 64, 0);
    chain Play at (2, 64, 0) layout stack facing up;
    clock Play;
}

place Station {
    gold_block at (0, 63, 0);
    chest crate1 at (32, 64, 4) facing west {
        slot 0: iron_ingot * 4;
        slot 2: gold_ingot * 1;
    }
}
"#;
        let unit = parse(src).expect("should parse");
        assert!(matches!(unit.items[0], Item::Chain(_)));
        assert!(matches!(unit.items[1], Item::World(_)));
        assert!(matches!(unit.items[2], Item::Place(_)));
        let info = extract_binary_info(&unit);
        assert_eq!(info.origin, Some([0, 64, 0]));
        assert_eq!(info.chains[0].name, "Play");
        assert_eq!(info.chains[0].layout.as_deref(), Some("stack"));
        assert_eq!(info.blocks[0].block, "gold_block");
        assert_eq!(info.containers[0].name, "crate1");
        assert_eq!(info.containers[0].slots.len(), 2);
    }

    #[test]
    fn empty_source_fails_without_pack() {
        let errors = parse("").expect_err("should fail");
        assert!(errors.iter().any(|e| e.message.contains("pack")));
    }

    #[test]
    fn rejects_unrecognized_token() {
        let errors = parse("pack x; int y = $;").expect_err("should fail lexing");
        assert!(errors.iter().any(|e| e.message.contains("unrecognized")));
    }

    #[test]
    fn rejects_incomplete_class() {
        let errors = parse("pack x; public class Runner {").expect_err("should fail");
        assert!(!errors.is_empty());
    }

    #[test]
    fn private_field_from_other_class_is_rejected() {
        let src = r#"
pack metro.escape;
public class Runner {
    private int keys;
    public void ok() { this.keys += 1; }
}
public class Other {
    public void bad(Runner r) { r.keys += 1; }
}
"#;
        let unit = parse(src).expect("should parse");
        let errors = sema::check(&unit);
        assert!(errors.iter().any(|e| e.message.contains("private field")));
    }

    #[test]
    fn foreach_and_keyword_method_name() {
        let src = r#"
pack metro.escape;
public class Runner {
    public void tickHold(Region zone) {
        foreach (Player p : Players.all()) {
            as (p) at (p) {
                Player.self().in(zone);
            }
        }
    }
}
"#;
        parse(src).expect("should parse");
    }

    #[test]
    fn parses_enum_switch_local_and_new() {
        let src = r#"
pack demo;
public enum Phase { LOBBY, PLAY }
public class Match {
    public static int phase;
    public void tick() {
        Phase p = Phase.PLAY;
        switch (Match.phase) {
            case Phase.LOBBY -> { Match.phase = Phase.PLAY; }
            default -> { }
        }
        Seq<Player> all = Players.all();
        BlockPos origin = BlockPos.of(0, 64, 0);
        Region box = new Region();
    }
}
"#;
        parse(src).expect("should parse");
    }

    #[test]
    fn compiles_metro_escape_example() {
        let root = std::path::Path::new("examples/metro-escape");
        let (config, unit) = crate::project::load_and_merge(root).expect("load");
        let art = crate::compile::compile_unit(&unit, &config).expect("compile");
        assert_eq!(&art.mincb[0..4], b"MINC");
        assert_eq!(
            mincb::decode_pack(&art.mincb).as_deref(),
            Some("metro.escape")
        );
        let dump = crate::lower::dump_commands(&art.lowered);
        assert!(dump.contains("scoreboard objectives add"), "{dump}");
        assert!(dump.contains("execute if score"), "{dump}");
        assert!(dump.contains("tag @s"), "{dump}");
        assert!(
            dump.contains("ticking")
                || art
                    .info
                    .ticking_areas
                    .iter()
                    .any(|t| t.name == "metro_station")
        );
        assert_eq!(art.info.containers[0].name, "crate1");
        assert!(
            art.command_blocks.iter().any(|b| b.mode == 2),
            "clock Repeat CB"
        );
        assert!(
            dump.contains("if entity @s[tag="),
            "early-return tag gates:\n{dump}"
        );
        let image = mincb::decode_image(&art.mincb).expect("decode");
        assert!(!image.command_blocks.is_empty());
        assert!(art.functions.keys().any(|p| p.contains("install")));
        assert!(image.symbols.iter().any(|s| s.kind == 1), "tag symbols");
        assert!(
            image
                .tags
                .iter()
                .any(|id| image.symbols.iter().any(|s| s.id == *id && s.kind == 1))
                || image.symbols.iter().any(|s| s.kind == 1),
            "TAGS section"
        );
        assert!(
            image.meta_json.contains("gamerules") && image.meta_json.contains("commandblockoutput"),
            "{}",
            image.meta_json
        );
        for f in &image.functions {
            assert!(
                !f.path.is_empty() || image.symbols.iter().any(|s| s.id == f.symb),
                "FUNC path recovered from SYMB"
            );
        }
        assert!(!image.containers.is_empty());
        assert!(
            !image.containers[0].block.is_empty(),
            "CONT block_id recovered"
        );
        assert!(
            image.containers[0].slots.iter().any(|s| !s.item.is_empty()),
            "CONT item_id recovered"
        );
        let dump_bin = mincb::dump_commands_from_image(&image);
        assert!(
            dump_bin.contains("# chain") || dump_bin.contains("# function"),
            "{dump_bin}"
        );
    }

    #[test]
    fn or_lowers_to_two_execute_lines() {
        let src = r#"
pack demo;
public class M {
    public static int a;
    public static int b;
}
public chain C {
    if (M.a == 1 || M.b == 2) {
        cmd("say or");
    }
}
"#;
        let unit = parse(src).unwrap();
        let info = extract_binary_info(&unit);
        let cfg = crate::config::MincConfig {
            edition: crate::config::Edition::Bedrock,
            pack: "demo".into(),
            ..crate::config::MincConfig::default()
        };
        let dump =
            crate::lower::dump_commands(&crate::lower::lower_project(&unit, &cfg, &info).unwrap());
        let n = dump.matches("say or").count();
        assert!(n >= 2, "OR should emit two execute lines:\n{dump}");
    }

    #[test]
    fn java_hasitem_uses_if_items() {
        let src = r#"
pack demo;
public chain C {
    foreach (Player p : Players.all().hasItem(Items.GOLD_INGOT, 1)) {
        as (p) { cmd("say gold"); }
    }
}
"#;
        let unit = parse(src).unwrap();
        let info = extract_binary_info(&unit);
        let cfg = crate::config::MincConfig {
            edition: crate::config::Edition::Java,
            pack: "demo".into(),
            game_version: "1.21.11".into(),
            ..crate::config::MincConfig::default()
        };
        let dump =
            crate::lower::dump_commands(&crate::lower::lower_project(&unit, &cfg, &info).unwrap());
        assert!(dump.contains("if items entity"), "{dump}");
        assert!(!dump.contains("hasitem="), "{dump}");
        assert!(!dump.contains("nbt="), "{dump}");
    }

    #[test]
    fn link_at_label_emits_function() {
        let src = r#"
pack demo;
public chain Lobby {
    label start;
    cmd("say hi");
}
public chain Play {
    cmd("say play");
}
@Controller
world W {
    origin (0, 64, 0);
    chain Lobby at (0, 70, 0) layout linear facing east;
    chain Play at (2, 64, 0) layout stack facing up;
    link Lobby.start => Play;
}
"#;
        let unit = parse(src).unwrap();
        let info = extract_binary_info(&unit);
        assert_eq!(info.links[0].from_label.as_deref(), Some("start"));
        let cfg = crate::config::MincConfig {
            edition: crate::config::Edition::Bedrock,
            pack: "demo".into(),
            ..crate::config::MincConfig::default()
        };
        let dump =
            crate::lower::dump_commands(&crate::lower::lower_project(&unit, &cfg, &info).unwrap());
        assert!(dump.contains("function demo/chain/play"), "{dump}");
    }

    #[test]
    fn java_only_cannot_be_called_from_shared() {
        let src = r#"
pack demo;
public class A {
    @JavaOnly
    public static void javaThing() { cmd("say j"); }
    public static void shared() { A.javaThing(); }
}
"#;
        let unit = parse(src).unwrap();
        let cfg = crate::config::MincConfig {
            edition: crate::config::Edition::Java,
            pack: "demo".into(),
            ..crate::config::MincConfig::default()
        };
        let errors = sema::check_with_config(&unit, Some(&cfg));
        assert!(
            errors.iter().any(|e| e.message.contains("JavaOnly")),
            "{errors:?}"
        );
    }

    #[test]
    fn cli_place_and_dump_mincb() {
        let dir = std::path::PathBuf::from("target/test-minc-place");
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(
            crate::cli::run(vec![
                "minc".into(),
                "new".into(),
                dir.display().to_string(),
                "--edition".into(),
                "bedrock".into(),
                "--version".into(),
                "1.21.70".into(),
            ]),
            0
        );
        assert_eq!(
            crate::cli::run(vec![
                "minc".into(),
                "build".into(),
                dir.display().to_string(),
                "--out".into(),
                "dist".into(),
            ]),
            0
        );
        let mincb = dir.join("dist/test-minc-place.mincb");
        let mar = dir.join("dist/test-minc-place.mar");
        assert!(mar.is_file());
        assert_eq!(
            crate::cli::run(vec![
                "minc".into(),
                "inspect".into(),
                mincb.display().to_string(),
                "--chain".into(),
                "Main".into(),
            ]),
            0
        );
        assert_eq!(
            crate::cli::run(vec![
                "minc".into(),
                "inspect".into(),
                mar.display().to_string(),
            ]),
            0
        );
        assert_eq!(
            crate::cli::run(vec![
                "minc".into(),
                "dump".into(),
                mincb.display().to_string(),
                "--commands".into(),
            ]),
            0
        );
        let place_out = dir.join("dist/place");
        assert_eq!(
            crate::cli::run(vec![
                "minc".into(),
                "place".into(),
                mincb.display().to_string(),
                "--out".into(),
                place_out.display().to_string(),
            ]),
            0
        );
        assert!(
            place_out
                .join("functions/test.minc.place/install.mcfunction")
                .is_file()
                || place_out
                    .join("functions/test/minc/place/install.mcfunction")
                    .is_file()
        );
        assert!(
            place_out.join("test_minc_place.mcstructure").is_file()
                || std::fs::read_dir(&place_out).unwrap().any(|e| e
                    .unwrap()
                    .path()
                    .extension()
                    .and_then(|s| s.to_str())
                    == Some("mcstructure"))
        );
    }

    #[test]
    fn cli_new_check_build() {
        let dir = std::path::PathBuf::from("target/test-minc-new");
        let _ = std::fs::remove_dir_all(&dir);
        let code = crate::cli::run(vec![
            "minc".into(),
            "new".into(),
            dir.display().to_string(),
            "--edition".into(),
            "bedrock".into(),
            "--version".into(),
            "1.21.70".into(),
        ]);
        assert_eq!(code, 0);
        assert!(dir.join("minc.toml").is_file());
        let code = crate::cli::run(vec![
            "minc".into(),
            "check".into(),
            dir.display().to_string(),
        ]);
        assert_eq!(code, 0);
        let code = crate::cli::run(vec![
            "minc".into(),
            "build".into(),
            dir.display().to_string(),
            "--out".into(),
            "dist".into(),
        ]);
        assert_eq!(code, 0);
        assert!(dir.join("dist/test-minc-new.mincb").is_file());
        assert!(dir.join("dist/test-minc-new.mar").is_file());
    }

    #[test]
    fn illegal_constructs_have_real_spans() {
        let null_src = "pack p;\npublic class A { public void m() { Player x = null; } }\n";
        let unit = parse(null_src).expect("parse null");
        let errors = sema::check(&unit);
        let e = errors
            .iter()
            .find(|e| e.message.contains("null"))
            .expect("null error");
        assert!(e.span.end > e.span.start, "{e:?}");
        assert!(null_src[e.span.clone()].contains("null"));

        let new_src = "pack p;\npublic class A { public void m() { Player p = new Player(); } }\n";
        let unit = parse(new_src).expect("parse new");
        let errors = sema::check(&unit);
        let e = errors
            .iter()
            .find(|e| e.message.contains("new"))
            .expect("new error");
        assert!(e.span.end > e.span.start, "{e:?}");

        let arr = parse("pack p; public class A { int[] xs; }").expect_err("int[]");
        assert!(arr
            .iter()
            .any(|e| e.span.end > e.span.start && e.message.contains("int[]")));

        let tr = parse("pack p; public class A { public void m() { try { } } }").expect_err("try");
        assert!(tr
            .iter()
            .any(|e| e.span.end > e.span.start && e.message.contains("try")));
    }

    #[test]
    fn private_field_span_is_use_site() {
        let src = r#"
pack metro.escape;
public class Runner {
    private int keys;
    public void ok() { this.keys += 1; }
}
public class Other {
    public void bad(Runner r) { r.keys += 1; }
}
"#;
        let unit = parse(src).expect("parse");
        let errors = sema::check(&unit);
        let e = errors
            .iter()
            .find(|e| e.message.contains("private field"))
            .expect("private");
        assert!(e.span.end > e.span.start);
        assert!(src[e.span.clone()].contains("keys"));
        assert!(!src[e.span.clone()].contains("private int keys"));
    }

    #[test]
    fn score_add_uses_compiler_temps() {
        let src = r#"
pack demo;
public class Match {
    public static int a;
    public static int b;
    public static int c;
}
public chain C {
    Match.a = Match.b + Match.c;
}
"#;
        let unit = parse(src).unwrap();
        let info = extract_binary_info(&unit);
        let cfg = crate::config::MincConfig {
            edition: crate::config::Edition::Bedrock,
            pack: "demo".into(),
            ..crate::config::MincConfig::default()
        };
        let dump =
            crate::lower::dump_commands(&crate::lower::lower_project(&unit, &cfg, &info).unwrap());
        assert!(dump.contains("#t0 mt"), "{dump}");
        assert!(dump.contains("operation"), "{dump}");
    }

    #[test]
    fn instance_int_emits_add_zero() {
        let src = r#"
pack demo;
public class Runner {
    private int keys;
    public void giveKey() { this.keys += 1; }
}
"#;
        let unit = parse(src).unwrap();
        let info = extract_binary_info(&unit);
        let cfg = crate::config::MincConfig {
            edition: crate::config::Edition::Bedrock,
            pack: "demo".into(),
            ..crate::config::MincConfig::default()
        };
        let dump =
            crate::lower::dump_commands(&crate::lower::lower_project(&unit, &cfg, &info).unwrap());
        assert!(
            dump.contains("scoreboard players add @s") && dump.contains(" 0"),
            "{dump}"
        );
    }

    #[test]
    fn players_raw_checked_against_edition() {
        let src = r#"
pack demo;
public chain C {
    foreach (Player p : Players.raw("@a[hasitem={item=gold_ingot}]")) {
        as (p) { cmd("say x"); }
    }
}
"#;
        let unit = parse(src).unwrap();
        let cfg = crate::config::MincConfig {
            edition: crate::config::Edition::Java,
            pack: "demo".into(),
            game_version: "1.21.11".into(),
            ..crate::config::MincConfig::default()
        };
        let errors = sema::check_with_config(&unit, Some(&cfg));
        assert!(
            errors.iter().any(|e| e.message.contains("hasitem")),
            "{errors:?}"
        );
        assert!(errors.iter().any(|e| e.span.end > e.span.start));
    }

    fn dump_ed(src: &str, edition: crate::config::Edition) -> String {
        let unit = parse(src).unwrap();
        let info = extract_binary_info(&unit);
        let cfg = crate::config::MincConfig {
            edition,
            pack: "demo".into(),
            game_version: if edition == crate::config::Edition::Java {
                "1.21.11".into()
            } else {
                "1.21.70".into()
            },
            ..crate::config::MincConfig::default()
        };
        crate::lower::dump_commands(&crate::lower::lower_project(&unit, &cfg, &info).unwrap())
    }

    #[test]
    fn parses_temp_list_and_run() {
        let src = r#"
pack demo;
public class Kit {
    public static void open() {
        temp int flash = 3;
        temp Item key = Items.GOLD_INGOT.count(1);
        List<Item> loot = List.of(Items.GOLD_INGOT.count(1), Items.IRON_INGOT.count(4));
        foreach (Item it : loot) {
            give(Players.all(), it);
        }
        run "say kit";
        /kill @e[type=zombie];
        run give @a diamond 1;
    }
}
"#;
        let unit = parse(src).expect("parse builtins");
        let Item::Class(class) = &unit.items[0] else {
            panic!("class");
        };
        let Member::Method(m) = &class.members[0] else {
            panic!("method");
        };
        assert!(m
            .body
            .iter()
            .any(|s| matches!(s.kind, crate::ast::StmtKind::Local { is_temp: true, .. })));
        assert!(m
            .body
            .iter()
            .any(|s| matches!(s.kind, crate::ast::StmtKind::Run { .. })));
        assert_eq!(
            m.body
                .iter()
                .filter(|s| matches!(s.kind, crate::ast::StmtKind::Run { .. }))
                .count(),
            3
        );
    }

    #[test]
    fn temp_int_is_not_a_scoreboard_and_resets_scratch() {
        let src = r#"
pack demo;
public class Match {
    public static int a;
    public static int b;
    public static int c;
}
public chain C {
    temp int flash = 3;
    Match.a = flash;
    Match.a = Match.b + Match.c;
}
"#;
        let dump = dump_ed(src, crate::config::Edition::Bedrock);
        assert!(dump.contains("scoreboard players set mcs"), "{dump}");
        assert!(dump.contains(" 3"), "{dump}");
        assert!(!dump.contains("objectives add flash"), "{dump}");
        assert!(dump.contains("scoreboard players reset #t"), "{dump}");
    }

    #[test]
    fn list_foreach_unrolls_give_per_edition() {
        let src = r#"
pack demo;
public chain C {
    List<Item> loot = List.of(Items.GOLD_INGOT.count(1), Items.IRON_INGOT.count(4));
    foreach (Item it : loot) {
        give(Players.all(), it);
    }
}
"#;
        let be = dump_ed(src, crate::config::Edition::Bedrock);
        assert!(be.contains("give @a gold_ingot 1"), "{be}");
        assert!(be.contains("give @a iron_ingot 4"), "{be}");
        assert!(!be.contains("minecraft:gold_ingot"), "{be}");
        let je = dump_ed(src, crate::config::Edition::Java);
        assert!(je.contains("give @a minecraft:gold_ingot 1"), "{je}");
        assert!(je.contains("give @a minecraft:iron_ingot 4"), "{je}");
        assert!(!je.contains("hasitem="), "{je}");
    }

    #[test]
    fn run_and_slash_emit_one_command() {
        let src = r#"
pack demo;
public chain C {
    run "say hello";
    /say hi;
    say("kit");
}
"#;
        let dump = dump_ed(src, crate::config::Edition::Bedrock);
        assert!(dump.contains("say hello"), "{dump}");
        assert!(dump.contains("say hi"), "{dump}");
        assert!(dump.contains("say kit"), "{dump}");
    }

    #[test]
    fn effect_tellraw_title_split_editions() {
        let src = r#"
pack demo;
public chain C {
    effect(Players.all(), "speed", 10, 1);
    tellraw(Players.all(), Text.raw("撤离"));
    title(Players.all(), Title.TITLE, "地铁");
    xp(Players.self(), 5);
}
"#;
        let be = dump_ed(src, crate::config::Edition::Bedrock);
        assert!(be.contains("effect @a speed 10 1 false"), "{be}");
        assert!(!be.contains("effect give"), "{be}");
        assert!(be.contains("rawtext"), "{be}");
        assert!(be.contains("title @a title 地铁"), "{be}");
        assert!(be.contains("xp 5 @s"), "{be}");
        let je = dump_ed(src, crate::config::Edition::Java);
        assert!(
            je.contains("effect give @a minecraft:speed 10 1 false"),
            "{je}"
        );
        assert!(je.contains("{\"text\":\"撤离\"}"), "{je}");
        assert!(!je.contains("rawtext"), "{je}");
        assert!(je.contains("{\"text\":\"地铁\"}"), "{je}");
        assert!(je.contains("xp add @s 5 points"), "{je}");
    }

    #[test]
    fn temp_second_use_is_rejected() {
        let src = r#"
pack demo;
public class Match { public static int a; public static int b; }
public chain C {
    temp int flash = 3;
    Match.a = flash;
    Match.b = flash;
}
"#;
        let unit = parse(src).unwrap();
        let info = extract_binary_info(&unit);
        let cfg = crate::config::MincConfig {
            edition: crate::config::Edition::Bedrock,
            pack: "demo".into(),
            ..crate::config::MincConfig::default()
        };
        let err = crate::lower::lower_project(&unit, &cfg, &info).expect_err("second use");
        assert!(
            err.iter().any(|e| e.message.contains("already consumed")),
            "{err:?}"
        );
    }

    #[test]
    fn item_count_is_ok_on_bedrock() {
        let src = r#"
pack demo;
public chain C {
    give(Players.all(), Items.GOLD_INGOT.count(1));
}
"#;
        let unit = parse(src).unwrap();
        let cfg = crate::config::MincConfig {
            edition: crate::config::Edition::Bedrock,
            pack: "demo".into(),
            ..crate::config::MincConfig::default()
        };
        let errors = sema::check_with_config(&unit, Some(&cfg));
        assert!(
            errors.iter().all(|e| !e.message.contains("execute store")),
            "{errors:?}"
        );
    }

    #[test]
    fn java_replaceitem_is_item_replace() {
        let src = r#"
pack demo;
public chain C {
    replaceItem(Players.self(), "hotbar.0", Items.GOLD_INGOT.count(1));
}
"#;
        let be = dump_ed(src, crate::config::Edition::Bedrock);
        assert!(
            be.contains("replaceitem entity @s slot.hotbar.0 gold_ingot 1"),
            "{be}"
        );
        let je = dump_ed(src, crate::config::Edition::Java);
        assert!(
            je.contains("item replace entity @s hotbar.0 with minecraft:gold_ingot 1"),
            "{je}"
        );
        assert!(!je.contains("replaceitem"), "{je}");
    }

    #[test]
    fn compiles_tutorial_kit_example() {
        let root = std::path::Path::new("examples/tutorial-kit");
        let (config, unit) = crate::project::load_and_merge(root).expect("load");
        let art = crate::compile::compile_unit(&unit, &config).expect("compile");
        let dump = crate::lower::dump_commands(&art.lowered);
        assert!(dump.contains("give @a gold_ingot 1"), "{dump}");
        assert!(dump.contains("give @a iron_ingot 4"), "{dump}");
        assert!(dump.contains("say kit open"), "{dump}");
        assert!(dump.contains("kill @e[type=item]"), "{dump}");
        assert!(dump.contains("effect @a speed"), "{dump}");
        assert!(!dump.contains("objectives add flash"), "{dump}");
        assert!(!dump.contains("effect give"), "{dump}");
        assert!(!art.mar.is_empty());
        let opened = crate::mar::open(&art.mar).expect("mar");
        assert_eq!(opened.mincb, art.mincb);
        assert_eq!(opened.edition(), Some(config.edition.as_str()));
    }

    #[test]
    fn unknown_item_is_rejected() {
        let src = r#"
pack demo;
public chain C {
    give(Players.all(), Items.NOT_A_REAL_ITEM.count(1));
}
"#;
        let unit = parse(src).unwrap();
        let errors = sema::check(&unit);
        assert!(
            errors.iter().any(|e| e.message.contains("NOT_A_REAL_ITEM")),
            "{errors:?}"
        );
    }

    #[test]
    fn item_traits_and_tag_object_lower() {
        let src = r#"
pack demo;
public chain C {
    Tag key = Tag.of("metro_key");
    give(Players.all(), Items.DIAMOND_SWORD.named("钥匙").lock().tag(key));
    foreach (Player p : Players.all().withTag(key)) {
        as (p) {
            p.add(key);
            if (p.has(key)) {
                cmd("say tagged");
            }
        }
    }
}
"#;
        let be = dump_ed(src, crate::config::Edition::Bedrock);
        assert!(be.contains("diamond_sword"), "{be}");
        assert!(be.contains("item_lock"), "{be}");
        assert!(
            be.contains("tag @a[tag=metro_key] add metro_key")
                || be.contains("tag @s add metro_key"),
            "{be}"
        );
        assert!(be.contains("if entity") && be.contains("metro_key"), "{be}");
        let je = dump_ed(src, crate::config::Edition::Java);
        assert!(je.contains("minecraft:diamond_sword["), "{je}");
        assert!(je.contains("custom_name="), "{je}");
        assert!(je.contains("metro_key"), "{je}");
    }

    #[test]
    fn java_for_while_plus_plus_and_ternary_are_not_one_to_one() {
        let src = r#"
pack demo;
public class Match { public static int a; }
public chain C {
    for (int i = 0; i < 3; i++) {
        cmd("say hi");
    }
    Match.a = true ? 2 : 9;
    Match.a++;
    if (false) {
        cmd("say dead");
    }
}
"#;
        let dump = dump_ed(src, crate::config::Edition::Bedrock);
        assert_eq!(dump.matches("say hi").count(), 3, "{dump}");
        assert!(!dump.contains("say dead"), "{dump}");
        assert!(!dump.contains(" 9"), "{dump}");
        assert!(
            dump.contains("scoreboard players set") || dump.contains("scoreboard players add"),
            "{dump}"
        );
    }

    #[test]
    fn mar_is_zip_with_manifest_and_mincb() {
        let src = r#"
pack demo;
public chain C { cmd("say mar"); }
"#;
        let unit = parse(src).unwrap();
        let cfg = crate::config::MincConfig {
            edition: crate::config::Edition::Bedrock,
            pack: "demo".into(),
            name: "kit".into(),
            game_version: "1.21.70".into(),
            ..crate::config::MincConfig::default()
        };
        let art = crate::compile::compile_unit(&unit, &cfg).expect("compile");
        assert!(crate::mar::is_mar(&art.mar));
        let (mincb, manifest) = crate::mar::unwrap_payload(&art.mar).expect("unwrap");
        assert_eq!(mincb, art.mincb);
        let man = manifest.expect("manifest");
        assert!(man.contains("Edition: bedrock"));
        assert!(man.contains("Game-Version: 1.21.70"));
        assert!(man.contains("Minc-Format: 1"));
    }

    #[test]
    fn compiles_gold_shrine_java_mar() {
        let root = std::path::Path::new("examples/gold-shrine");
        let (config, unit) = crate::project::load_and_merge(root).expect("load");
        assert_eq!(config.edition, crate::config::Edition::Java);
        assert_eq!(config.game_version, "1.21.1");
        let art = crate::compile::compile_unit(&unit, &config).expect("compile");
        assert!(!art.mincb.is_empty());
        assert!(!art.mar.is_empty());
        let opened = crate::mar::open(&art.mar).expect("mar");
        assert_eq!(opened.edition(), Some("java"));
        assert_eq!(opened.game_version(), Some("1.21.1"));
        assert!(
            art.image.meta_json.contains("gold_block"),
            "{}",
            art.image.meta_json
        );
        assert!(art
            .image
            .world_blocks
            .iter()
            .any(|b| b.block.contains("beacon")));
        assert!(art
            .image
            .containers
            .iter()
            .any(|c| c.block.contains("chest")));
        let dump = crate::lower::dump_commands(&art.lowered);
        assert!(dump.contains("say MINC shrine online"), "{dump}");
        assert!(dump.contains("title") || dump.contains("tellraw"), "{dump}");
        let place = crate::place::install_functions(&art.image);
        let world = place
            .values()
            .find(|b| b.contains("fill ") && b.contains("gold_block"))
            .cloned()
            .unwrap_or_default();
        assert!(
            world.contains("fill 0 62 0 26 62 16 stone_bricks"),
            "{world}"
        );
        assert!(world.contains("fill 0 63 0 26 63 16 gold_block"), "{world}");
    }
}
