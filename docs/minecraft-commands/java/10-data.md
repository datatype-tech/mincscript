# Java `/data`

Root literals from the tree: `get`, `merge`, `modify`, `remove`.

## Targets

| Kind | Address |
| --- | --- |
| `block <pos>` | Block entity |
| `entity <single entity>` | Entity NBT |
| `storage <resource>` | Command storage (`minecraft:storage`) |

## Forms

```
data get (block|entity|storage) <src> [<path>] [<scale>]
data merge (block|entity|storage) <src> <compound>
data remove (block|entity|storage) <src> <path>
data modify (block|entity|storage) <src> <path> <mode> <source>
```

### `modify` modes (all present in the tree)

`append` | `insert <index>` | `merge` | `prepend` | `set`

Each mode then takes one of:

| Source | Syntax |
| --- | --- |
| `from` | `from (block|entity|storage) <src> [<path>]` |
| `value` | `value <nbt_tag>` |
| `string` | `string (block|entity|storage) <src> <path> [<start>] [<end>]` |
| `compute` | `compute (block <pos>|entity <entity>|default) (float|integer) <provider>` |

`compute` is current-tree (26.x) numeric extraction. Implement it; do not treat `/data` as “merge only”.

## `/data get` scale

Optional double multiplies the numeric result (command `result`).

## Storage ids

`storage ns:path` is a resource location, not a selector. Created on first merge.

Full expansions: generated usages under `data`.
