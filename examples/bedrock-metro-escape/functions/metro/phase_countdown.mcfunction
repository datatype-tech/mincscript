scoreboard players add metro countdown 1
execute if score metro countdown matches 20 run title @a[tag=in_round] title 9
execute if score metro countdown matches 40 run title @a[tag=in_round] title 8
execute if score metro countdown matches 60 run title @a[tag=in_round] title 7
execute if score metro countdown matches 80 run title @a[tag=in_round] title 6
execute if score metro countdown matches 100 run title @a[tag=in_round] title 5
execute if score metro countdown matches 120 run title @a[tag=in_round] title 4
execute if score metro countdown matches 140 run title @a[tag=in_round] title 3
execute if score metro countdown matches 160 run title @a[tag=in_round] title 2
execute if score metro countdown matches 180 run title @a[tag=in_round] title 1
execute if score metro countdown matches 200 run function metro/begin_play
