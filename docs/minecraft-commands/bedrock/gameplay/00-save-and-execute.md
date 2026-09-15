# Save, persist, and actually execute

“保存指令” and “实行指令” are different layers. A `.mcfunction` on disk is inert until an **execution host** calls it. A command block in a chunk is inert when that chunk is not ticking. This page is the contract.

## Two stores

| Store | What you put there | Survives quit/reboot? | Who edits it |
| --- | --- | --- | --- |
| **Behavior pack** | `functions/**.mcfunction`, `functions/tick.json`, `dialogue/*.json`, loot tables | Yes, as long as the pack stays applied (world `behavior_packs/` or global) | You, git, `/reload` |
| **World database** | Scoreboard objectives/scores, entity/player tags, named ticking areas, command blocks as blocks, `structure save … disk`, gamerules | Yes, with the world | Commands at runtime |

`/save <mode>` is **dedicated-server only** (`query` / `hold` / `resume`). It is not how you persist a command system. Do not teach it as “save my functions.”

`/structure save … memory` is **RAM**. It will not be there after the world unloads. `disk` writes the structure into the world DB.

Player inventories, tags, spawnpoints, and dummy scores are world data. **Function text is not.** If you only type commands into chat, they are gone. Put them in a pack or in command blocks.

## Execution hosts (the only ways commands run)

```
chat /function          → one-shot, executor = player
tick.json               → every gameplay tick, executor = server, origin Overworld 0,0,0
/schedule delay         → later, still a function
/schedule on_area_loaded → after those chunks/entities exist
command block           → Impulse / Chain / Repeat, only while its chunk ticks
NPC button / on_open    → player interaction; @initiator is the clicker
/scriptevent            → Script API can listen and continue the system
```

Official: `.mcfunction` lines have **no leading `/`**. Nested path: `functions/init/ouch.mcfunction` is `/function init/ouch`. One `/function` call may not run more than **10,000** commands including nested functions (gamerule `functioncommandlimit`, default 10000).

`/reload` reloads functions and scripts from the pack. It does not reset scores.

### Host comparison (pick this first)

| Need | Use | Do not rely on |
| --- | --- | --- |
| Match loop, timers, loot checks | `tick.json` + functions | A Repeat command block at spawn with no ticking area |
| One-off authored event (button, pressure plate) | Impulse command block or NPC button | `tick.json` polling a gold block is also valid (official CTM style) |
| Chunk not loaded yet | `/schedule on_area_loaded add tickingarea <name> <fn>` | Bare `structure load` on tick 0 |
| Command blocks must run with nobody nearby | `/tickingarea add` (max **10** per world; circle radius **0–4** chunks; box ≤ **100** chunks) | Simulation distance luck |
| Versioned logic, git, `/reload` | functions | Command blocks as the only copy of the program |

`tick.json` uses **gameplay ticks** (20/s), not redstone ticks (10/s). Multiple behavior packs: all `tick.json` `values` arrays are **additive**.

Default execution origin for `tick.json` functions: **Overworld `0, 0, 0`**, as the server, **as soon as the world is initialized**, possibly **before players or target chunks exist**. Official warning: this causes unintended behavior if you are sloppy.

Command blocks: Repeat runs every in-game tick while powered (or Always Active). Chain runs when the previous block in the arrow direction activates. Conditional runs only if the previous command succeeded. **Needs Redstone** vs **Always Active**. Delay in Ticks + Execute on First Tick apply to repeating blocks.

When the player walks away, the chunk unloads and **command blocks stop**. Functions in `tick.json` keep running (server-side). That is why modern maps moved the loop to functions.

## Persistence map (what “save” means per subsystem)

| Game data | Persist how | Lost when |
| --- | --- | --- |
| Function source | Pack file | Pack removed / not on dedicated server |
| `tick.json` schedule | Pack file | Pack removed |
| Dummy scores (phase, timers, fake players) | World | Objective removed / world deleted |
| Tags (`queued`, `metro_key`) | Entity in world | Player never joined that world again (tags live on the player) / `/tag remove` |
| Ticking area | World, by **name** | `/tickingarea remove` |
| Structure snapshot | `disk` in world DB | `/structure delete` or `memory` + session end |
| Command block text | Block NBT in chunk | Chunk not copied; World Builder forgot the CB |
| Dialogue scenes | Pack `dialogue/*.json` | Pack removed |
| `/camera` `/inputpermission` `/hud` `/music` `/fog` | Runtime on the player | Often **not** what you want after reboot — re-apply from `init` / join |

Init therefore both **creates** world objects (objectives, ticking areas) and **re-asserts** presentation (gamerules, camera default).

## The load-order trap (why maps “don’t execute”)

1. `tick.json` fires.
2. Players are not in the selector yet → `@a` is empty.
3. Station chunks are not loaded → `structure load` / `testforblock` / armor stands missing.
4. `execute if score metro inited matches 1` **cannot succeed** if the objective does not exist yet — and `unless` also fails when the objective is missing. A tick function that *only* tests a score never bootstraps.

**Working bootstrap** (functions continue after a failed line):

```
scoreboard objectives add metro dummy
scoreboard players add metro inited 0
execute if score metro inited matches 0 run function metro/init
execute if score metro inited matches 1 run function metro/loop
```

`objectives add` of an existing objective fails every later tick. Turn off spam:

```
gamerule commandblockoutput false
gamerule sendcommandfeedback false
```

Those two gamerules are what official Complete-the-Monument uses so Repeat/tick systems do not flood chat. They do **not** disable the commands.

After bootstrap, if you need entities in a far station:

```
tickingarea add circle 32 64 0 4 metro_station true
schedule on_area_loaded add tickingarea metro_station metro/on_station_loaded
```

`preload true` asks the area to load earlier in world start. Official: still pair with `on_area_loaded` if the next commands require those chunks.

Join-safe work (cutscenes, tags) belongs in:

```
execute as @a[tag=!metro_joined] run function metro/on_join
```

not in the same breath as world init.

## Command-block loop vs function loop

Official emerald-for-diamond-block loop (Repeat Always Active → Conditional Chain):

1. `/testforblock 0 4 0 diamond_block`
2. `/testfor @p[tag=!placed_block]`
3. `/give @p emerald`
4. `/tag @p add placed_block`

That is AND via **Conditional** chain. The function equivalent is one tick line:

```
execute if block 0 4 0 diamond_block as @p[tag=!placed_block] run function metro/reward
```

and `metro/reward` does `give`, `tag add`. New execute **supersedes** `/testfor*` for new code; both still parse. `min_engine_version` ≥ 1.19.50 is required for new execute in functions.

Prefer the function form: it is saved in git, reloads, and does not need a ticking area unless it mutates unloaded blocks.

## Structure save/load as round reset

Author once (Creative, station built):

```
structure save metro_station 16 56 -16 48 80 16 disk
```

Each round:

```
structure load metro_station 16 56 -16
```

Optional animation overload exists (`block_by_block` / `layer_by_layer`) for show. Reset crates, doors, and NPC positions this way instead of a hundred `/setblock`s.

If the station must exist when nobody is there, the load still needs the destination **loaded** (ticking area or a player).

## How to prove execution (checklist)

1. Pack applied, cheats on, `min_engine_version` ≥ 1.19.50.
2. Chat: `/function metro/init` — should succeed (“executed N functions”).
3. Chat: `/scoreboard objectives list` — `metro` exists.
4. Stand in the world; `/function metro/tick` should be unnecessary if `tick.json` lists `metro/tick`.
5. Break a test: `say tick` once in `metro/tick` — you must see `[Server]` immediately. If not, `tick.json` is wrong or the pack is not in the stack.
6. `/tickingarea list` — named areas you created are there after reboot.
7. `/structure load` after reboot only works if you saved `disk`.
8. Content log: a **syntax error in a function file rejects the whole function** (it will not appear in `/function` suggestions). Fix, `/reload`.

## Dedicated server vs local

Functions must be on **that world’s** behavior pack stack (or a pack the server actually loads). Copying `com.mojang/development_behavior_packs` on one device does nothing for a remote server. World save zip should include `behavior_packs/` when you ship a map.

## What this is not

- Education `/agent` is outside the 82-command set.
- Java datapack `tick.json` namespaces, macros, and `#function` tags are a different format. Bedrock `tick.json` is only `{ "values": [ "path", ... ] }`.
- Script API can replace parts of this loop; `/scriptevent` is the official bridge. Commands remain valid without scripts.
