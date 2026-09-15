# Project config and CLI

Every MincScript project is a directory with **`minc.toml`** at the root. That file is the only place that names the target **edition** and **game version**. Source files do not contain `if bedrock`. If a construct cannot lower to that pair, the compiler errors at the use site.

## `minc.toml`

```toml
[project]
name = "metro-escape"
pack = "metro.escape"          # Java/Bedrock namespace stem
score_revision = 1            # 删字段后加一，避免记分板 id 复用

[target]
edition = "bedrock"            # "java" | "bedrock"
game_version = "1.21.70"       # 目标游戏版本，点分号

[emit]
mincb = true                   # 统一二进制，默认 true
functions = true               # 同时吐出 .mcfunction，方便对照
datapack_or_behavior_pack = true

[world]
dimension = "overworld"
origin = [0, 64, 0]            # 总控默认原点；链坐标相对此点，除非写绝对坐标

[chains]
default_layout = "stack"       # linear | stack | snake | box | points
default_facing = "up"          # east west south north up down
max_span = 32                  # 单行/单列最多多少个方块后折返（snake/box）
clock = "Play"                 # 哪一条链是每刻总钟；也可在 controller.mcs 声明

[command_block]
default_type = "chain"         # impulse | chain | repeat
default_redstone = "always_active"
default_conditional = false
track_output = false

[limits]
function_command_limit = 10000 # 与基岩 functioncommandlimit / Java 实际深度对齐
```

### Edition + version (what they do)

| Field | Effect |
| --- | --- |
| `edition = "bedrock"` | Selector `r=` / `hasitem` / no `store` / no `/team` / `rawtext` / `replaceitem`. `min_engine_version` derived from `game_version`. |
| `edition = "java"` | `distance=` / item components `[]` / `/data` / `/item` / `execute store` allowed when used. |
| `game_version` | Picks a **command ISA snapshot**. Example: Bedrock `1.19.50` is the floor for new `/execute`; Java `1.20.5` is the floor for item components. Unknown version → error, do not guess. |

The compiler loads the checked-in grammars under `docs/minecraft-commands/` (or a future frozen JSON keyed by version). It must not emit a command that is missing from that snapshot.

`pack` is the namespace: Java `metro.escape:play`, Bedrock function path `metro/escape/play`.

## CLI (`minc`)

The shipped binary is **`minc`** (Rust). Subcommands; no hidden flags in source.

```
minc new <dir> --edition bedrock --version 1.21.70
minc new <dir> --edition java --version 1.21.11

minc check                 # 类型检查 + 版本/版本特性检查，不写文件
minc build                 # 写出 dist/<name>.mincb 以及可选函数包
minc build --out dist/
minc build --emit functions-only   # 调试：只看命令，不排命令方块

minc inspect dist/metro-escape.mincb
minc inspect --chain Play

minc layout Play --layout stack --origin 2 70 0 --facing up
minc layout Lobby --layout linear --origin 0 70 0 --facing east

minc dump dist/metro-escape.mincb --commands
```

`minc new` writes:

- `minc.toml`
- `src/controller.mcs`
- `src/chains/Main.chain.mcs` (empty Repeat clock)
- `src/logic/World.mcs`
- `.gitignore` for `dist/`

`minc layout` **does not compile**. It rewrites the chain’s placement attributes in `controller.mcs` (or a generated `src/controller.layout.mcs` include) so coordinates stay in source, not in a side cache.

### Create-project defaults

| `--edition` | `tick` host | Default clock chain |
| --- | --- | --- |
| bedrock | `functions/tick.json` **and** optional Repeat CB | functions preferred; CB still emitted if any `.chain.mcs` exists |
| java | datapack `function` tick tag | same policy |

## Coordinates in the CLI vs in source

- CLI `--origin x y z` is **world absolute** unless `--relative` is passed.
- In `controller.mcs`, `at (2, 6, 0)` is **relative to `world.origin`** by default.
- `at absolute (100, 64, -30)` is world space.
- `minc layout` updates those numbers; `minc build` is the only command that writes MINCB.

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | ok |
| 1 | source / type / edition error (diagnostics on stderr, Ariadne) |
| 2 | IO / missing `minc.toml` |
| 3 | inspect of a MINCB targeting a different edition than `minc.toml` (warn + fail if `--strict`) |
