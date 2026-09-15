scoreboard players add metro play_tick 1
execute as @a[tag=in_round,tag=!crate1,x=32,y=64,z=4,dx=2,dy=2,dz=2] run function metro/open_crate
execute as @a[tag=in_round,hasitem={item=gold_ingot,quantity=1..},tag=!metro_key] run function metro/on_key
execute if score metro phase matches 2..3 as @a[tag=in_round,tag=!metro_dead,x=-4,y=60,z=-4,dx=8,dy=10,dz=8] run function metro/on_death
execute if score metro play_tick matches 1200 run function metro/open_extract
execute unless entity @a[tag=in_round,tag=!extracted,tag=!metro_dead] run function metro/force_end
