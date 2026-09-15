# Java item and block components

Since 1.20.5, items in commands use **component patches**, not `{tag:…}` NBT on the item id.

## Item stack (`minecraft:item_stack`) — `/give`, `/item replace`

```
item_stack = item_id , [ "[" , patch , { "," , patch } , "]" ] ;
patch      = component_id , [ "=" , value ]
           | "!" , component_id ;          (* remove / default *)
```

Examples:

```
diamond_sword[minecraft:enchantments={levels:{"minecraft:sharpness":5}}]
potion[potion_contents={potion:"minecraft:night_vision"}]
diamond_pickaxe[max_damage=10,!unbreakable]
```

Namespaces on well-known components may be omitted (`enchantments=`).

Count is a **separate** `brigadier:integer` argument on `/give`, not inside the stack parser.

## Item predicate (`minecraft:item_predicate`)

Used by `/clear`, `/execute if items`, loot conditions.

```
item_predicate = item_id | "#" , tag , [ "[" , predicates , "]" ]
```

Component predicates can test presence, ranges, and nested objects. This is stricter than a stack (it does not always require a full item).

## Block state + nbt

```
block_state     = block_id , [ "[" , state_eq , { "," , state_eq } , "]" ]
block_predicate = id_or_tag , [ "[" , states , "]" ] , [ nbt_compound ]
state_eq        = name , "=" , value
```

Example: `chest[facing=north,type=single]`  
`minecraft:block_predicate` also accepts `#minecraft:logs[axis=y]`.

## Slot names (`minecraft:item_slot` / `slot_source`)

| Pattern | Meaning |
| --- | --- |
| `weapon.mainhand` `weapon.offhand` | Hands |
| `armor.head` `armor.chest` `armor.legs` `armor.feet` | Armor |
| `weapon.*` `armor.*` `container.*` `hotbar.*` `enderchest.*` | Ranges (`slot_source`) |
| `container.0` … | Chest / player inventory |
| `enderchest.0` | Ender chest |
| `villager.*` `horse.*` | Entity inventories |
| `contents` | Block entity contents (furnace, chest) |

`if slots` tests that the slot source exists; `if items` also tests an item predicate.
