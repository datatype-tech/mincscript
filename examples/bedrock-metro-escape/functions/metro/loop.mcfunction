execute as @a[tag=!metro_joined] run function metro/on_join
execute if score metro phase matches 0 run function metro/phase_lobby
execute if score metro phase matches 1 run function metro/phase_countdown
execute if score metro phase matches 2 run function metro/phase_playing
execute if score metro phase matches 3 run function metro/phase_extract
execute if score metro phase matches 4 run function metro/phase_end
