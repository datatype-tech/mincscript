# Items, NBT, text

| Topic | Java | Bedrock |
| --- | --- | --- |
| `/give` extra | `id[components]` + count | `amount` `data` `components` JSON |
| Enchant in give | component `enchantments` | `/enchant` or aux for potions |
| Entity NBT write | `/data` | mostly impossible; `event`, `damage`, `inputpermission` |
| Block entity NBT | `/data` | limited (`structure`, clone) |
| Command storage | `/data storage` | **none** |
| Text | JSON / SNBT component | `rawtext` |
| Titles | `/title` JSON via component | `/title` plain + `/titleraw` JSON |
| Loot | `/loot` Java graph | `/loot` BE graph |
| Inventory slot API | `/item` | `/replaceitem` |

## Block states

Java: `chest[facing=north]`  
Bedrock: official `blockStates: block properties` — treat as a different parser.

## Identifiers

Both accept `minecraft:` prefix. Bedrock still accepts some **legacy numeric ids** in old worlds; do not emit numbers from MincScript.
