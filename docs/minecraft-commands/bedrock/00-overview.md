# Bedrock Edition commands — overview

**Pinned signatures:** Mojang `@minecraft/api-docs-generator` markdown in MicrosoftDocs/minecraft-creator (`creator/Commands/commands`, 82 commands), matching Microsoft Learn.

Bedrock is **not** Brigadier. Overloads are listed as separate usage lines with `name: type` arguments. Chaining `/execute` uses the recursive type `executechainedoption_0`.

## Enabling

Official primer (minecraft.net) + Creator intro:

- World must have **Allow Cheats**.
- Command blocks need cheats + operator.
- Chat commands start with `/`. Tab completes syntax.
- Functions live in behavior packs; `/reload` reloads functions and scripts.

## Permission labels in the official files

`Any` | `Game Directors` | `Admin` | `Owner` | `Host`

`Requires Cheats?` is `Yes`/`No` per command (see generated syntax).

## What Bedrock has that Java does not

`camera`, `camerashake`, `dialogue`, `fog`, `hud`, `inputpermission`, `playanimation`, `music`, `structure`, `tickingarea`, `replaceitem`, `testfor`/`testforblock`/`testforblocks`, `event`, `mobevent`, `scriptevent`, `titleraw`, `aimassist`, `allowlist` (Java `whitelist`), …

Education/agent (`@c`, `/agent`, …) are **not** in the 82-command Creator set and are out of scope here.

## Java commands you must not emit on Bedrock

`/data`, `/item`, `/bossbar`, `/team`, `/return`, `/execute store`, `execute on`, `nbt=` selectors, item component `[]` patches, function macros.
