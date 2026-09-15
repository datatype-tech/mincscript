# MincScript 语法教程

这份教程按「你写什么 → 编译器吐出什么指令」来讲。MincScript 长得像小 Java，但 **没有堆、没有运行时数组、没有 `null`**。`int` 是记分板，`boolean` 实例字段是标签。**基岩版和 Java 版各打一套指令**，不要假设某一行在两边都能跑。

更短的规格表见 [03 语法](03-syntax.md)。命令怎么分版见 [`docs/minecraft-commands/`](../minecraft-commands/README.md)。

## 1. 一个能编译的最小文件

```java
pack demo.kit;

public class Kit {
    public static void open() {
        say("hello");
        give(Players.all(), Items.GOLD_INGOT.count(1));
    }
}
```

`minc.toml` 里的 `target.edition` 决定这一次构建吐基岩还是 Java。同一份源码可以打两次：

```toml
[target]
edition = "bedrock"   # 或 java
game_version = "1.21.70"
```

## 2. 类型：什么会变成记分板，什么不会

| 你写 | 运行时 | 不要当成 |
| --- | --- | --- |
| 类上的 `int` 字段 | dummy 记分板 | 临时计算器 |
| 实例 `boolean` | `/tag` | 记分板 0/1（静态 boolean 才是） |
| `Player` / `Entity` | 选择器字符串 | 对象指针 |
| `BlockPos` / `Item` / `Region` | 编译期描述符 | 世界里的实体 |
| `Seq<T>` | 一条选择器 | 列表 |
| `List<T>` | **编译期展开** | Java `ArrayList` |
| `temp …` | 用一次就扔掉 | 记分板目标 |
| `String` | 只存在于编译期 | 玩家名字 |

类字段会进 MINCB 的符号表（`m00`、`t0a`…）。**局部 `temp`、`List`、一次性物品描述不会进计数表。**

## 3. 语句：一条源码 → 一条（或一组）指令

三种「直接执行一条指令」的写法，语义相同：都发出 **一行、无前导 `/`** 的 mcfunction 文本。

```java
cmd("say hello");          // 表达式，字符串原样（去掉前导 /）
run "say hello";          // 语句
run say hello;            // 语句：分号前的原文
/say hi;                  // 语句：斜杠命令
say("kit");               // 类型化内置，两边都是 `say kit`
```

`minc check --strict-raw` 会禁止 `cmd` / `run` / `/`。能用类型化内置就不要手写。

手写时编译器仍会用当前 edition 的命令快照检查首词，并拒绝基岩上的 `execute store`、`team`、`nbt=`、`distance=`、`item[`。

## 4. 临时变量、临时对象（不是记分板）

`temp` 修饰局部名字：**出现在一条语句里之后立刻销毁**。编译器不会给它 `scoreboard objectives add`。

```java
temp int flash = 3;                          // 内联成字面量 3
temp Item key = Items.GOLD_INGOT.count(1);    // 编译期物品，不是箱子、不是记分
temp Player everyone = Players.all();        // 选择器句柄
temp BlockPos pad = BlockPos.of(0, 64, 0);

give(everyone, key);                         // 用掉 key 和 everyone
setblock(pad, Blocks.GOLD_BLOCK);            // 用掉 pad
Match.score = flash;                         // 用掉 flash → `scoreboard players set … 3`
// Match.other = flash;                      // 错误：已经消费过
```

复杂算式仍可能用编译器私有的 `#tN mt`（目标名 `mt`，假玩家 `#t0`）。那是 **一条指令内部的草稿纸**，算完立刻：

```
scoreboard players operation #t0 mt = …
scoreboard players operation mcs m00 = #t0 mt
scoreboard players reset #t0 mt
```

`#tN` 不会出现在你的类字段表里，也不会留在计分板上当「计数」。

| 写法 | 会不会 `objectives add` 你的名字 |
| --- | --- |
| `private int keys;` 字段 | 会（`m00` 这类短名） |
| `int x = 1;` 局部常量 | 否，内联 |
| `temp int x = 1;` | 否，内联且一次性 |
| `temp int x = a + b;` | 否；只用 `mt` 草稿纸，随后 `reset` |

## 5. 列表：`List.of` + `foreach` 展开

`Seq<T>` 是选择器。`List<T>` 是 **编译期元组**：`foreach` 按元素复制循环体，不生成运行时数组、不读写箱子。

```java
List<Item> loot = List.of(
    Items.GOLD_INGOT.count(1),
    Items.IRON_INGOT.count(4)
);
foreach (Item it : loot) {
    give(Players.all(), it);
}
```

基岩会得到两行：

```
give @a gold_ingot 1
give @a iron_ingot 4
```

Java 会得到：

```
give @a minecraft:gold_ingot 1
give @a minecraft:iron_ingot 4
```

也可以直接写：

```java
foreach (int t : List.of(20, 40, 60)) {
    // 展开成三条，t 分别是 20 / 40 / 60
}
```

`int[]`、运行时 `new List()`、往 List 里 `.add` 仍然非法。运行时集合请用记分、标签或箱子。

## 6. 选择器（编译期拼出来）

```java
Players.all()
    .inBox(64, 64, 0, 3, 3, 3)
    .withTag("metro_key")
    .withoutTag("extracted")
    .hasItem(Items.GOLD_INGOT, 1)
    .within(BlockPos.of(0, 64, 0), 4);
```

| API | 基岩 | Java |
| --- | --- | --- |
| `.within(origin, r)` | `r=4` | `distance=..4` |
| `.hasItem(item, n)` | `hasitem={item=…,quantity=n..}` | `execute if items entity @s container.* minecraft:… n..` |
| `Players.raw("@a[…]")` | 按基岩选择器检查 | 按 Java 检查（基岩的 `hasitem=` 会报错） |

`as (p) at (p) { … }` 是语句前缀，不是 `Player` 上的方法。

## 7. 类型化世界指令（两边语法不同的，编译器会拆开）

这些是语句/表达式内置，不是记分板。

```java
give(Players.all(), Items.GOLD_INGOT.count(1));
kill(Players.entities().withTag("dead"));
effect(Players.all(), "speed", 10, 1);
effect(Players.self(), "clear");
clear(Players.self(), Items.DIRT);
playsound("random.levelup", Players.all(), BlockPos.here(), 1);
particle("minecraft:villager_happy", BlockPos.of(0, 64, 0));
summon("zombie", BlockPos.of(0, 64, 0));
setblock(BlockPos.of(0, 63, 0), Blocks.GOLD_BLOCK);
title(Players.all(), Title.TITLE, "地铁逃生");
tellraw(Players.all(), Text.raw("§e撤离点已开启"));
weather("clear");
time("night");
difficulty("peaceful");
xp(Players.self(), 5);
xp(Players.self(), 5, true);          // 等级
enchant(Players.self(), "sharpness", 1);
replaceItem(Players.self(), "hotbar.0", Items.GOLD_INGOT.count(1));
Player.self().gamemode(GameMode.ADVENTURE);
Player.self().teleport(BlockPos.of(0, 64, 0));
World.gamerule("commandblockoutput", false);
```

物品还可以：

```java
Items.GOLD_INGOT.count(1).data(0);                 // 仅基岩 aux
Items.DIAMOND_SWORD.component("enchantments", "{levels:{\"minecraft:sharpness\":5}}");
                                                    // Java 打成 id[enchantments=…]
```

### 同一调用，两边发出的指令

| 源码 | 基岩 | Java |
| --- | --- | --- |
| `give(@a, gold×2)` | `give @a gold_ingot 2` | `give @a minecraft:gold_ingot 2` |
| `.data(7)` | `give … 2 7` | **编译错误** |
| `.component(k,v)` | `give … amount data {json}` | `give … id[k=v] count` |
| `effect(…, speed, 10, 1)` | `effect @a speed 10 1 false` | `effect give @a minecraft:speed 10 1 false` |
| `effect(…, "clear")` | `effect @s clear` | `effect clear @s` |
| `tellraw` | `{"rawtext":[{"text":"…"}]}` | `{"text":"…"}` |
| `title` 正文 | 纯文本 | `{"text":"…"}` |
| `xp(p, 5)` | `xp 5 @s` | `xp add @s 5 points` |
| `xp(p, 5, true)` | `xp 5L @s` | `xp add @s 5 levels` |
| `replaceItem(…, hotbar.0, …)` | `replaceitem entity @s slot.hotbar.0 …` | `item replace entity @s hotbar.0 with …` |
| `playsound` | `playsound <声> <玩家> [坐标]` | 多一个 **source**（默认 `master`） |
| `summon` 带 `{NoAI:1b}` | **错误**（基岩没有这条 SNBT） | `summon minecraft:zombie … {…}` |
| `.within(…, 4)` | `r=4` | `distance=..4` |
| `return;`（函数里） | 用 `if` 把门，**不**发 `/return` | `return` |
| `random(0, 10)` | `scoreboard players random` | **错误**（Java 没有这条） |

`Player` 上也可以 `p.give(item)`、`p.kill()`、`p.effect(...)`、`p.clear(...)`，和顶层函数是同一套打印机。

## 8. 类、记分板、控制流（和 Java 像的那部分）

```java
public class Runner {
    private int keys;              // 每玩家 dummy
    private boolean extracted;     // 标签

    public void giveKey() {
        this.keys += 1;
        this.extracted = false;
    }
}

public enum Phase { LOBBY, PLAY, EXTRACT }

if (Match.phase == Phase.PLAY) {
    Runner.of(p).tickHold();
} else {
    Lobby.tick();
}

foreach (Player p : Players.all().withTag("in_round")) {
    as (p) at (p) {
        Runner.of(p).giveKey();
    }
}

switch (Match.phase) {
    case Phase.LOBBY -> Lobby.tick();
    case Phase.PLAY -> Play.tick();
    default -> { }
}
```

`a + b` 这种运行时整数运算会走 `scoreboard players operation` 和一次性 `#tN mt`，不是 `temp` 关键字，但也 **不会** 给你的变量建目标。

## 9. 文件角色

| 文件 | 角色 |
| --- | --- |
| `*.mcs` | 类、枚举、静态逻辑 |
| `*.chain.mcs` | **一条**命令方块链 |
| `controller.mcs` | 世界原点、链坐标、ticking area、箱子 include |
| `*.place.mcs` | `place` / `fill` / `chest` |

```java
pack demo.kit;

@Controller
world KitWorld {
    origin (0, 64, 0);
    chain Demo at (2, 64, 0) layout stack facing up;
    clock Demo;
}
```

## 10. 非法写法（看起来像 Java，这里没有）

| 非法 | 用这个 |
| --- | --- |
| `new Player()` | `Players.self()` / `Players.all()` |
| `null` | `exists() == false` |
| `int[]` / 运行时 `List` | `List.of` 展开，或记分 / 箱子 |
| `try/catch` | 不要 |
| 基岩上 `Players.all().count()` | `.exists()`（没有 `execute store`） |
| 基岩上 `.component` / Java 上 `.data` | 看第 7 节 |

完整例子：[illustrative 地铁逃生](example-metro.md)，可构建工程：`examples/metro-escape/`、`examples/tutorial-kit/`。
