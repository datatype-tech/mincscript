# Bedrock JSON text (`rawtext`)

Official `/tellraw` / `/titleraw` examples:

```
/tellraw @a {"rawtext":[{"text":"Hello World"}]}
/tellraw @a {"rawtext":[{"translate":"commands.testfor.success","with":["PlayerName"]}]}
```

## `rawtext` array entries

| Type | Shape |
| --- | --- |
| text | `{"text":"…"}` |
| translate | `{"translate":"key","with":[…]}` |
| selector | `{"selector":"@a"}` |
| score | `{"score":{"name":"@s","objective":"obj"}}` |

There is **no** Java `click_event` / hover / fonts feature-set in Bedrock `rawtext`. `/title` is **plain text**; JSON titles are `/titleraw`.

`/say`, `/tell`, `/me` take `message` (plain, selectors expanded), not JSON.
