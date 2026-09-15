# Bedrock metro-escape example pack

Drop-in **behavior pack** that actually runs: `functions/tick.json` calls `metro/tick` every gameplay tick. This is the executable skeleton for [`docs/minecraft-commands/bedrock/gameplay/`](../../docs/minecraft-commands/bedrock/gameplay/README.md).

It is not a finished gun map. It **is** a complete match FSM using only official Bedrock command signatures: lobby → countdown → loot/key → extract hold → reset.

## Apply

1. Copy this folder into `com.mojang/development_behavior_packs/` (or the world’s `behavior_packs/`).
2. Create/open a world, **Activate Cheats**, enable the pack, `min_engine_version` is already `1.21.0` (new `/execute`).
3. Build the rooms at the default coords (or change every `x y z` in `functions/metro/`):

| Place | Coords | Role |
| --- | --- | --- |
| Lobby | `0 64 0` | Death/respawn. Stand on a **gold block** to queue; queued players start the match |
| Reset dump | `3 64 0` | After a round, so nobody is still standing on the start gold |
| Station spawn | `32 64 0` | Teleport on match start |
| Crate AABB | `32 64 4` size `dx=2 dy=2 dz=2` | First walk-in gives loot + a gold ingot key |
| Extract AABB | `64 64 0` size `3×3×3` | Stand 3 seconds with key (`metro_hold` 60) |

4. Optional: `/function metro/save_station` after you build the station (saves `disk` structure `metro_station`).
5. Optional NPC: `/give @p spawn_egg 1 51`, place at lobby, `/tag @e[type=npc,c=1] add metro_npc`, then `/dialogue change @e[tag=metro_npc,c=1] metro_queue`. Scene file: `dialogue/metro_lobby.json`.
6. `/reload` after edits. Chat `/function metro/init` if you need to re-run world setup (ticking area, gamerules).

## Prove it executes

- You must see titles / actionbars without typing commands every tick. If not, the pack is not in the stack or `tick.json` is missing.
- `/scoreboard objectives list` shows `metro`.
- `/tickingarea list` shows `metro_station` after init (once a player has triggered bootstrap).
- Function files **must not** start lines with `/`. A syntax error unloads the whole function (content log).

## Layout

```
manifest.json
dialogue/metro_lobby.json
functions/tick.json
functions/metro/*.mcfunction
```
