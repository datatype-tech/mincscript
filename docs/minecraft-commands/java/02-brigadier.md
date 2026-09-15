# Brigadier mapping for Java commands

Java Edition does not parse commands with a handwritten PEG for each verb. It registers a `CommandDispatcher<ServerCommandSource>`. The dump in [`data/brigadier-tree.json`](data/brigadier-tree.json) is that graph after `java -DbundlerMainClass=net.minecraft.data.Main -jar server.jar --reports`.

## Node fields

| Field | Meaning |
| --- | --- |
| `type` | `literal` or `argument` (root is a literal named by the map key) |
| `parser` | Argument type id (`brigadier:*` or `minecraft:*`) |
| `properties` | Parser extras (`min`/`max`, `type`/`amount` on entities, string mode) |
| `children` | Next tokens |
| `executable` | This prefix may end the command |
| `redirect` | Continue as if the named node’s children were here |
| `permissions` | Command source predicate |

## Redirects (critical for `/execute`)

After a modifier such as `as <targets>`, the node **redirects to `execute`**. That means another full execute-instruction is required (or a terminal condition / `run`).

Implementation:

```
if node.redirect == ["execute"]:
    parse_execute_instruction_again()
```

Do **not** inline a copy of the execute tree; you will desync when Mojang adds `if slots`.

## Executable conditions

`execute if entity @s` is a complete command (success count = number of matching entities). The same node also redirects, so `execute if entity @s run say ok` is valid. The generated usages list **both** forms when the tree has `executable` and `redirect`.

`execute if function` only redirects (a following instruction is required).

## Aliases

Aliases are extra root literals with the same child graph:

| Alias | Target |
| --- | --- |
| `tp` | `teleport` |
| `w`, `tell` | `msg` |
| `tm` | `teammsg` |
| `xp` | `experience` |

Parse them as separate roots that produce the same AST opcode.

## Suggested AST

```text
Command
  name: string
  nodes: [Literal | Argument]
  executable: bool

Argument
  name: string
  parser: string
  properties: object
  raw: string
  value: typed
```

Keep `raw` for round-tripping SNBT/JSON.

## Walking to usage strings

`tools/extract_minecraft_commands.py` DFS-walks the tree. Any new vanilla command is automatically listed in `generated-usages.md` after re-extraction. Handwritten pages in this folder document **parsers and chaining rules** the JSON cannot explain (selector grammar inside `minecraft:entity`, SNBT, execute forking).
