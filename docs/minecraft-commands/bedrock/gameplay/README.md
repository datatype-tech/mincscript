# Bedrock commands as a game system

The 82 official Bedrock commands are not a cheat sheet. Combined, they are enough to ship a **match-based in-world game**: lobby, countdown, loot, keys, NPCs, extract, reset. Mature maps (official Microsoft samples, Bedrock Commands Community systems, and Chinese 地铁逃生 / extraction maps) all share the same lattice.

This folder is the playbook. The drop-in behavior pack is [`examples/bedrock-metro-escape/`](../../../../examples/bedrock-metro-escape/).

| Page | Question it answers |
| --- | --- |
| [00 — Save and execute](00-save-and-execute.md) | Where commands live, what survives a reboot, and **which engines actually run them** |
| [01 — State and logic](01-state-and-logic.md) | Scoreboard FSM, tags, selector algebra, clocks |
| [02 — 地铁逃生 architecture](02-metro-escape.md) | How extraction maps are assembled from those primitives |
| [03 — Combinable recipes](03-command-recipes.md) | Official-syntax snippets you can cross-wire |

Grammar for each verb remains in [`../`](../README.md). **Do not emit Java-only forms** (`store`, `on`, `if data`, `nbt=`, `distance=`, item `[]` components, `/return`).

## The lattice (what to combine)

Every Bedrock command game is six axes. A 地铁逃生 is those axes filled in, not a new language.

| Axis | Holds | Typical commands |
| --- | --- | --- |
| **Clock** | “now” | `tick.json` → `/function` → `scoreboard players add` / `operation` / `matches` |
| **Phase** | match state | dummy objective + fake player (`metro phase 0..4`) |
| **Fact** | boolean per entity | `/tag`, selector `tag=` / `tag=!` |
| **Inventory** | keys, loot, loadout | `hasitem`, `/give`, `/clear`, `/replaceitem`, `/loot` |
| **Space** | rooms, crates, extract | `x/y/z/dx` or `r/rm`, `execute if block` / `if entity` |
| **Presentation** | feel | `/title` `/titleraw` `/tellraw` `/camera` `/music` `/fog` `/dialogue` `/inputpermission` `/hud` |

`/execute` is the **wiring**. It does not store data. Chain `as` / `at` / `if` / `unless` then `run` one of the other axes. Bedrock evaluate **depth-first**; Java is breadth-first — do not share a walker.

## Proven cases (not theory)

| Case | What it proves | Source |
| --- | --- | --- |
| Official command-block reward loop | Repeat Always Active + Conditional Chain = “test → not already rewarded → give → mark” | Microsoft Learn *Getting Started with Command Blocks* |
| Official Complete the Monument | Fake `#` players as booleans, `players test`, `operation += *`, silent gamerules | Microsoft Learn *Create an In-World Game* |
| Official snowball fight | **Tags as teams** (Bedrock has no `/team`), `players random`, `execute if score … matches`, `family=` | Microsoft Learn *Use Command Blocks to Have a Snowball Fight* |
| Official NPC + `/dialogue` | Buttons run commands; `@initiator`; scene JSON; hidden NPC as a UI popup | Microsoft Learn *NPC Dialogue* + `minecraft-samples` `npc_dialogue_sample` |
| Official `tick.json` | 20 gameplay ticks/s function clock, **runs before the world is fully loaded** | Microsoft Learn *Introduction to tick.json* |
| Official ticking area + `/schedule` | Command blocks **stop when the chunk unloads**; named areas persist; wait until chunks/entities exist | Microsoft Learn *Tickingarea Command* |
| Official `/structure` | `saveMode disk` persists in the world DB; `memory` does not | Official `structure` command |
| Bedrock Wiki logic gates | AND/OR/XOR from selector args + `if`/`unless` | wiki.bedrock.dev *Execute Logic Gates* |
| Bedrock Wiki scoreboard timers | Function delay = clock score + `matches` / modulo | wiki.bedrock.dev *Scoreboard Timers* |
| 量筒 function template | Production `tick.json` + timeline controller (functions’ two hard problems: conditions and time) | Open Bedrock function template |
| 地铁逃生 maps / tutorials | Same FSM: 匹配, 物资箱, 男团, 钥匙房, 撤离倒计时 | Community maps + *和平精英* mode as the **rules** reference |

Microsoft’s snowball-fight sample writes `/scoreboard objectives add random test`. That is **invalid**. The only addable criterion is `dummy`. Use `dummy`, then `scoreboard players random`.

## Compiler implication

MincScript should treat Bedrock commands as:

1. A **closed 82-command ISA**
2. Plus **execution hosts** (`tick.json`, command blocks, NPC, `/schedule`, `/function`, `/scriptevent`)
3. Plus **persistence hosts** (behavior pack files vs world DB)

A map that “has commands” but no host will not run. A map that hosts command blocks without a ticking area will run only while a player is nearby. That is the usual “I saved it but it does not execute” failure.
