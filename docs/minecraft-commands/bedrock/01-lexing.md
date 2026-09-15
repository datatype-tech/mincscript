# Bedrock Edition lexing

## Line structure (official)

1. `/`
2. command name
3. arguments separated by spaces

Quoted strings: `"text with spaces"`. Unquoted tokens cannot contain spaces.

## Types as they appear in official signatures

| Written type | Meaning |
| --- | --- |
| `target` / `targets` | Player name or selector |
| `x y z` / `position` | Three coordinates (`~` allowed; `^` allowed in command args, **not** in selector `x,y,z` except tilde) |
| `int` | 32-bit integer |
| `float` | Float |
| `string` | Word or quoted |
| `json` | JSON object |
| `message` | Greedy text; selectors expanded to names |
| `text` | String |
| `Block` / `Item` | Enum of namespaced ids |
| `block properties` | Block-state list |
| `codebuilderargs` | Nested **command** (`/execute run`) |
| `executechainedoption_0` | Another execute subcommand |
| `wildcard int` | int or `*` |
| `fullintegerrange` | Integer range for `matches` |
| `compareoperator` | Score compare op |
| `filepath` | `/path/to/file` |
| `time` (weather duration) | ticks; TimeSpec enums elsewhere |

## Coordinates

Same three writings as Java (`absolute`, `~`, `^`) in **command** position arguments.

**Selector `x y z`:** official + wiki: **tilde allowed, caret not**. Integer positions are **center-corrected** to `+0.5` unless written as `0.0`.

## JSON

Bedrock uses JSON for:

- `/tellraw` `/titleraw` — `{ "rawtext": [ … ] }`
- `/give` optional `components`
- `/dialogue`
- some camera/hud payloads

This is **not** Java SNBT. Unquoted Java `{Health:20f}` is invalid here.

## Comments

Behavior-pack functions: `#` comments. No Java `$(macro)` substitution.

## Block states

Often `[ "direction": 0 ]` JSON-ish or `["facing"="north"]` depending on command. Official clone/fill use `blockStates: block properties`. Implement a parser that accepts:

```
["facing"="north","open"=true]
```

and the JSON object form used by item components.
