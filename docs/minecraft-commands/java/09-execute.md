# Java `/execute` (complete chain grammar)

Verified against the `26.3-rc-3` brigadier tree and the Wiki Syntax tree (including the 26.3 `slots` condition, which the wiki flags as new).

## EBNF

```
execute       = "execute" , instr+ ;
instr         = modifier | store | condition | run ;

modifier      = align | anchored | as | at | facing | in | on
               | positioned | rotated | summon ;

align         = "align" , swizzle ;
anchored      = "anchored" , ( "eyes" | "feet" ) ;
as            = "as" , entity_multi ;
at            = "at" , entity_multi ;
facing        = "facing" , ( vec3 | "entity" , entity_multi , anchor ) ;
in            = "in" , dimension ;
on            = "on" , relation ;
positioned    = "positioned" , ( vec3 | "as" , entity_multi | "over" , heightmap ) ;
rotated       = "rotated" , ( rotation | "as" , entity_multi ) ;
summon        = "summon" , entity_type ;

relation      = "attacker" | "controller" | "leasher" | "origin"
               | "owner" | "passengers" | "target" | "vehicle" ;

heightmap     = "world_surface" | "motion_blocking"
               | "motion_blocking_no_leaves" | "ocean_floor" ;

store         = "store" , ( "result" | "success" ) , store_dest ;
store_dest    = "block" , block_pos , nbt_path , num_type , scale
              | "bossbar" , resource , ( "max" | "value" )
              | "entity" , entity_single , nbt_path , num_type , scale
              | "score" , score_holders , objective
              | "storage" , resource , nbt_path , num_type , scale ;
num_type      = "byte" | "short" | "int" | "long" | "float" | "double" ;
scale         = double ;

condition     = ( "if" | "unless" ) , cond ;
cond          = "biome" , block_pos , biome
              | "block" , block_pos , block_predicate
              | "blocks" , block_pos , block_pos , block_pos , ( "all" | "masked" )
              | "data" , data_src
              | "dimension" , dimension
              | "entity" , entity_multi
              | "function" , function          (* MUST continue; not executable alone *)
              | "items" , items_src , slot_source , item_predicate
              | "slots" , items_src , slot_source
              | "loaded" , block_pos
              | "predicate" , predicate
              | "score" , score_single , objective , score_cmp
              | "stopwatch" , id , int_range ;

data_src      = "block" , block_pos , nbt_path
              | "entity" , entity_single , nbt_path
              | "storage" , resource , nbt_path ;

items_src     = "block" , block_pos | "entity" , entity_multi ;

score_cmp     = ( "<" | "<=" | "=" | ">=" | ">" ) , score_single , objective
              | "matches" , int_range ;

run           = "run" , command ;   (* any root command, including another execute *)
```

`instr+` is required. The chain may end at:

1. `run <command>`
2. a **condition** that is `executable` in the tree (all `if`/`unless` except `function`)

A chain that ends on a modifier (`execute as @s`) is **unparseable**.

## Forking (Java is breadth-first)

- `as` / `at` / `positioned as` with multiple entities **forks**: later instructions run once per entity.
- Java processes the chain **left to right, one subcommand at a time** (breadth-first). `run` cannot affect earlier forks.
- `execute as @e run execute as @e run summon x` is **not** the same as Bedrock depth-first (see crosswalk). Nested `run execute` is a no-op wrapper.
- Depth-first in Java is done by putting the inner `execute` inside a **function**.

## Context vector (what each modifier changes)

| Subcommand | Executor `@s` | Position `~` | Rotation `^` | Dimension | Anchor |
| --- | --- | --- | --- | --- | --- |
| `as` | yes | no | no | no | no |
| `at` | no | yes | yes | yes | no |
| `positioned` | no | yes | no | no | resets to feet if `positioned <pos>` |
| `rotated` | no | no | yes | no | no |
| `facing` | no | no | yes | no | no |
| `in` | no | no | no | yes | no |
| `align` | no | floors selected axes | no | no | no |
| `anchored` | no | no | no | no | eyes/feet |
| `on` | yes (related entity) | no | no | no | no |
| `summon` | yes (new entity) | no | no | no | no |

`on passengers` forks per passenger. Missing relation → branch dies.

## Store

`success` is 0 or 1. `result` is command-specific (often a count). Values are written **per branch**. Same storage location: last branch wins (not summed). `/execute`’s own success count (command block comparator) **sums** branch successes.

`store` then `run function` has extra rules: function success/result may be unavailable; see [`13-functions.md`](13-functions.md).

## Conditions — semantics

| Condition | Success when |
| --- | --- |
| `block` | Block predicate matches |
| `blocks all` | Every block equals destination region |
| `blocks masked` | Non-air source blocks match |
| `data` | Path exists |
| `entity` | Selector matches ≥1 |
| `score … matches` | Score in range (holder must have the objective) |
| `score <op>` | Numeric compare |
| `predicate` | Predicate file true |
| `biome` | Pos in biome / tag |
| `dimension` | Execution dimension |
| `loaded` | Chunk loaded |
| `items` | Matching item in slot range |
| `slots` | Slot range exists / occupied (26.3; tree: `slot_source` only) |
| `function` | Function “succeeds”; **always** continues the chain |
| `stopwatch` | Stopwatch id’s value in range |

`unless` inverts the boolean. Failed condition **kills that branch**, not the whole command.

## `run`

The argument is a **full command** using the same dispatcher (leading `/` optional). `run execute …` is legal but redundant.

## Examples (must parse on Java)

```
execute as @a at @s if block ~ ~-1 ~ minecraft:stone run say on stone
execute in minecraft:the_nether as @p positioned over world_surface run tp @s ~ ~ ~
execute as @e[type=armor_stand] on vehicle run kill @s
execute store result score @p dummy if entity @e[type=sheep,distance=..16]
execute as @a if items entity @s weapon.mainhand diamond_sword run say armed
execute if slots entity @s container.* run say has inventory
execute summon marker run tag @s add spawned
```

Full generated expansions: [`commands/generated-usages.md`](commands/generated-usages.md) section `execute`.
