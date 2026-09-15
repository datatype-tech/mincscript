# State, clocks, and selector logic

Bedrock has no `/data`, no `/team`, no predicates, no `execute store`. The entire game program is:

- **integers** on dummy scoreboards (including fake players)
- **tags** on entities
- **selector predicates** (`hasitem`, `scores`, `r`, `m`, …)
- **`/execute` if/unless** as gates

That is enough. Official Complete-the-Monument and snowball fight are this model. 地铁逃生 is the same model with more phases.

## Dummy scoreboards are RAM that saves

Official add:

```
scoreboard objectives add <objective> dummy [displayName]
```

Only `dummy` is addable. Display: `sidebar` / `list` / `belowname`.

Scores persist in the world. Holders do **not** need to be online. A hash prefix (`#red`) hides a fake player from the sidebar. A namespace-looking name (`metro`, `.Timer`) is still just a string holder.

| Pattern | Example | Use |
| --- | --- | --- |
| World register | `scoreboard players set metro phase 2` | Match FSM |
| Per-player | `scoreboard players add @s metro_hold 1` | Extract channel time |
| Boolean via 0/1 | `#red wool_placed` 0 then 1 | “already rewarded” |
| RNG | `scoreboard players random @p roll 0 100` | Team split, crate rarity |
| Compare | `execute if score A obj matches 1..` | Gates |
| Arithmetic | `operation <t> <obj> += <s> <obj>` | Totals, clocks, modulo timers |
| Inclusive test | `scoreboard players test <p> <obj> <min> [max]` | Chain-conditional CBs; `*` = unbounded |

`players test` is Bedrock-only. New function code should prefer `execute if score … matches` so the same line both tests and `run`s.

`players add <holder> obj 0` creates the score at 0 if missing and leaves an existing value alone — useful after `objectives add`.

Selector: `@a[scores={metro_hold=60..}]`. Bedrock allows `!` on score ranges; Java does not.

There is **no** `deathCount` criterion. Detect death with space (respawn at lobby spawnpoint) or Script API.

## Tags are facts

```
tag <entity> add|remove <name>
tag <entity> list
```

Official snowball fight uses `team1` / `team2` tags because `/team` does not exist. 地铁逃生 uses the same: `queued`, `in_round`, `metro_key`, `extracted`, `metro_dead`, `crate1`.

AND in one selector: `@a[tag=in_round,tag=metro_key,tag=!extracted]`.

Clear on reset: `tag @a remove in_round` (one tag per command).

## Clocks

Functions have no “Delay in Ticks.” Two official-compatible clocks:

### A. Phase timer (countdown / extract window)

Each tick, while `phase` matches:

```
scoreboard players add metro countdown 1
execute if score metro countdown matches 200 run function metro/begin_play
```

20 ticks ≈ 1 second. Milestone titles at 20, 40, … are just more `matches` lines.

### B. Repeating interval (BCC scoreboard timer)

```
scoreboard players add .Timer metro_ticks 1
scoreboard players operation * metro_events = .Timer metro_ticks
scoreboard players operation .ExtractWarning metro_events %= .30s metro_ticks
execute if score .ExtractWarning metro_events matches 0 run title @a actionbar 撤离点即将刷新
```

`.30s` is a fake player whose score is `600`. Official `operation` `%=` is floor modulo.

### C. Delayed one-shot

```
schedule delay add metro/open_extract 1200
```

Official `DelayMode`: `append` (several pending) or `replace`. `/schedule delay clear <fn>` / `/schedule clear <fn>`. This is not a substitute for the match FSM; use it for “after the area loads” and cutscenes.

### D. Per-entity async timer

```
scoreboard players add @e[type=npc,scores={metro_ticks=0..}] metro_ticks 1
execute as @e[type=npc,scores={metro_ticks=6000}] run event entity @s minecraft:despawn
```

Set score `-1` to stop, `0` to restart (BCC entity-timer pattern).

## `/execute` is the ALU

Official chain: modifiers (`as` `at` `in` `positioned` `rotated` `facing` `align` `anchored`) then conditions (`if`/`unless` `block` `blocks` `entity` `score`) then `run <command>`.

No `store` / `on` / `summon` subcommand / `if data` / `predicate` / `items` / `slots` / `loaded` / `biome`.

Forking is **depth-first**. Nested `as @e` does not match Java order.

### Selector algebra (BCC gates)

These are the same tables community maps use. Inputs can be `tag`, `scores`, `hasitem`, `type`, `family`, `name`.

| Gate | Form |
| --- | --- |
| Buffer | `execute if entity @s[tag=red] run …` |
| NOT | `execute if entity @s[tag=!red] run …` |
| AND | `execute if entity @s[tag=red,tag=green] run …` |
| NAND | `execute unless entity @s[tag=red,tag=green] run …` |
| OR | `execute unless entity @s[tag=!red,tag=!green] run …` |
| NOR | `execute if entity @s[tag=!red,tag=!green] run …` |
| XOR | OR then NAND: two `unless` clauses |

Inventory OR (key **or** extract pass):

```
execute unless entity @s[hasitem=[{item=gold_ingot,quantity=0},{item=compass,quantity=0}]] run function metro/can_extract
```

`hasitem` `quantity=0` means “does not have.” Combined with `unless` + both zero → has at least one.

AND for extract (in zone **and** has key **and** in round):

```
execute as @a[tag=in_round,tag=metro_key,tag=!extracted,x=64,y=64,z=0,dx=3,dy=3,dz=3] run scoreboard players add @s metro_hold 1
```

That single selector **is** the AND gate. Prefer it over nested `if entity`.

## Spatial tests

| Need | Bedrock |
| --- | --- |
| Radius | `@a[x=64,y=64,z=0,r=4]` (`rm` min). Tilde allowed in selector xyz; **caret is not**. Integer xyz is **center-corrected**. |
| Room AABB | `x y z dx dy dz` — omitted axis is **0**, not infinite |
| Standing on block | `execute as @a at @s if block ~ ~-1 ~ gold_block run …` |
| Nearest / furthest | `c=1` / `c=-1` (negative reverses) |
| Family | `@e[family=monster]` — Java has no equivalent |

Do not write `distance=` / `limit=` / `sort=`.

## Inventory tests

`hasitem` is the Bedrock replacement for Java `execute if items`. Required key `item`. Optional `data`, `quantity` (default `1..`; `0` = none), `location` + `slot`.

```
@a[hasitem={item=gold_ingot,quantity=1..}]
@s[hasitem={item=stick,location=slot.weapon.mainhand}]
@a[hasitem=[{item=bread,quantity=1..},{item=apple}]]
```

Give / take:

```
give <player> <Item> [amount] [data] [components JSON]
clear [player] [item] [data] [maxCount]
replaceitem entity <target> slot.hotbar.0 0 cooked_beef 8
```

`data` is aux (Java has no aux). Keep components to the documented subset (`can_place_on`, `can_destroy`, `item_lock`, `keep_on_death`). `item_lock` is how adventure maps stop players from dropping keys.

`/loot give @s loot <table>` fills from a loot table instead of a hardcoded `/give`. `/loot insert <chestPos> loot …` fills a crate without a player.

## Teams, health, “who is alive”

No `/team`. Use tags + `/effect` + `/gamemode`.

No `execute store` into a health score. Approximate:

- Death: `spawnpoint` at lobby; during `phase` 2–3, anyone with `in_round` back in the lobby AABB is dead.
- Downed: `effect` + `inputpermission set @s movement disabled` + tag `downed`.
- Spectate: `gamemode spectator @s` (official enum includes `spectator`).

## Presentation without Java text components

```
tellraw @a {"rawtext":[{"text":"§e撤离点已开启"}]}
titleraw @a actionbar {"rawtext":[{"score":{"name":"metro","objective":"countdown"}}]}
title @a times 10 40 10
title @a title 地铁逃生
```

`/title` is **plain**. JSON is `/titleraw` / `/tellraw` with `rawtext` only.

Cutscene lock (official `/inputpermission`):

```
inputpermission set @a[tag=in_round] movement disabled
inputpermission set @a[tag=in_round] camera disabled
```

Permissions: `camera`, `movement`, `jump`, `lateral_movement`, `sneak`, `dismount`, `mount`, `move_*`. States `enabled` / `disabled`. Re-enable on `begin_play`.

Camera presets (official enum): `minecraft:first_person`, `third_person`, `third_person_front`, `free`, `follow_orbit`, `fixed_boom`, `control_scheme_camera`.

```
camera @a[tag=in_round] set minecraft:free ease 1.5 in_out_quad pos 8 72 8 facing 32 64 0
camera @a set minecraft:first_person
```

`/music play|queue|stop|volume`, `/fog push|pop`, `/camerashake`, `/playanimation`, `/hud` complete the broadcast layer. They are **not** the FSM. Re-apply after join.

## NPC and `@initiator`

`/give @p spawn_egg 1 51` (official). One NPC can run **multiple** commands. Buttons: `tag @initiator add queued`. `@initiator` is **not** valid on Java.

Scene files live in the pack (`dialogue/`). `/dialogue open <npc> <player> [scene]` can pop UI without clicking (NPC must still be in a loaded ticking chunk). `/dialogue change` retargets later talk (quest flags).

## Script bridge

```
scriptevent metro:extract_success @s
```

Use when guns, custom entities, or UI go beyond the 82 commands. The match FSM can stay in functions.

## Function hygiene

- One concern per file (`phase_playing` calls `open_crate`, `on_key`, …).
- All commands in one function run **in the same tick**, in order, including nested `/function`.
- Too many files cost memory on devices (official performance guidance). A 地铁逃生 loop should be a handful of functions, not thousands.
- `min_engine_version` pins **command version** inside functions. Old packs keep old execute.

## What not to invent

| Java habit | Bedrock |
| --- | --- |
| `execute store result score` | `scoreboard players` after a known event; or `scriptevent` |
| `execute if data` | `hasitem` / entity exists / block |
| `/team` | `tag` |
| `predicate` | function + selector |
| Function macros | not present |
| `/return` | skip with `execute if` around the rest |
