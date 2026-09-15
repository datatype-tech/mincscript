# Bedrock argument types and enums

Taken from the official command index + per-command pages. Enums are closed sets unless noted.

## Common enums

| Enum | Values |
| --- | --- |
| `Boolean` | `true` `false` |
| `Difficulty` | `peaceful`/`p`/`0`, `easy`/`e`/`1`, `normal`/`n`/`2`, `hard`/`h`/`3` |
| `GameMode` | `survival`/`s`/`0`, `creative`/`c`/`1`, `adventure`/`a`/`2`, `default`/`d`/`5` (spectator where enabled) |
| `Dimension` | `overworld` `nether` `the_end` |
| `ActorLocation` | `eyes` `feet` |
| `BlocksScanMode` | `all` `masked` |
| `Option_If_Unless` | `if` `unless` |
| `CloneMode` | `normal` `force` `move` |
| `MaskMode` | `replace` `masked` `filtered` |
| `StructureRotation` | `0_degrees` `90_degrees` `180_degrees` `270_degrees` |
| TimeSpec | `day` `noon` `sunset` `night` `midnight` `sunrise` (wiki/official `/time`) |

## Scoreboard operators

Official `/scoreboard players operation` uses `+= -= *= /= %= = < > ><` like Java.

`execute if score` compareoperator is `< <= = >= >`.

## Integer ranges

`matches <range: fullintegerrange>`: `N`, `N..`, `..N`, `N..M` (same idea as Java). Selector `quantity` on `hasitem` also uses this, plus `!`.

## Targets

`CommandSelector<Player>` vs `CommandSelector<Actor>`: player-only vs any entity. `@e` is illegal where a player selector is required (`/give`).

## Items vs Java

`/give <player> <itemName: Item> [amount] [data] [components]`

- `data` is the **aux value** (0–32767), not Java components.
- Potions: aux selects effect (`give @s potion 1 5`).
- `components` JSON supports a **small** set: `minecraft:can_place_on`, `minecraft:can_destroy`, `minecraft:item_lock`, `minecraft:keep_on_death`.

Do not emit Java `diamond_sword[enchantments=…]` on Bedrock.
