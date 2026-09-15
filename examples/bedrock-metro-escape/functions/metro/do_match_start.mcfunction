scoreboard players set metro phase 1
scoreboard players set metro countdown 0
gamemode adventure @a[tag=queued]
tag @a[tag=queued] add in_round
spawnpoint @a[tag=in_round] 0 64 0
teleport @a[tag=in_round] 32 64 0
inputpermission set @a[tag=in_round] movement disabled
inputpermission set @a[tag=in_round] camera disabled
camera @a[tag=in_round] set minecraft:free ease 1.5 in_out_quad pos 24 72 8 facing 32 64 0
music play record.cat 1.0 2.0 play_once
title @a[tag=in_round] times 10 40 10
title @a[tag=in_round] title 地铁逃生
title @a[tag=in_round] subtitle 即将出发
