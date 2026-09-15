# Bedrock `/execute` (new syntax)

Official signatures (Microsoft Learn + generated `execute.md`). Old `execute <origin> <pos> detect … <command>` is **removed** (Bedrock 1.19.50+ new execute). Do not parse the old form.

## EBNF (matches official lines)

```
execute     = "execute" , chain ;
chain       = instr , { instr } ;
instr       = modifier | condition | run ;

modifier    = "as" , target
             | "at" , target
             | "in" , dimension
             | "positioned" , vec3
             | "positioned" , "as" , target
             | "rotated" , yaw , pitch
             | "rotated" , "as" , target
             | "facing" , vec3
             | "facing" , "entity" , target , ( "eyes" | "feet" )
             | "align" , axes
             | "anchored" , ( "eyes" | "feet" ) ;

condition    = ( "if" | "unless" ) , cond ;
cond        = "block" , vec3 , Block , [ block_states ]
             | "blocks" , vec3 , vec3 , vec3 , ( "all" | "masked" )
             | "entity" , target
             | "score" , target , objective , compare , target , objective
             | "score" , target , objective , "matches" , int_range ;

run         = "run" , command ;
```

Official type name for “another execute instruction” is `executechainedoption_0` / `ExecuteChainedOption_0`. After every modifier the next token is that chained option (required). After conditions, the chained option is **optional** in the official grammar (`[chainedCommand]`), so a trailing `if` is a valid command.

There is **no** `store`, `on`, `summon`, `predicate`, `data`, `biome`, `dimension` (as if-condition; `in` exists as modifier), `items`, `slots`, `loaded`, `function`, `stopwatch`, `positioned over`.

## Forking (Bedrock is depth-first)

Wiki + bugs MC-125067 / MCPE-165278:

Given two armor stands A,B:

`execute as @e[type=armor_stand] as @e[type=armor_stand] run summon armor_stand`

- **Java:** breadth-first (4 summons in a different order/structure)
- **Bedrock:** depth-first (nested per entity)

MincScript must compile execute **per edition**, not share a walker.

## Context

Same qualitative table as Java for `as` / `at` / `positioned` / `rotated` / `facing` / `in` / `align` / `anchored`, minus `on`/`summon`/`store`.

`align` axes: permutation of `x`/`y`/`z` without repeats (`xyz`, `xz`, …).

## Score conditions

```
execute if score <target> <obj> = <source> <obj> run …
execute if score <target> <obj> matches 1..10 run …
```

Operators: `< <= = >= >`. Holders are `target` (selectors ok). This replaces a large part of Java `execute if data` / predicates.

## `run`

`command: codebuilderargs` is any Bedrock command **without** a second leading execute wrapper requirement. `run say hi` is valid.

## Examples (official)

```
execute as @s positioned as @s if block ~ ~-1 ~ grass run say Player is standing on grass.
execute as @a at @s if block ~ ~-1 ~ grass run say Standing on grass.
```

## Parser tests

- `execute store …` must **fail** on Bedrock.
- `execute on vehicle` must **fail**.
- `execute if data …` must **fail**.
- `execute if entity @e run say x` must parse.
- Old `execute @p ~ ~ ~ detect ~ ~-1 ~ stone say hi` must **fail**.
