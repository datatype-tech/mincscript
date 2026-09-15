inputpermission set @a[tag=in_round] movement enabled
inputpermission set @a[tag=in_round] camera enabled
camera @a set minecraft:first_person
title @a[tag=in_round] title 出发
playsound random.levelup @a[tag=in_round]
scoreboard players set metro phase 2
scoreboard players set metro play_tick 0
