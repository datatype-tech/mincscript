# Java Edition commands — overview

**Pinned tree:** Minecraft `26.3-rc-3` (data version 5022). Latest stable at extraction: `26.2`.

Java commands are a **Brigadier command graph**. There is no single regex. The dispatcher walks a tree of:

- **literal** nodes (exact tokens such as `execute`, `if`, `store`)
- **argument** nodes (typed parsers such as `minecraft:entity`)
- **executable** flags (this prefix is a complete command)
- **redirects** (continue parsing at another node — used by `/execute` to loop)

Root literals in this tree (94, including aliases `tp`/`teleport`, `w`/`msg`, `tm`/`teammsg`, `xp`/`experience`):

See [`commands/generated-usages.md`](commands/generated-usages.md).

## Chat vs other sources

| Context | Leading `/` | Notes |
| --- | --- | --- |
| Chat | required | Official minecraft.net primer |
| Command block | optional | First token is the command name |
| Function / datapack `.mcfunction` | optional | `#` line comments; macros use `$(name)` |
| Server console | optional | Same graph, different permission source |

## Permission model

Each root node may carry `permissions` with `minecraft:command_level` (`all`, `moderators`, `gamemasters`, `admins`, `owners`) or more specific predicates. The tree records this; the parser should keep it on the AST even if a later compiler ignores it.

## What this edition has that Bedrock does not

Full `/data`, `/item`, `/return`, `/bossbar`, `/team`, `/datapack`, `/forceload`, `/worldborder`, `/fillbiome`, `execute store`, `execute on`, `execute summon`, predicates, SNBT selectors, item components, function macros.

See [`../crosswalk/command-matrix.md`](../crosswalk/command-matrix.md).
