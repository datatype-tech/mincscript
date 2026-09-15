# Combinable Bedrock recipes (official syntax)

Every snippet is valid against the 82-command generator signatures. Lines are shown as in **`.mcfunction` (no `/`)**. Cross-wire by calling `function` from `tick.json` or from another function. Placeholders like `32 64 0` are the example station.

Do not paste these into Java datapacks.

## 0. Hosts

### tick.json

```json
{ "values": ["metro/tick"] }
```

### Delay / load wait

```
schedule delay add metro/open_extract 1200 replace
schedule on_area_loaded add tickingarea metro_station metro/on_station_loaded
schedule delay clear metro/open_extract
```

### Named ticking area (persists in the world)

```
tickingarea add circle 32 64 0 4 metro_station true
tickingarea list
tickingarea preload metro_station true
```

### Round snapshot

```
structure save metro_station 16 56 -16 48 80 16 disk
structure load metro_station 16 56 -16
structure load metro_station 16 56 -16 0_degrees none true true
```

## 1. Bootstrap (tick-safe)

```
scoreboard objectives add metro dummy
scoreboard players add metro inited 0
execute if score metro inited matches 0 run function metro/init
execute if score metro inited matches 1 run function metro/loop
```

`init` (once):

```
gamerule commandblockoutput false
gamerule sendcommandfeedback false
scoreboard objectives add metro_timer dummy
scoreboard objectives add metro_hold dummy
scoreboard players set metro phase 0
scoreboard players set metro inited 1
tickingarea add circle 32 64 0 4 metro_station true
schedule on_area_loaded add tickingarea metro_station metro/on_station_loaded
```

Join:

```
execute as @a[tag=!metro_joined] run function metro/on_join
```

## 2. Phase dispatch

```
execute if score metro phase matches 0 run function metro/phase_lobby
execute if score metro phase matches 1 run function metro/phase_countdown
execute if score metro phase matches 2 run function metro/phase_playing
execute if score metro phase matches 3 run function metro/phase_extract
execute if score metro phase matches 4 run function metro/phase_end
```

## 3. Lobby match (gold block + tag)

Official CTM idea, new execute:

```
execute as @a at @s if block ~ ~-1 ~ gold_block run tag @s add queued
execute if score metro phase matches 0 if entity @a[tag=queued] run function metro/match_start
```

NPC button (scene JSON `commands`):

```
tag @initiator add queued
tellraw @initiator {"rawtext":[{"text":"§a已加入匹配"}]}
```

Open that scene without a click (NPC must be loaded):

```
dialogue open @e[type=npc,tag=metro_npc,c=1] @p metro_queue
```

## 4. Cutscene lock + camera + music

```
inputpermission set @a[tag=in_round] movement disabled
inputpermission set @a[tag=in_round] camera disabled
camera @a[tag=in_round] set minecraft:free ease 1.5 in_out_quad pos 8 72 8 facing 32 64 0
music play record.cat 1.0 2.0 play_once
title @a[tag=in_round] times 10 40 10
title @a[tag=in_round] title 地铁逃生
title @a[tag=in_round] subtitle 即将出发
```

Unlock:

```
inputpermission set @a[tag=in_round] movement enabled
inputpermission set @a[tag=in_round] camera enabled
camera @a set minecraft:first_person
```

`MusicRepeatMode` includes `play_once` / loop-style values from `/help music`. If a track id is missing, the command still parses; swap the string.

## 5. Scoreboard countdown (10 s)

```
scoreboard players add metro countdown 1
execute if score metro countdown matches 20 run title @a[tag=in_round] title 9
execute if score metro countdown matches 200 run function metro/begin_play
```

Interval warning (modulo clock):

```
scoreboard players add .Timer metro_timer 1
scoreboard players operation .Warn metro_timer = .Timer metro_timer
scoreboard players operation .Warn metro_timer %= .sec20 metro_timer
execute if score .Warn metro_timer matches 0 run title @a[tag=in_round] actionbar 搜刮物资并寻找钥匙
```

Set `.sec20 metro_timer` to `400` once in `init` for a 20 s beep. Keep the divisor on a **different holder** than `.Timer` or the modulo will clobber the clock — BCC uses two objectives (`ticks` vs `events`) for that reason.

## 6. Crate (reward ring)

```
execute as @a[tag=in_round,tag=!crate1,x=32,y=64,z=4,dx=2,dy=2,dz=2] run function metro/open_crate
```

`open_crate`:

```
give @s iron_ingot 4
give @s bread 2
give @s gold_ingot 1
tag @s add crate1
playsound random.chestopen @s
particle minecraft:villager_happy 32 65 4
titleraw @s actionbar {"rawtext":[{"text":"§a物资箱已搜刮"}]}
```

Loot-table variant:

```
loot give @s loot chests/simple_dungeon
loot insert 32 64 4 loot chests/simple_dungeon
```

Locked key item (components subset):

```
give @s gold_ingot 1 0 {"item_lock":{"mode":"lock_in_inventory"},"keep_on_death":{}}
```

Confirm `/help give` for the exact JSON your engine version accepts; do not emit Java `[minecraft:custom_data={}]`.

## 7. Key + door AND

```
execute as @a[tag=in_round,hasitem={item=gold_ingot,quantity=1..},tag=!metro_key] run function metro/on_key
```

```
execute if entity @a[tag=metro_key,x=40,y=64,z=0,dx=2,dy=3,dz=2] run fill 40 64 1 40 66 1 air
```

OR (key or compass pass):

```
execute as @a[tag=in_round] unless entity @s[hasitem=[{item=gold_ingot,quantity=0},{item=compass,quantity=0}]] run tag @s add can_extract
```

## 8. Extract channel (AND + hold timer + leave reset)

```
execute as @a[tag=in_round,tag=metro_key,tag=!extracted,x=64,y=64,z=0,dx=3,dy=3,dz=3] run scoreboard players add @s metro_hold 1
execute as @a[tag=in_round] unless entity @s[x=64,y=64,z=0,dx=3,dy=3,dz=3] run scoreboard players set @s metro_hold 0
execute as @a[tag=in_round,scores={metro_hold=60..},tag=!extracted] run function metro/do_extract
```

`do_extract`:

```
tag @s add extracted
gamemode spectator @s
title @s title 撤离成功
playsound random.levelup @s
scriptevent metro:extracted extracted
```

Window:

```
scoreboard players add metro play_tick 1
execute if score metro play_tick matches 1200 run function metro/open_extract
```

End if nobody left in play:

```
execute unless entity @a[tag=in_round,tag=!extracted,tag=!metro_dead] run function metro/force_end
```

## 9. Death via lobby AABB

```
spawnpoint @a[tag=in_round] 0 64 0
execute if score metro phase matches 2..3 as @a[tag=in_round,tag=!metro_dead,x=-4,y=60,z=-4,dx=8,dy=10,dz=8] run function metro/on_death
```

```
tag @s add metro_dead
gamemode spectator @s
title @s title 行动失败
```

## 10. Teams without `/team`

```
tag @p[r=2] remove team2
tag @p[r=1] add team1
```

Random split (fixed official sample):

```
scoreboard objectives add roll dummy
scoreboard players random @p roll 0 100
execute if score @p roll matches 0..50 run tag @p add team1
execute if score @p roll matches 51..100 run tag @p add team2
```

## 11. Snowball-style projectile (family × radius × damage)

```
execute as @e[type=snowball] at @s as @e[family=player,c=1,r=1] run damage @s 2
```

Prefer `at @s` on the snowball (the official sample’s double `at @e[family=player]` is harder to reason about). `c=1` = nearest.

## 12. Presentation pack

```
tellraw @a {"rawtext":[{"text":"§e撤离点已开启"}]}
titleraw @a actionbar {"rawtext":[{"text":"钥匙 "},{"selector":"@a[tag=metro_key]"}]}
fog @a push minecraft:fog_default metro_extract
camerashake add @a 0.2 1 positional
hud @a hide
playsound note.pling @a 64 64 0 1 1
```

`/fog` `push` needs a real `fogId`; `pop`/`remove` uses the `userProvidedId` (`metro_extract`). `/hud` visibility enum is `hide` / `reset` (see `/help hud`).

## 13. Reset

```
tag @a remove queued
tag @a remove in_round
tag @a remove metro_key
tag @a remove crate1
tag @a remove extracted
tag @a remove metro_dead
tag @a remove can_extract
scoreboard players reset @a metro_hold
clear @a
gamemode adventure @a
inputpermission set @a movement enabled
inputpermission set @a camera enabled
teleport @a 0 64 0
structure load metro_station 16 56 -16
scoreboard players set metro phase 0
scoreboard players set metro countdown 0
scoreboard players set metro play_tick 0
```

## 14. Command-block equivalent (only if you must)

Repeat Always Active → Chain Conditional Always Active:

1. `testforblock 0 4 0 diamond_block`
2. `testfor @p[tag=!placed_block]`
3. `give @p emerald`
4. `tag @p add placed_block`

Put the chain inside a ticking area or it dies when players leave. Prefer recipe 6.

## Cross-combo cheat sheet

| You want | Wire |
| --- | --- |
| If A and B | one selector with both args, or Conditional chain |
| If A or B | `unless entity @s[a=!…,b=!…]` |
| After N seconds | `scoreboard players add` + `matches 20*N` or `schedule delay` |
| Once per player | `tag=!done` then `tag add done` |
| Once per world | fake player 0→1 |
| Far logic | `tickingarea` + `on_area_loaded` |
| UI talk | `dialogue` + `@initiator` |
| Guns/custom | keep FSM here, `scriptevent` out |
