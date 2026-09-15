# MincScript

A Java-shaped language that will compile to Minecraft (Java Edition or Bedrock). **Language design** (no compiler for this syntax yet): [`docs/language/`](docs/language/README.md).

This repository currently has a **placeholder frontend**: a Logos lexer, a Chumsky parser, and Ariadne diagnostics that only accept `let` and arithmetic. Interpreter and the design in `docs/language/` wait for later work.

Minecraft command grammars (Java + Bedrock), intended for the compiler backend, live in [`docs/minecraft-commands/`](docs/minecraft-commands/README.md). Bedrock-as-game-engine (地铁逃生-style match FSM, save vs execute) is in [`docs/minecraft-commands/bedrock/gameplay/`](docs/minecraft-commands/bedrock/gameplay/README.md); a drop-in behavior pack is [`examples/bedrock-metro-escape/`](examples/bedrock-metro-escape/).

## Status

The parser accepts:

- `let` bindings: `let x = 1 + 2;`
- expression statements: `1 + 2 * 3;`
- integers, identifiers, unary `-`, `+ - * /`, and comparisons (`== != < > <= >=`)
- `//` line comments

## Build

```bash
cargo test
cargo run -- examples/hello.mcs
```

Pipe source on stdin if no file is given:

```bash
echo 'let x = 1 + 2;' | cargo run
```

## Crates

| Crate | Role |
| --- | --- |
| `logos` | Lexer |
| `chumsky` 0.9 | Parser combinators |
| `ariadne` 0.5 | Error reports |
| `thiserror` | Error types |
