# Java parser implementation notes

## Order of work

1. Brigadier walker over `data/brigadier-tree.json` (already complete for **command shapes**).
2. Custom parsers for `minecraft:entity`, `minecraft:nbt_*`, `minecraft:component`, `minecraft:item_stack`, `minecraft:block_predicate`, `minecraft:particle`, `minecraft:message`.
3. `/execute` as a recursive descent that **redirects** to itself.
4. `.mcfunction` reader: comments, macros, then one command per line.

## Tests that must exist (not optional)

- Every generated usage for `execute`, `data`, `item`, `loot`, `scoreboard`, `function`, `return` parses.
- Mixing `^` and `~` in one vec3 fails.
- `@r[type=cow]` is a **syntax/semantic error** on Java.
- `execute as @s` without `run`/condition is unparseable.
- `execute if function ns:f` without a following instruction is unparseable.
- `give @p diamond_sword{Enchantments:[]}` is **not** valid current syntax (components, not `{tag}`).
- `tp` and `teleport` produce the same opcode.

## Particles

`minecraft:particle` is a mini-language: `dust r g b scale`, `dust_color_transition …`, `block <block_state>`, `item <item_stack>`, `entity_effect`, `shriek`, `vibration`, `sculk_charge`, etc. Implement by registry of particle extra parsers, not one regex.

## Gamerules, attributes, advancements, dialog, waypoint, compute, test

These roots exist in the tree. Do not stub them as “unknown command”. Use `generated-usages.md` as the checklist. Semantics can lag; **parse shapes cannot**.
