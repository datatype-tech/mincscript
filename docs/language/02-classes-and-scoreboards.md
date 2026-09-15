# Classes, fields, and the scoreboard rule

Minecraft has no objects. MincScript pretends it does, then **lowers fields to dummy scores and tags**. Privacy is a compiler rule: a private field is only readable or writable in methods of the same class (and in nested helpers the compiler generates for that class). Other packs, chain files, and `cmd "..."` raw strings **cannot name the mangled objective**.

## What a class is

```java
pack metro.escape;

public class Runner {
    private int keys;           // 记分板：每玩家一个整数
    private int hold;           // 撤离站立计时
    private boolean extracted;  // 标签，不是记分板
    private boolean dead;

    public void giveKey() {
        this.keys += 1;
        this.extracted = false;
    }

    public boolean canExtract() {
        return this.keys >= 1 && !this.extracted && !this.dead;
    }
}
```

| Member | Runtime | Why |
| --- | --- | --- |
| `int` instance field | dummy **score**, holder = the entity `this` | only integers persist per entity this way |
| `boolean` instance field | **tag** `pack.Class.field` (then shortened) | facts; matches how maps already use tags |
| `static int` | dummy score, holder = fake player `#Class` | world registers (`Phase`, timers) |
| `static boolean` | tag on a marker / fake player **or** 0/1 score on `#Class` | v1: 0/1 score, simpler on both editions |
| `enum` | `static int` on a world object | see below |
| methods | function fragment or CB commands | no extra state |

`this` in a method that runs `as` a player is `@s`. Static methods have no `this`; they may only touch `static` fields and globals.

### World object

```java
public static class Match {
    public static int phase;      // 0 lobby … 4 end
    public static int countdown;
}
```

There is always an implicit world holder. You do not spawn it. The compiler uses a reserved fake player `#mcs` (Java) / `mcs` (Bedrock, no `#` required but `#` is allowed in names). Display names that start with `#` stay off the Bedrock sidebar (official Complete-the-Monument trick).

## Access control (the whole point)

```java
// in Extract.chain.mcs — 不允许
Runner r = Player.self();
r.keys += 1;           // error: keys is private in Runner
```

```java
// 正确：链里只调公开 API
foreach (Player p : Players.inRound()) {
    Runner.of(p).tryExtract(ExtractZone.BOX);
}
```

| Modifier | Who may read/write |
| --- | --- |
| `private` | same class body only |
| `public` | any MincScript in this project |
| *(no `protected` in v1)* | — |

Raw commands:

```java
cmd("scoreboard players add @s m12 1");
```

The compiler **does not scan** the string for stolen ids. If you use `cmd`, you opt out of privacy for that line. `minc check --strict-raw` forbids `cmd` entirely.

Chain files are not inside `Runner`, so they must use `public` methods.

## Name allocation (16-character reality)

Bedrock (and still-common Java) objective names are **short**. The compiler does **not** emit `metro.escape.Runner.keys`.

At compile time it assigns:

```
metro.escape.Runner.keys  →  objective id "m03"
metro.escape.Match.phase  →  objective id "m00"
```

The mapping is stored in MINCB section `SYMB`. `minc inspect` prints it. Generated setup commands:

```
scoreboard objectives add m00 dummy
scoreboard objectives add m03 dummy
```

Tags similarly become `t0a`, `t0b`, … with the same table.

Recompilation must be **stable**: ids are assigned by sorted qualified name so a rebuilt pack does not shuffle every score. Deleted fields leave holes (never reuse an id in the same major `project` version; `[project] score_revision` in `minc.toml` bumps the scheme).

## `int` semantics

- 32-bit signed, same as Minecraft scores
- `+=` `-=` `*=` `/=` `%=` lower to `operation` or `add`/`remove`/`set`
- Division by zero: command fails for that holder (game rule); the language does not insert extra guards unless you write `if (x != 0)`
- Comparisons `== != < > <= >=` and ranges `x in 1..10` → `matches`

Bedrock-only `players random` is `random(min, max)` and **only compiles on Bedrock**. Java uses `random` datapack / `spreadplayers` is not a substitute — error on Java unless `@Host` is a function using a Java 1.20.5+ alternative the snapshot lists.

## `boolean` vs `int` 0/1

Do not use `int` for flags. `boolean extracted` becomes a tag so selectors stay `@a[tag=t0c]`. Using `int` would force `scores={m07=1}` and waste an objective.

## Enums

```java
public enum Phase {
    LOBBY, COUNTDOWN, PLAY, EXTRACT, END
}

// Match.phase is static int; assignments:
Match.phase = Phase.PLAY;          // set score to 2
if (Match.phase == Phase.EXTRACT) { ... }
```

Ordinals are 0-based, frozen in source order. Reordering enum constants is a **breaking** change (call out in `score_revision`).

## `Player` / `Entity` handles

```java
public final class Player {
    public static Player self();                 // @s
    public static Player nearest();             // @p
    public static Seq<Player> all();            // @a
    public static Seq<Entity> entities();      // @e
}
```

`Seq<T>` is not a heap list. It is a **selector expression** (a builtin, not user-written generics). `foreach` is the only iteration. `.count()` on Bedrock cannot be stored without `execute store` (Java) — on Bedrock, “is there at least one” is `if (Players.all().withTag("x").exists())`.

Factory:

```java
Runner.of(Player p)   // same handle, typed as Runner so field access type-checks
```

`Runner` does not wrap a different entity. It is `Player` plus the field namespace.

## Initialization

```java
@OnLoad
public static void setup() {
    // 编译器已发出 objectives add；这里只放 gamerule、tickingarea
    World.gamerule("commandblockoutput", false);
    World.gamerule("sendcommandfeedback", false);
}
```

`@OnLoad` runs once (Bedrock: `inited` score gate in `tick.json`; Java: `load` function tag). Do not create objectives in user code.

Instance ints start **unset** (Minecraft missing score). First `+=` on Bedrock via `add` creates 0 then adds — the compiler emits `add 0` before first use in that method when the field is read. Document as: reads of never-written ints are 0 after setup, or a compile warning if a path reads before write.

## What you must not do

- One shared scoreboard objective for two classes
- Java `deathCount` criteria on Bedrock (Bedrock add = dummy only)
- Storing strings, inventories, or NBT in fields — use world blocks/chests or edition-specific `@JavaOnly` data APIs (v2)
