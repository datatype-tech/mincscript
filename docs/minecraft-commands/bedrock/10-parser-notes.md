# Bedrock parser implementation notes

## Order of work

1. Load [`data/command-index.json`](data/command-index.json) and register every `official_usages` line as an overload.
2. Implement selector parameters from [`03-selectors.md`](03-selectors.md) (this is where most Java ports break).
3. Implement `/execute` as a **recursive** chain (`executechainedoption_0`), depth-first semantics at **runtime**, not only parse time.
4. JSON `rawtext` + give `components`.
5. Remaining roots from the 82-command list; reject anything else (including Java-only verbs).

## Tests that must exist

- All 82 official usage lines parse.
- `execute store result …` fails.
- `@r[type=cow]` parses.
- `@a[distance=..5]` fails (`distance` is Java).
- `give @p diamond_sword[enchantments={}]` fails.
- `tellraw @a {"rawtext":[{"text":"x"}]}` parses; Java component without `rawtext` should fail unless you explicitly accept a compatibility mode.
- `allowlist` parses; `whitelist` does not (Java).
- `xp` is the Bedrock root (Java’s primary root is `experience` with alias `xp`).

## `/help`

Official:

```
/help [command: CommandName]
/help <page: int>
```

Useful at runtime for discovering extra overloads; the **checked-in** spec is still the generator markdown.
