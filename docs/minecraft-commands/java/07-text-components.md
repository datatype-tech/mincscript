# Java text components

Parser: `minecraft:component` (`/tellraw`, `/title`, `/bossbar set name`, scoreboard display names). `minecraft:style` is a style-only object (`scoreboard players display numberformat … styled`).

`minecraft:message` (`/say`, `/me`, `/msg`) is **not** JSON: greedy plain text in which selectors are expanded to display names.

## Values that must parse as a component

1. A JSON string: `"hello"`
2. A JSON array: `["a",{"text":"b","color":"red"}]`
3. A JSON object: `{"text":"hi","bold":true}`
4. SNBT-like object with the same keys (chat often accepts both)

## Object keys (non-exhaustive, required for a complete emitter)

| Kind | Keys |
| --- | --- |
| text | `text` |
| translate | `translate`, `with`, `fallback` |
| score | `score:{name,objective,value?}` |
| selector | `selector`, `separator` |
| keybind | `keybind` |
| nbt | `nbt`, `block`/`entity`/`storage`, `interpret`, `separator` |
| object | `object` (atlas / player head — current versions) |

Shared style: `color`, `bold`, `italic`, `underlined`, `strikethrough`, `obfuscated`, `font`, `shadow_color`, `click_event`, `hover_event`, `extra`.

1.21.5+ renamed `clickEvent`/`hoverEvent` to `click_event`/`hover_event` with structured payloads. The parser should accept both while emitting the current form.

## `/tellraw` / `/title`

```
tellraw <targets> <component>
title <targets> (clear|reset|title|subtitle|actionbar) …
title <targets> times <fadeIn> <stay> <fadeOut>
```

See generated usages for the exact tree.
