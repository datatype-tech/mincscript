# MincScript

A Java-shaped language that compiles to Minecraft (Java Edition or Bedrock). Language design: [`docs/language/`](docs/language/README.md). Command grammars live in [`docs/minecraft-commands/`](docs/minecraft-commands/README.md).

The `minc` compiler walks **tokenizer → parser (syntax validation) → type check → command lowering → MINCB**. Bedrock and Java each get their own execute/selector printer; they do not share an execute walker.

## Status

Implemented, starting from the lexer:

- **Tokenizer** — Java-like keywords, Unicode identifiers, `//` / `/* */`, string escapes
- **Parser** — `pack` / `import`, classes, enums, chains, `controller` / `world`, `place` / chests, `if` / `foreach` / `switch`, `as`/`at`, locals, `new`, `cmd`
- **Check** — private fields and methods stay inside their class
- **Extract** — stable score/tag ids (`m00`, `t0a`, …) for MINCB `SYMB`
- **Lower** — dummy scores, tags, `execute` (edition-specific), chain flatten, stack/linear/snake/box layouts
- **CLI** — `new`, `check`, `build`, `inspect`, `dump --commands`, `layout`

A buildable 地铁逃生 sketch is [`examples/metro-escape/`](examples/metro-escape/).

## Build

```bash
cargo test
cargo run --bin minc -- check examples/metro-escape
cargo run --bin minc -- build examples/metro-escape --out dist/
cargo run --bin minc -- dump --commands
```

`minc new my-pack --edition bedrock --version 1.21.70` writes `minc.toml` plus a clock chain.

## Crates

| Crate | Role |
| --- | --- |
| `logos` | Lexer |
| `ariadne` 0.5 | Error reports |
| `thiserror` | Error types |
