# tick.json 每刻入口。objective 不存在时 if/unless score 都不会成功，所以无条件 add。
scoreboard objectives add metro dummy
scoreboard objectives add metro_hold dummy
scoreboard players add metro inited 0
execute if score metro inited matches 0 run function metro/init
execute if score metro inited matches 1 run function metro/loop
