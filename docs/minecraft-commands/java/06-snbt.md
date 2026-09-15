# Java SNBT (named binary tag as text)

Java commands embed NBT as **SNBT**, not Bedrock JSON. Parsers: `minecraft:nbt_compound_tag`, `minecraft:nbt_tag`. Selectors use SNBT in `nbt={…}`.

## Values

```
nbt        = compound | list | array | string | number | bool ;
compound   = "{" , [ pair , { "," , pair } ] , "}" ;
pair       = key , ":" , nbt ;
key        = unquoted | quoted ;
list       = "[" , [ nbt , { "," , nbt } ] , "]" ;
array      = "[" , ( "B" | "I" | "L" ) , ";" , [ number , { "," , number } ] , "]" ;
string     = quoted | unquoted ;
bool       = "true" | "false" ;     (* stored as bytes 1 / 0 *)
```

## Numeric suffixes (case-sensitive)

| Suffix | Type |
| --- | --- |
| none / `d` | double |
| `f` | float |
| `b` | byte |
| `s` | short |
| `l` | long |
| (integer, no suffix) | int |

Examples: `{HurtTime:10s,Health:20.0f,OnGround:1b,UUID:[I;1,2,3,4]}`.

## Strings

Quoted with `"` and `\'`/`\\`/`\"` escapes. Unquoted strings are identifiers (no spaces).

## Heterogeneous lists

SNBT lists are typed in binary NBT. A list’s elements must share a type. Empty list `[]` is allowed. Nested compounds in `{Item:{id:"minecraft:stone",count:1}}` are compounds, not mixed lists.

## NBT paths (`minecraft:nbt_path`)

Used by `/data` and `execute store` / `if data`.

```
path       = segment , { "." , segment | index | filter } ;
segment    = quoted | unquoted | compound_filter ;
index      = "[" , integer , "]" ;
filter     = "[" , compound , "]" ;     (* match list element *)
```

Examples:

```
Items[0].id
Items[{Slot:0b}].count
Inventory[-1]
{a:1}.b
```

Root `{…}` as a path segment matches the whole tag against a compound.

## Testing vs merging

- `execute if data entity @s Inventory[{id:"minecraft:diamond"}]` — existence
- `data modify … set value {a:1}` — write
- Selector `nbt=` is a **subset match** (specified keys must match; extra keys on the entity are ok)

## Not Bedrock

Bedrock `/give` components are JSON (`{"minecraft:can_place_on":{…}}`). Java `/give` uses item **components** in `[]` after the id, which are **not** the same grammar as SNBT on the command token (they are component patches). SNBT still appears in `/data` and `nbt=` selectors.
