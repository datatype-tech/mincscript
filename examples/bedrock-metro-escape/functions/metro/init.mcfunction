gamerule commandblockoutput false
gamerule sendcommandfeedback false
time set noon
weather clear
scoreboard players set metro phase 0
scoreboard players set metro countdown 0
scoreboard players set metro play_tick 0
scoreboard players set metro extract_timer 0
scoreboard players set metro end_tick 0
tickingarea add circle 32 64 0 4 metro_station true
schedule on_area_loaded add tickingarea metro_station metro/on_station_loaded
scoreboard players set metro inited 1
