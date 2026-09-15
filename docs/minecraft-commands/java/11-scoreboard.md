# Java `/scoreboard`

Two roots in the tree: `objectives`, `players`. (No `teams` here — that is `/team`.)

## Objectives

```
scoreboard objectives add <objective> <criteria> [<display component>]
scoreboard objectives remove <objective>
scoreboard objectives list
scoreboard objectives setdisplay <slot> [<objective>]
scoreboard objectives modify <objective> displayname <component>
scoreboard objectives modify <objective> rendertype (hearts|integer)
scoreboard objectives modify <objective> numberformat (blank|fixed <component>|styled <style>)
```

Criteria include `dummy`, `trigger`, `deathCount`, `playerKillCount`, `totalKillCount`, `health`, `xp`, `level`, `food`, `air`, `armor`, `dummy`, and resource-shaped ones (`minecraft.custom:minecraft.jump`, `minecraft.killed:minecraft.zombie`, …). The parser is `minecraft:objective_criteria`.

## Players

```
scoreboard players list [<holder>]
scoreboard players get <holder> <objective>
scoreboard players set <holders> <objective> <int>
scoreboard players add <holders> <objective> <int>=0
scoreboard players remove <holders> <objective> <int>=0
scoreboard players reset <holders> [<objective>]
scoreboard players enable <holders> <objective>          (* trigger *)
scoreboard players operation <targets> <obj> <op> <source> <obj>
scoreboard players display name <holders> <obj> <component>
scoreboard players display numberformat <holders> <obj> (blank|fixed|styled)
```

`minecraft:score_holder` with `amount=multiple` accepts selectors and `*` (all tracked names).

## Operations (`minecraft:operation`)

`+=` `-=` `*=` `/=` `%=` `=` `<` (min) `>` (max) `><` (swap)

Integer division toward 0. `/` and `%` by 0 fail that holder.

## Display slots (`minecraft:scoreboard_slot`)

`list`, `sidebar`, `below_name` / `belowname` (tree uses the vanilla id), `sidebar.team.<color>`.

## Relation to `/execute if score`

Score compare in execute uses **single** holders. `matches` uses `minecraft:int_range`.
