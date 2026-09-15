# Selectors: Java vs Bedrock

## Variables

| | Java | Bedrock |
| --- | --- | --- |
| `@p @a @e @s` | yes | yes (`@p` living-only) |
| `@n` | yes | **no** |
| `@r` | players only | players **or** `type=` |
| `@initiator` | **no** | yes |

## Arguments

| Concept | Java | Bedrock |
| --- | --- | --- |
| Distance | `distance=8..16` | `rm=8,r=16` |
| Count | `limit=3,sort=nearest` | `c=3` (`c=-3` furthest) |
| Gamemode | `gamemode=survival` | `m=survival` / `m=s` / `m=0` |
| Level | `level=8..16` | `lm=8,l=16` |
| Pitch / yaw | `x_rotation` `y_rotation` ranges | `rx`/`rxm` `ry`/`rym` |
| Scores | `{a=1..}` no `!` | `{a=1..}` and `{a=!5}` |
| Type tags | `#minecraft:skeletons` | no `#type` tags; use `family` |
| Family | no | `family=` |
| NBT | `nbt={…}` SNBT | **no** |
| Items | `/execute if items` | `hasitem={…}` |
| Team | `team=` | **no** |
| Predicate / advancements | yes | **no** |
| Origin integers | not center-corrected | center-corrected |
| `~` in selector xyz | no | yes |
| `^` in selector xyz | no | no |

## Compiler

Keep **edition-tagged selector AST**. A translator table can rewrite `distance=..10` → `r=10` but cannot rewrite `nbt=` or `predicate=`.
