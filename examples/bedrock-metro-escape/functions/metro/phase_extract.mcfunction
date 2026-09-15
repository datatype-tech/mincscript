scoreboard players remove metro extract_timer 1
execute as @a[tag=in_round,tag=metro_key,tag=!extracted,x=64,y=64,z=0,dx=3,dy=3,dz=3] run scoreboard players add @s metro_hold 1
execute as @a[tag=in_round] unless entity @s[x=64,y=64,z=0,dx=3,dy=3,dz=3] run scoreboard players set @s metro_hold 0
execute as @a[tag=in_round,scores={metro_hold=60..},tag=!extracted] run function metro/do_extract
execute if score metro phase matches 2..3 as @a[tag=in_round,tag=!metro_dead,x=-4,y=60,z=-4,dx=8,dy=10,dz=8] run function metro/on_death
execute if score metro extract_timer matches 0 run function metro/force_end
execute unless entity @a[tag=in_round,tag=!extracted,tag=!metro_dead] run function metro/force_end
