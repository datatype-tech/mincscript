# Java Edition lexing

A command is a sequence of tokens. Brigadier does **not** tokenize the whole line first; each argument parser consumes from the remaining string. The lexer you write for MincScript should still expose these atoms so the two-stage pipeline (MincScript → command graph) is testable.

## Characters

```
command      = [ "/" ] , root_literal , { " " , node } ;
whitespace   = " " | "\t" ;          (* chat/functions: single ASCII space is the separator *)
comment      = "#" , { any - "\n" } ; (* .mcfunction only, full line *)
```

Inside SNBT, JSON, and quoted strings, spaces are **not** command separators.

## Identifiers and resource locations

```
unquoted     = ( letter | "_" ) , { letter | digit | "_" | "." | "-" } ;
resource     = [ namespace , ":" ] , path ;
namespace    = { letter | digit | "_" | "-" | "." }- ;
path         = { letter | digit | "_" | "-" | "/" | "." }- ;
```

Unnamespaced ids in vanilla default to `minecraft:`. Tags use `#namespace:path` where a parser is `resource_or_tag` / `resource_or_tag_key`.

## Quoted strings (`brigadier:string` with `type=phrase` or greedy)

```
quoted       = '"' , { string_char } , '"' ;
string_char  = escape | ( any - '"' - "\\" - "\n" ) ;
escape        = "\\" , ( "\\" | '"' ) ;
```

`type=word` is an unquoted token. `type=greedy` consumes the rest of the line (used by `minecraft:message` after expanding selectors).

## Numbers

```
integer      = [ "-" ] , digits ;
double       = [ "-" ] , ( digits , [ "." , digits ] | "." , digits ) ;
range        = [ number ] , ".." , [ number ] | number ;
```

`minecraft:int_range` / `minecraft:float_range` accept `N`, `N..`, `..N`, `N..M`. Java selector `distance`/`level`/`x_rotation` use this, **not** Bedrock’s paired min/max keys.

## Selectors (entry)

```
selector     = "@" , variable , [ "[" , selector_args , "]" ] ;
variable     = "p" | "r" | "a" | "e" | "s" | "n" ;
```

No space between `@e` and `[`. Arguments: see [`04-selectors.md`](04-selectors.md).

## Coordinates

A `minecraft:vec3` / `minecraft:block_pos` is three coordinates. Each is:

```
coord        = absolute | relative | local ;
absolute     = double ;
relative     = "~" , [ double ] ;
local        = "^" , [ double ] ;
```

**Constraint (Java):** a single vec is either all-world (`absolute`/`relative` mix allowed) **or** all-local (`^`). Mixing `^` with `~` or a bare number in the same vec is a parse error.

`minecraft:rotation` is yaw then pitch; `~` is allowed; `^` is not.

## SNBT and JSON

`minecraft:nbt_compound_tag` starts at `{`. `minecraft:nbt_tag` may start at `{`, `[`, `"`, a number, or an unquoted token (`true`, `false`, `trueb` is invalid — see [`06-snbt.md`](06-snbt.md)).

`minecraft:component` (text) is JSON or a SNBT-like compound depending on version; current tree still names it `minecraft:component`. Quoted JSON and unquoted string shortcuts both appear in vanilla chat.

## Item / block predicates

```
item_stack   = resource , [ "[" , components , "]" ] ;
block_state  = resource , [ "[" , states , "]" ] ;
```

Component lists are comma-separated `id=value` or `id` / `!id` predicates. This replaced inline `{NBT}` on items in 1.20.5+.

## Function macros (`.mcfunction` only)

```
macro_insert = "$(" , ident , [ "|" , default_text ] , ")" ;
```

A line that contains `$(` is a macro line. It is **not** valid in chat. `/function ns:fn {k:1}` supplies the compound used for substitution.
