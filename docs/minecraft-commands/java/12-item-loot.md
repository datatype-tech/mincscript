# Java `/item`, `/loot`, `/place`, `/clone`, `/fill`

These are the world/inventory mutators. All forms are in `generated-usages.md`; this page states the **composition rules**.

## `/item`

```
item replace (block <pos>|entity <entities>) <slot> with <item_stack> [<count>]
item replace … from (block|entity) <src> <slot> [<modifier>]
item modify (block|entity) … <slot> <modifier>
item fill …          (* 26.x tree *)
item override …      (* 26.x tree *)
```

`minecraft:loot_modifier` is a datapack item-modifier id.

## `/loot`

```
loot (give <players> | spawn <vec3> | insert <pos> | replace (block|entity) …)
     (fish <table> <pos> <tool> | loot <table> | kill <entity> | mine <pos> [<tool>])
```

The first group is the **destination**; the second is the **source**. Both are required.

## `/place`

Places features / jigsaw templates / structures / POI:

```
place feature <feature> [<pos>]
place jigsaw <pool> <target> <max_depth> [<pos>]
place structure <structure> [<pos>]
place template <template> [<pos>] [<rot>] [<mirror>] [<integrity>] [<seed>]
```

Exact children: generated usages.

## `/clone`

Java (tree):

```
clone [from <dimension>] <begin> <end> [to <dimension>] <destination>
      [strict] (replace|masked|filtered <block_predicate>) (force|move|normal)
```

`from`/`to` dimensions are current-tree. `filtered` takes a `minecraft:block_predicate`.

## `/fill` / `/fillbiome`

```
fill <from> <to> <block> [replace|destroy|keep|hollow|outline|strict]
fill … replace <filter>
fillbiome <from> <to> <biome> [replace <filter biome>]
```

## `/setblock`

```
setblock <pos> <block> [destroy|keep|replace|strict]
```
