# Parser and compiler strategy

## Two backends

```
MincScript source
        │
        ▼
   shared HIR (optional)
        │
   ┌────┴────┐
   ▼         ▼
 Java IR   Bedrock IR
   │         │
   ▼         ▼
 .mcfunction  BP functions
 (Brigadier)  (overloads)
```

Sharing HIR is optional. Sharing **token trees** for execute/selectors is a bug source. Prefer:

```
enum Edition { Java, Bedrock }
```

on every command node.

## Completeness definition

**Java complete** = every usage string in `java/commands/generated-usages.md` parses with the matching argument parsers.

**Bedrock complete** = every official usage in `bedrock/commands/generated-syntax.md` parses, plus selector grammar in `bedrock/03-selectors.md`.

That is stricter than “we support `/give` and `/tp`”.

## Regeneration

When Minecraft ships a new snapshot:

1. Refresh `mcmeta` tree and Creator markdown (`SOURCES.md`).
2. Re-run `tools/extract_minecraft_commands.py`.
3. Diff `generated-usages.md` / `command-matrix.md`.
4. Any new `parser` or official overload is a **failing test** until handwritten grammar pages are updated.

## Do not

- Parse old Bedrock `execute detect`.
- Emit Java `execute store` on Bedrock.
- Use one selector argument table.
- Treat wiki examples as syntax (they mix editions). Official lines + brigadier win.
