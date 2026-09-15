# Java `/function`, macros, `/return`, `/schedule`

## `/function`

```
function <name:minecraft:function>
function <name> <arguments:minecraft:nbt_compound_tag>
function <name> with (block <pos> | entity <entity> | storage <id>) [<path>]
```

`<name>` may be a function id or `#namespace:tag`.

### Macros

A `.mcfunction` line containing `$(ident)` is expanded **before** parse:

```
$(variable)
$(variable|default text)
```

The compound from `function … {k:1}` or `with storage …` supplies keys. Missing keys without default → that line is skipped (function may still run other lines).

Macros cannot appear in chat.

## `/return`

```
return fail
return value <int>
return run <command>
```

`return run` sets the function’s result to the inner command’s result and **stops** the function. `return fail` fails the function. Used with `execute store` and `execute if function`.

## `/schedule`

```
schedule function <name> <time> [append|replace]
schedule clear <name>
```

Time is `minecraft:time` (`1s`, `20t`, `1d`). `replace` (default) resets the wait; `append` queues another.

## Function execution vs execute forking

`execute as @e run function ns:f` runs `f` **once per entity**, depth-first relative to that function’s contents. That is how Java emulates Bedrock’s depth-first execute.

Command `success`/`result` after `run function` depend on `/return` and whether the function forked internally.
