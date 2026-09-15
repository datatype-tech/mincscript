# Java Edition target selectors

## Variables

| Variable | Meaning | Default sort / limit |
| --- | --- | --- |
| `@p` | nearest player | `sort=nearest`, `limit=1` |
| `@n` | nearest **entity** (alive) | nearest 1 |
| `@r` | random **player** | `sort=random`, `limit=1` |
| `@a` | all online players (dead included) | no limit |
| `@e` | all loaded alive entities + alive players | no limit |
| `@s` | executor | fails if the source is not an entity |

Java does **not** have `@initiator`, `@c`, `@v`. Random **non-player** entities: `@e[sort=random,limit=1]`, not `@r[type=…]` (that is Bedrock).

Player names and UUIDs are also valid `minecraft:entity` values and are **not** selectors.

## Grammar

```
selector      = "@" , var , [ "[" , [ arg , { "," , arg } ] , "]" ] ;
arg           = key , "=" , value ;
key           = ident ;
value         = "!" , atom | atom ;
atom          = range | compound | quoted | unquoted ;
```

No space between `@e` and `[`. Spaces **are** allowed around `=` and `,` **inside** the brackets. Arguments are AND-ed. Keys are case-sensitive.

`!` negates where the argument allows it (`type=!cow`, `gamemode=!creative`, `tag=!x`, `nbt=!{…}`, `predicate=!id`).

## Argument map (Java only)

| Key | Value | Repeat | Notes |
| --- | --- | --- | --- |
| `x` `y` `z` | double | each once | Origin for distance/volume/sort. **Not** center-corrected (`x=0` is 0.0, not 0.5). Limits search to the **current dimension**. |
| `distance` | float range | once | Euclidean from feet to origin. `10`, `..10`, `10..`, `8..16` |
| `dx` `dy` `dz` | double | each once | Hitbox vs cuboid. Missing axes default to `0`. Volume is at least 1 block on each specified axis (game adds 1.0 to the positive corner). |
| `scores` | `{obj=range,…}` | once | Java ranges; **no** `!` on scores (that is Bedrock) |
| `tag` | string / empty | many | `tag=` means zero tags; `tag=!` means at least one |
| `team` | team / empty | eq once; `!` many | `team=` = no team |
| `name` | string | eq once; `!` many | Quoted if spaces |
| `type` | id or `#tag` | positive once; `!` many | **Illegal** on `@a` `@p` `@r` |
| `predicate` | resource | many | AND |
| `nbt` | SNBT compound | many | Heavy; prefer `tag=` |
| `advancements` | `{id=bool}` or `{id={crit=bool}}` | once | Players only |
| `level` | int range | once | Players only |
| `gamemode` | `survival` `creative` `adventure` `spectator` | eq once; `!` many | Players only. No `s`/`c`/`a` shorthand (Bedrock). No `default`. |
| `x_rotation` | float range | once | Pitch −90..90 |
| `y_rotation` | float range | once | Yaw −180..180, 0 = south |
| `limit` | positive int | once | |
| `sort` | `nearest` `furthest` `random` `arbitrary` | once | Defaults: `@p` nearest, `@r` random, `@e`/`@a` arbitrary |

Java has **no** `r`/`rm`, `c`, `l`/`lm`, `m`, `family`, `hasitem`, `haspermission`, `has_property`. Using them is a parse error.

## Type / player-type rules

`minecraft:entity` properties in the tree:

- `type=players` — names, UUIDs, `@a/@p/@r/@s` if the executor is a player; `@e[type=player]`
- `type=entities` — any entity
- `amount=single` vs `multiple` — `@a` is illegal when `single` is required (`/kill` allows multiple; `/data get entity` is single)

## Examples that must parse

```
@p
@n[type=minecraft:item]
@e[type=#minecraft:skeletons,distance=..16,sort=nearest,limit=1]
@a[gamemode=survival,level=10..,advancements={story/smelt_iron=true}]
@e[nbt={OnGround:1b},tag=keep]
@s[predicate=ns:in_arena,predicate=!ns:debug]
```
