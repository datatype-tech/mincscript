# MincScript

A programming language implemented in Rust.

This repository currently has the **frontend skeleton**: a Logos lexer, a Chumsky parser, and Ariadne diagnostics. Interpreter, types, and the rest of the language wait for later work.

Minecraft command grammars (Java + Bedrock), intended for a later compiler backend, live in [`docs/minecraft-commands/`](docs/minecraft-commands/README.md). Bedrock-as-game-engine (地铁逃生-style match FSM, save vs execute) is in [`docs/minecraft-commands/bedrock/gameplay/`](docs/minecraft-commands/bedrock/gameplay/README.md); a drop-in behavior pack is [`examples/bedrock-metro-escape/`](examples/bedrock-metro-escape/).

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
