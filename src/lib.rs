//! MincScript: Java-shaped language that compiles to Minecraft MINCB.

pub mod ast;
pub mod cli;
pub mod compile;
pub mod config;
pub mod diagnostic;
pub mod extract;
pub mod layout;
pub mod lexer;
pub mod lower;
pub mod mincb;
pub mod parser;
pub mod project;
pub mod sema;
pub mod span;
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
    parser::parse_tokens(source.len()..source.len(), tokens)
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
    }
}
