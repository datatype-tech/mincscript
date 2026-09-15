# Bedrock world, structure, camera, script

Signatures: [`commands/generated-syntax.md`](commands/generated-syntax.md). Notes for compilers:

## Blocks

- `/setblock` `<pos> <Block> [blockStates] [destroy|keep|replace]`
- `/fill` region + optional `replace` filter + block states
- `/clone` `begin end dest [maskMode] [cloneMode]` and `filtered` overload with `tileName` + `blockStates`

## Structures

```
structure save <name> <from> <to> [saveMode]
structure load <name> <to> [rotation] [mirror] [includeEntities] …
structure delete <name>
```

This is **not** Java `/place template`.

## Ticking areas

```
tickingarea add <from> <to> [name]
tickingarea add circle <center> <radius> [name]
tickingarea remove <name> | <pos>
tickingarea list [all-dimensions]
```

Java uses `/forceload` (chunk tickets), different units.

## Camera / HUD / input / animation

`camera`, `camerashake`, `hud`, `inputpermission`, `playanimation`, `fog`, `music`, `dialogue`, `aimassist` are Bedrock creator commands. Parse **only** the official overloads. They have no Java equivalents.

## Script

```
scriptevent <messageId> [message]
script <debugger subcommands>
```

`scriptevent` is the command↔script bridge. Java has no equivalent (use `/trigger` or function macros instead).

## Detect commands

`/testfor`, `/testforblock`, `/testforblocks` still exist. Prefer `/execute if entity|block|blocks` in new code. Both must parse.

## `/function`

Runs `functions/<path>.mcfunction` in a behavior pack. **No** `{macros}` argument. `/schedule` exists with Bedrock-specific `on_area_loaded` vs time forms — see official `schedule.md`.
