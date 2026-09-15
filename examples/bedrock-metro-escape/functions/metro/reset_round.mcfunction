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
camera @a set minecraft:first_person
teleport @a 3 64 0
structure load metro_station 16 56 -16
scoreboard players set metro phase 0
scoreboard players set metro countdown 0
scoreboard players set metro play_tick 0
scoreboard players set metro extract_timer 0
scoreboard players set metro end_tick 0
title @a title 大厅
tellraw @a {"rawtext":[{"text":"§7已重置。金块再次开始。"}]}
