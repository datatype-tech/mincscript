# Chains, controller, and world scenery

A **`.chain.mcs` file is one command-block machine**: an ordered list of commands with facing, Impulse/Chain/Repeat, Conditional, redstone, and delay. The **controller** is the only file that says *where* those machines sit, how they are **connected in space** (linear vs 堆叠 stack, …), and which extra blocks and chests belong in the same bounding box.

Regular `.mcs` classes are not placed unless a chain or `@OnTick` function calls them.

## One file, one chain

`src/chains/Play.chain.mcs`:

```java
pack metro.escape;

@Chain("Play")
@Repeat
@AlwaysActive
public chain Play {
    if (Match.phase == Phase.PLAY) {
        foreach (Player p : Players.all().withTag("in_round")) {
            as (p) {
                Runner.of(p).tickPlay();
            }
        }
        Match.playTick += 1;
        if (Match.playTick == 1200) {
            Extract.open();
        }
    }
}
```

Rules:

- Exactly one `chain Name { ... }` per `.chain.mcs`. The name matches the file stem (`Play`).
- The body is the **same language** as a method. The compiler **flattens** it to a sequence of commands (one CB per command, or packed — see packing).
- You may call `public` methods on classes. Those calls become `function …` **inside** a CB, or are inlined if the method is `@Inline` and small.
- No `class` in a chain file.

The first block’s type comes from annotations:

| Annotation | Block |
| --- | --- |
| `@Repeat` | Repeat (clock) |
| `@Impulse` | Impulse (default if neither Repeat nor the controller marks it clock) |
| `@AlwaysActive` / `@NeedsRedstone` | redstone mode |
| `@Delay(n)` | on the **next** statement (or the whole chain if on `chain`) |
| `@Conditional` | that statement is Conditional Chain (runs only if previous succeeded) |

`if` in the body is usually **execute if** inside **Unconditional** blocks, not Minecraft “Conditional” CBs. Use `@Conditional` when you want the official emerald-loop pattern (previous CB success). The controller may set `chains.prefer = "execute"` (default) or `"cb_conditional"`.

## Connection layouts (空间怎么排)

Minecraft chain CBs fire toward the **arrow**. The layout is the path of arrows.

| `layout` | 中文 | Geometry | Default facing |
| --- | --- | --- | --- |
| `linear` | 直线连锁 | one row along facing (east/west/north/south) | `east` |
| `stack` | **堆叠** | one column along Y | `up` (or `down`) |
| `snake` | 折返 | row of `max_span`, then step Y or Z, reverse | `east` |
| `box` | 箱体 | fill a cuboid in XYZ order (`order = "xzy"` etc.) | first axis |
| `points` | 点位表 | every statement has `at (x,y,z)` | each block’s own facing |

`stack` is the default in `minc.toml` for this design: easy to read in Creative, small footprint, arrows up.

The next block must sit on the **face the arrow points to**. The compiler sets `facing` on each CB. Illegal: two CBs with arrows that do not touch.

```
linear east, 4 commands:

[R]→[C]→[C]→[C]
```

```
stack up, 4 commands:

[C]  ↑
[C]  ↑
[C]  ↑
[R]  Repeat at bottom
```

`minc layout Play --layout stack --origin 2 70 0 --facing up` writes into the controller.

### Packing vs one-command-per-block

`[chains] pack = "one"` (default): one game command per CB. Easy to debug.

`pack = "function"`: the chain is a **single** Repeat/Impulse CB whose command is `function pack/chain/Play`, and the body lives in a function file. Still a “chain file” in source, but the world only gets 1–2 blocks. Use this when the chain is huge (10k limit still applies inside the function).

v1 can mix: controller says `Play uses pack.function`, `Lobby uses pack.one`.

## Controller (`src/controller.mcs`)

Reserved filename. One `controller` block.

```java
pack metro.escape;

@Controller
world MetroEscape {
    origin (0, 64, 0);
    dimension overworld;

    tickingArea "metro_station" circle (32, 64, 0) r=4 preload;

    chain Lobby  at (0, 70, 0)  layout linear facing east;
    chain Play   at (2, 64, 0)  layout stack  facing up;
    chain Extract at (4, 64, 0) layout stack  facing up;

    clock Play;

    // 链与链：红石或函数调用，不是箭头（箭头只在同一 chain 内）
    link Lobby.start => Play;
}
```

| Clause | Meaning |
| --- | --- |
| `origin` | all relative `at` values |
| `chain NAME at … layout … facing …` | must name a `.chain.mcs` |
| `clock NAME` | that chain is Repeat Always Active; others default Impulse unless annotated |
| `link A.event => B` | after a labeled point in A, `function` into B or place a redstone block at B’s first CB |
| `tickingArea` | Bedrock `/tickingarea`; Java → `/forceload` on the covered chunks |
| `include place Station` | pull `Station.place.mcs` |

Coordinates:

```java
chain Play at (2, 6, 0);                 // relative to origin
chain Play at absolute (100, 70, -20);
chain Play at (2, 6, 0) bound (8, 20, 4); // box layout size
```

CLI overrides the `at` / `layout` / `facing` of one chain without hand-editing the rest.

### Clock vs `tick.json`

If `emit.functions = true` and any `@OnTick` exists, Bedrock also gets `tick.json`. You can run **both** a Repeat CB and `tick.json` — usually you pick one. Controller:

```java
host tick = functions;   // 推荐：不占区块
host tick = command_block Play;
host tick = both;
```

Functions still need the pack applied. Command blocks still need the chunk (or ticking area).

## World files: fixed blocks and chests

`src/world/Station.place.mcs`:

```java
pack metro.escape.world;

place Station {
    gold_block at (0, 63, 0);

    fill (16, 56, -16) to (48, 56, 16) with stone_bricks;
    fill (16, 57, -16) to (48, 80, 16) with air replace water;

    set air at (40, 64, 1);  // 钥匙门开口，reset 时靠结构或再 fill

    chest crate1 at (32, 64, 4) facing west {
        slot 0: iron_ingot * 4;
        slot 1: bread * 2;
        slot 2: gold_ingot * 1;
        slot 13: written_book title "撤离须知" pages "带金锭到东侧站三秒";
    }

    barrel stash at (33, 64, 4) {
        slot 0: cooked_beef * 8;
    }
}
```

Rules:

- Block ids are edition-checked (`minecraft:gold_block`).
- `chest` / `barrel` / `shulker_box` / `hopper` / `dispenser` / `furnace` are **containers**. Slots are `slot.container.N` on Bedrock, NBT `Items` on Java.
- Counts `* n`. Data value / components: `gold_ingot * 1 data 0` (Bedrock), `diamond_sword * 1 {enchantments:{}}` Java-only.
- `place` is applied when the MINCB is **installed** into a world (placer tool / structure load), not every tick.
- Round reset: either `structure save/load` (Bedrock `disk`) generated from the same `place` AABB, or re-apply the container section.

Signs, skulls, and other block entities: v1 only supports listed containers + `command_block` (those come from chains, do not also `place` them by hand).

## Labeled steps inside a chain

```java
public chain Lobby {
    label start;
    Match.phase = Phase.COUNTDOWN;
    title(Players.queued(), Title.TITLE, "即将出发");
}
```

`link Lobby.start => Play` compiles to: at `label start`, emit `function …/Play` **or** activate Play’s first CB. Prefer function call so Play need not be adjacent in the world.

## What the controller does not do

- It does not contain gameplay `if` (keep that in chains/classes). Tiny `link` only.
- It does not invent command syntax. All CB `Command` strings are compiler output from chain bodies.
