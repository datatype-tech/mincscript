# Bedrock coordinates

Command-argument positions (`x y z` in official signatures):

- Absolute numbers
- `~` relative to execution origin (changed by `/execute positioned|at`)
- `^` local (changed by `/execute anchored|rotated|facing`)

Selector origins:

- `~` allowed
- `^` **not** allowed
- Integer `x=0` becomes `0.5` unless `x=0.0`

`/tp` has extra `checkForBlocks: Boolean` and `facing` overloads that Java expresses differently. Official:

```
/tp <victim: target> <destination: target> [checkForBlocks: Boolean]
/tp <victim: target> <destination: x y z> [checkForBlocks: Boolean]
/tp <victim: target> <destination: x y z> [yRot] [xRot] [checkForBlocks]
/tp <victim: target> <destination: x y z> facing <lookAtEntity: target> [checkForBlocks]
/tp <victim: target> <destination: x y z> facing <lookAtPosition: x y z> [checkForBlocks]
```

`/tp @s` **does not work in command blocks** (official note): the block is not an entity. Use `positioned`/`as` from a player or an entity executor.
