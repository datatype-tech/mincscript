execute as @a at @s if block ~ ~-1 ~ gold_block run tag @s add queued
execute if score metro phase matches 0 if entity @a[tag=queued] run function metro/match_start
