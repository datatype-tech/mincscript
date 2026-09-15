# Java Edition coordinates and rotation

## World frame

- **+X** east, **+Y** up, **+Z** south.
- Block position = `floor` of each axis (block origin is the lower north-west corner).
- Entity position is the feet (hitbox bottom center). Eyes are ~1.62 above feet for players; `anchored eyes` uses that.

## Three writings

| Form | Token | Meaning |
| --- | --- | --- |
| Absolute | `12.5` | World coordinate |
| Relative | `~` or `~3` | Offset from **execution position** |
| Local | `^` or `^2` | Offset in the executor’s look frame: `^sway ^heave ^surge` (left, up, forward) |

`execute positioned` / `at` / `anchored` change what `~` and `^` mean.

## Vec parsers

| Parser | Count | Typical use |
| --- | --- | --- |
| `minecraft:block_pos` | 3 | Blocks (`setblock`, `if block`) |
| `minecraft:vec3` | 3 | Entity teleport, `positioned` |
| `minecraft:vec2` | `x z` | Worldborder center |
| `minecraft:column_pos` | `x z` | Spreadplayers |
| `minecraft:rotation` | `yaw pitch` | `rotated`, `tp … facing` |

## Local vs world mixing

Illegal: `^1 ~2 3`. Legal: `~1 ~ ~-2`, `1 64 ~`, `^ ^ ^5`.

## Rotation numbers

Yaw: 0 south, 90 west, ±180 north, −90 east. Pitch: −90 up, 90 down.

`facing entity <t> eyes` aims at eye height; `feet` at feet.
