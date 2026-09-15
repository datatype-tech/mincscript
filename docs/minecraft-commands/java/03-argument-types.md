# Java Edition argument types

Every `parser` value in the `26.3-rc-3` tree. Counts are occurrences in that tree (implementation checklist).

## Brigadier primitives

| Parser | Consumes | Notes |
| --- | --- | --- |
| `brigadier:bool` | `true` / `false` | Case-sensitive |
| `brigadier:integer` | signed 32-bit | Optional `min`/`max` in properties |
| `brigadier:float` | float | Optional min/max |
| `brigadier:double` | double | Optional min/max |
| `brigadier:string` | word / `"phrase"` / greedy rest | `properties.type` is `word`, `phrase`, or `greedy` |

## Positions, rotation, range

| Parser | Consumes | Notes |
| --- | --- | --- |
| `minecraft:block_pos` | `x y z` block-aligned | `~` / `^` per [`05-coordinates.md`](05-coordinates.md) |
| `minecraft:vec3` | three doubles | Entity-precision position |
| `minecraft:vec2` | `x z` | |
| `minecraft:column_pos` | `x z` column | |
| `minecraft:rotation` | `yaw pitch` | `~` allowed |
| `minecraft:swizzle` | unique subset of `x`/`y`/`z` | `xy`, `yzx`, `xyz` — no repeated axis |
| `minecraft:int_range` | `N`, `N..`, `..N`, `N..M` | Inclusive |
| `minecraft:float_range` | same with floats | Selectors `distance`, rotations |
| `minecraft:time` | `20`, `1s`, `1t`, `1d` | Converts to ticks |
| `minecraft:hex_color` | `#rrggbb` or integer | |

## Entities and players

| Parser | Properties | Notes |
| --- | --- | --- |
| `minecraft:entity` | `amount`: `single`/`multiple`; `type`: `players`/`entities` | Player names, UUIDs, or selectors |
| `minecraft:game_profile` | | Offline/online names for `/op`, `/ban` |
| `minecraft:score_holder` | `amount` | `*` is all score holders; `*` is a literal token |
| `minecraft:uuid` | dashed UUID | |
| `minecraft:entity_anchor` | `eyes` / `feet` | |
| `minecraft:gamemode` | | `survival` `creative` `adventure` `spectator` |

## Resources

| Parser | Notes |
| --- | --- |
| `minecraft:resource_location` | `ns:path` |
| `minecraft:resource` | Registry id, properties include registry |
| `minecraft:resource_key` | Registry key |
| `minecraft:resource_or_tag` | id or `#tag` |
| `minecraft:resource_or_tag_key` | Registry-backed tag |
| `minecraft:resource_selector` | Resource selector syntax (datapack tools) |
| `minecraft:dimension` | `overworld` / `the_nether` / `the_end` / custom |
| `minecraft:function` | Function id or `#tag` |
| `minecraft:dialog` | Dialog resource (26.x) |
| `minecraft:feature` | Worldgen feature |
| `minecraft:heightmap` | `world_surface` `motion_blocking` `motion_blocking_no_leaves` `ocean_floor` |
| `minecraft:particle` | Particle id plus extra args (dust, item, block, …) |
| `minecraft:loot_table` | Loot table id |
| `minecraft:loot_predicate` | Predicate id |
| `minecraft:loot_modifier` | Item modifier id |

## Blocks, items, text, NBT

| Parser | Notes |
| --- | --- |
| `minecraft:block_state` | `id[state=value]` |
| `minecraft:block_predicate` | Block id / tag / states / nbt predicate |
| `minecraft:item_stack` | `id[components]` for `/give` |
| `minecraft:item_predicate` | Predicate form for `/execute if items`, `/clear` |
| `minecraft:item_slot` | `weapon.mainhand`, `container.0`, … |
| `minecraft:slot_source` | Slot **range** source (`container.*`, `armor.*`, …) used by `if items` / `if slots` |
| `minecraft:nbt_compound_tag` | SNBT `{...}` |
| `minecraft:nbt_tag` | Any SNBT value |
| `minecraft:nbt_path` | [`10-data.md`](10-data.md) |
| `minecraft:component` | Text component |
| `minecraft:style` | Style-only component |
| `minecraft:message` | Greedy text; `@selectors` expanded to names |

## Scoreboard / teams / ops

| Parser | Notes |
| --- | --- |
| `minecraft:objective` | Objective name |
| `minecraft:objective_criteria` | `dummy`, `health`, `minecraft.killed:minecraft.zombie`, … |
| `minecraft:operation` | `+=` `-=` `*=` `/=` `%=` `=` `<` `>` `><` |
| `minecraft:scoreboard_slot` | `sidebar`, `list`, `belowname`, `sidebar.team.red`, … |
| `minecraft:team` | Team name |
| `minecraft:team_color` | Formatting color |
| `minecraft:template_mirror` | Structure mirror |
| `minecraft:template_rotation` | `none` `clockwise_90` `180` `counterclockwise_90` |
| `minecraft:swing_animation` | `/swing` |
| `minecraft:context_float_provider` | `/data modify … compute` |
| `minecraft:context_int_provider` | Same for ints |

If a future tree adds a parser not listed here, extraction will show it under `java/data/command-index.json` → `parsers`. Treat that as a hard compile error until a parser is implemented.
