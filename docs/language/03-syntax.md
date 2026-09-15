# Syntax

MincScript looks like a small Java 11: braces, `class`, `public`/`private`, `if`/`else`, `foreach`, annotations. It is **not** a JVM language. There is no `new` for entities, no `null`, no runtime generics (except the built-in `Seq<T>` selector type and compile-time `List<T>`), no exceptions.

File encoding: UTF-8. Comments: `//` and `/* */`. Identifiers: Unicode letters, `$` not used.

## Compilation units

| Suffix | Role |
| --- | --- |
| `*.mcs` | pack, classes, enums, static setup |
| `*.chain.mcs` | **one** command-block chain (see [04](04-chains-and-world.md)) |
| `controller.mcs` | exactly one per project, filename reserved |
| `*.place.mcs` | `place` / `fill` / `chest` only (may also live inside controller) |

Header of every file:

```java
pack metro.escape;
import metro.escape.Runner;
```

`pack` must match `minc.toml` `project.pack` or be a subpackage `metro.escape.world`.

## Types (v1)

| Type | Meaning |
| --- | --- |
| `void` | method returns nothing |
| `int` | score |
| `boolean` | tag or static 0/1 |
| `Player` | player selector handle |
| `Entity` | any entity handle |
| `BlockPos` | compile-time or runtime `~ ~ ~` / absolute |
| `Block` | block id + optional states |
| `Item` | item id + optional count / data / components |
| `Region` | two `BlockPos` or AABB / circle |
| `Seq<T>` | selector, not a list |
| `List<T>` | **compile-time** list (`List.of`); `foreach` unrolls. Not a heap array |
| `temp T x` | one-shot local: inlined or `#tN mt` scratch then `reset`. Never a user objective |
| `String` | **compile-time only** (titles, command fragments) |
| enum types | `int` ordinal |

`String` concatenation is compile-time: `"撤离" + "成功"` is fine; `p.name + "x"` is not (no runtime strings).

## Methods

```java
public void tryExtract(Region zone) { ... }
public static void tick() { ... }
boolean canExtract();   // package-private not in v1 — use public/private
```

Overloading: by arity only (not by type), to keep lowering simple.

`return` in a function-hosted method:

- Bedrock: no `/return` — the compiler uses `if` so later lines are skipped, or splits the method. **No** Java `/return` on Bedrock.
- Java: `/return` / `return run` allowed when `edition = java`.

## Control flow → `/execute`

```java
if (Match.phase == Phase.PLAY) {
    Runner.of(Player.self()).tickPlay();
} else if (Match.phase == Phase.EXTRACT) {
    Extract.tick();
} else {
    Lobby.tick();
}
```

Lowers to mutually exclusive `execute if score … matches N run function …`.

```java
if (block(BlockPos.at(0, 63, 0)) == Blocks.GOLD_BLOCK) {
    Match.start();
}
```

→ `execute if block 0 63 0 gold_block run function …`

AND is nested `if` or a single selector. OR uses the Bedrock `unless` trick / Java extra `if` lines — the compiler knows both (see command gameplay docs).

```java
foreach (Player p : Players.all().withTag("in_round")) {
    as (p) at (p) {
        Runner.of(p).tickHold();
    }
}
```

→ `execute as <sel> at @s run function …`

`as (expr) { }` / `at (expr)` / `facing (pos)` / `anchored (eyes|feet)` / `align (xyz)` are first-class. They **must** wrap a block. They do not exist as Java methods on `Player` to avoid mixing context with data.

## Selectors (fluent, then frozen)

```java
Players.all()
    .inBox(64, 64, 0, 3, 3, 3)
    .withTag("metro_key")
    .withoutTag("extracted")
    .hasItem(Items.GOLD_INGOT, 1)
```

The builder is compile-time only. It emits **one** selector string for the target edition:

- Bedrock: `@a[x=64,y=64,z=0,dx=3,dy=3,dz=3,tag=t0a,tag=!t0b,hasitem={item=gold_ingot,quantity=1..}]`
- Java: `@a[x=64,y=64,z=0,dx=3,dy=3,dz=3,tag=t0a,tag=!t0b]` plus `execute if items` if needed

Raw escape when the fluent API lags:

```java
Players.raw("@a[hasitem={item=gold_ingot,quantity=1..}]")  // checked against edition
```

Java `distance=` vs Bedrock `r=` is `.within(origin, radius)` — the compiler picks the key.

## Literals and world builtins

```java
BlockPos origin = BlockPos.of(0, 64, 0);
BlockPos here = BlockPos.here();          // ~
BlockPos feet = BlockPos.here().up(-1); // ~ ~-1 ~

Items.GOLD_INGOT.count(1);
Items.GOLD_INGOT.data(0);                 // Bedrock aux; error on Java
Items.DIAMOND_SWORD.component("item_lock", "{mode:lock_in_inventory}"); // edition-checked

cmd("title @a title 地铁逃生");           // 整行原样，仍禁止前导以外的语法裂口
run "say hello";                       // 一条语句 = 一条指令
/say hi;
give(Players.all(), Items.GOLD_INGOT.count(1));
title(Players.all(), Title.TITLE, "地铁逃生");
tellraw(Players.all(), Text.raw("§e撤离点已开启"));
```

`title` / `tellraw` / `give` / `kill` / `effect` / `clear` / `playsound` / `particle` / `summon` / `setblock` / `xp` / `enchant` / `replaceItem` are builtins. The compiler prints **edition-correct** lines (Java `effect give` vs Bedrock `effect`, Java `item replace` vs Bedrock `replaceitem`, Java text components vs Bedrock `rawtext`). See [06 Tutorial](06-tutorial.md).

## Annotations (logic)

```java
@OnLoad
@OnTick                    // 进 tick.json / datapack tick，而不是命令方块
@Host(HostKind.FUNCTION)   // 默认：普通 .mcs 方法
@Host(HostKind.COMMAND_BLOCK)
@JavaOnly
@BedrockOnly
```

`@JavaOnly` on a method: the method is skipped when targeting Bedrock (must not be called from shared code). Cross-edition projects use `minc.toml` edition **one** value per build; two editions = two builds.

## What looks like Java but is illegal

| Illegal | Why |
| --- | --- |
| `new Player()` | players are not allocated |
| `null` | missing selector is `exists() == false` |
| `try/catch` | no exceptions |
| `int[]` | no runtime arrays; compile-time `List.of` + `foreach` unroll, or scores / chests |
| `List` as a heap type | `List<T>` exists only as `List.of(...)` unrolled at compile time |
| `interface` / `abstract` | v2 |
| `synchronized` | no |
| `switch` on `String` | strings are compile-time; `switch` on `enum`/`int` is allowed |

```java
switch (Match.phase) {
    case Phase.LOBBY -> Lobby.tick();
    case Phase.PLAY -> Play.tick();
    default -> { }
}
```

## Expression lowering cheat sheet

| Source | Typical Bedrock | Typical Java |
| --- | --- | --- |
| `a + b` on scores | `operation t = a` then `+= b` | same |
| `p.keys >= 1` | `execute if score @s m03 matches 1..` | same |
| `p.extracted` | `entity @s[tag=t0c]` | same |
| `block(pos) == X` | `if block` | same |
| `p.give(item)` | `give` | `give` / `item` |
| `p.teleport(pos)` | `teleport` | `teleport` |
| `p.gamemode(ADVENTURE)` | `gamemode adventure` | `gamemode adventure` |

Temps for complex **runtime** score expressions use reserved fake players `#t0`, `#t1` in a compiler objective `mt`, then `scoreboard players reset #tN mt`. They are not user-visible types and are not entered in the class objective table. User `temp` locals never become dummy objectives. Compile-time `List<T>` is unrolled, not stored.

A full walkthrough with builtins, temps, lists, and the Java/Bedrock command split is [06 Tutorial](06-tutorial.md).
