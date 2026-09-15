summon npc 2 64 0
tag @e[type=npc,c=1] add metro_npc
dialogue change @e[type=npc,tag=metro_npc,c=1] metro_queue
tellraw @a {"rawtext":[{"text":"§7已召唤接待 NPC。请在创造里再调皮肤。"}]}
