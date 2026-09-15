# Bedrock target selectors

Official: MicrosoftDocs `TargetSelectors.md`. Cross-checked with Minecraft Wiki Target selectors.

## Variables

| Variable | Official meaning |
| --- | --- |
| `@p` | nearest **living** player |
| `@a` | all online players |
| `@r` | one random **living** player unless `type` is set |
| `@e` | all entities |
| `@s` | executor |
| `@initiator` | player who clicked an NPC dialogue button |

No `@n`. `@r[type=minecraft:cow]` **is** valid (Java forbids this).

## Grammar

```
@<p|a|r|e|s|initiator>[k=v,k=v]
```

AND across parameters. Negation with `!` where listed. Multiple `type=!` / `tag=` / `family=` allowed as in the official table.

## Parameter map (Bedrock)

| Key | Official | Java equivalent |
| --- | --- | --- |
| `x y z` | origin; **tilde ok, caret no**; integers center-corrected | `x y z` (no tilde, no center-correct) |
| `r` | max radius | `distance=..r` |
| `rm` | min radius | `distance=rm..` |
| `dx dy dz` | cuboid; unspecified axes **0** | same |
| `c` | count; **negative reverses order** (`@p[c=-1]` furthest) | `limit`+`sort` |
| `type` | id; illegal with `@a`/`@p`; allowed with `@r` | `type` (illegal on `@r`) |
| `m` | gamemode; `s/c/a/d` and `0/1/2/5` | `gamemode=` full names |
| `tag` | `/tag` | `tag` |
| `name` | name | `name` |
| `l` `lm` | max/min XP level | `level` range |
| `rx` `rxm` | max/min pitch | `x_rotation` |
| `ry` `rym` | max/min yaw | `y_rotation` |
| `scores` | `{obj=range}`; **`!` allowed** | `scores` without `!` |
| `family` | `minecraft:type_family` | **none** |
| `hasitem` | item test object / array of objects | Java `items` / `/execute if items` |
| `haspermission` | `{camera=enabled,…}` | none (use `/inputpermission`) |
| `has_property` | entity properties | none |

No `team`, `nbt`, `predicate`, `advancements`, `distance`, `limit`, `sort`.

## `hasitem` object

Required `item`. Optional:

- `data` — aux (defaults 0; potions need explicit data)
- `quantity` — int range; default `1..`; `0` means “does not have”; `!` inverts
- `location` + `slot` — slot family + index range (`slot.weapon.mainhand`, …)

```
@s[hasitem={item=stick,location=slot.weapon.mainhand}]
@a[hasitem=[{item=apple,quantity=1..},{item=bow}]]
```

## Examples that must parse on Bedrock and fail on Java

```
@r[type=minecraft:cow]
@a[m=c]
@e[r=10,rm=3]
@a[c=-1]
@e[family=monster,family=!undead]
@a[hasitem={item=diamond,quantity=1..}]
@initiator
```
