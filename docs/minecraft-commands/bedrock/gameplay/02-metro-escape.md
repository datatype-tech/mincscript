# 地铁逃生：把基岩命令交叉组合成一局搜打撤

地铁逃生不是一条新命令。它是 **大厅匹配 → 倒计时 → 搜刮 → 钥匙/门 → NPC → 撤离窗口 → 结算重置** 这一条状态机，用上一页的六个轴填满。和平精英的模式规则定义“玩什么”；微软官方样例、Bedrock Commands Community、量筒函数模板、以及基岩版地铁逃生房间教程，定义“命令怎么写才跑得起来”。

下面按 **系统** 拆，每个系统都写清：状态存在哪、谁每刻执行、和哪些命令交叉。坐标以示例包为准（大厅 `0 64 0`，站台 `32 64 0`，箱 `32 64 4`，撤离 `64 64 0`），落地时只换数字。

## 一局的状态机

Fake player `metro`，objective `metro`（再加 `metro_timer` 等）：

| `phase` | 名 | 每刻做什么 | 退出条件 |
| --- | --- | --- | --- |
| 0 | lobby | 站在金块/NPC 按钮 → `tag queued` | `function metro/match_start` |
| 1 | countdown | 锁移动，镜头，倒计时 title | `countdown` 到阈值 |
| 2 | playing | 箱、钥匙、死亡、到点开撤离 | 到时或全灭 |
| 3 | extract | 区内 hold 分；到点 `extracted` | 全员抽出/死亡/窗口结束 |
| 4 | end | title，清 tag，`structure load` | 短延迟后 `phase 0` |

`tick.json` → `metro/tick` → bootstrap → `metro/loop` 按 `phase` 调子函数。这就是量筒说的：函数缺的是 **条件**（新 execute 已补）和 **时间**（tick + scoreboard）。不要把五个阶段写进同一个 400 行文件里——交叉组合的单位是 **函数**，不是聊天栏。

```
tick.json
  metro/tick          保证 objective，分发 init/loop
    metro/init        一次：gamerule、tickingarea、schedule、phase=0
    metro/loop        五路 phase
      metro/phase_*   只读本阶段该读的选择器
```

## 成熟案例怎么映射到这五个阶段

### 官方：命令方块奖励环 / Complete the Monument

- **检测**（`testforblock` / `if block`）× **尚未领过**（tag 或 fake 分=0）× **发奖** × **打标**。
- Monument 用 `#red` `#green` `#blue` 当布尔，`operation += *` 当“三色齐了”。
- 地铁逃生的 **物资箱只开一次**、**钥匙只提示一次**、**三件神器才能开金库** 就是同一套：把 `wool_placed` 换成 `crate1` / `metro_key` / 计分。

### 官方：雪球大战

- **tag = 队伍**；随机分队用 `players random` + `execute if score matches`（criterion 必须是 `dummy`）。
- `execute as @e[type=snowball] at @e[family=player,c=1] if entity @e[type=snowball,r=1] run damage …` 是 **投射物 × family × 半径 × damage** 的交叉。地铁里的投掷物陷阱、手雷、自定义弹可以同一骨架（或交给 Script）。

### 官方：NPC Dialogue + samples

- 男团 / 商人 / 匹配员 = NPC + `dialogue/*.json`。
- 按钮命令用 `@initiator`，不要 `@p`（多人会领错）。
- `/dialogue open` 可把藏在 ticking area 里的 NPC 当 UI。
- `on_open_commands` / `on_close_commands` 做进场清包、离场恢复。

### 社区：BCC 逻辑门 + 计时器

- 撤离资格 OR：钥匙 **或** 撤离符，用 `unless` + `hasitem quantity=0`。
- 撤离 AND：`in_round` ∧ 在 AABB ∧ hold≥60。
- 倒计时、撤离窗口、物资刷新周期 = scoreboard timer，不是 Repeat CB 的 Delay。

### 社区：量筒主包

- `tick.json` 只跑 `main`；时间线与“每刻检测”分开，避免 10k 上限和低端机卡顿。
- 地铁逃生：`phase_playing` 里只做 **选择器命中才 `run function`**，不要无差别 `@e` 扫描全维度。

### 地铁逃生房间教程（物资箱、男团、匹配、钥匙房、撤离倒计时）

公开建造/指令视频已经把玩法拆成这些房间。命令层不必抄某张商业图，只要承认：**成熟案例用的就是这些房间 = 这些函数**。玩法规则对齐和平精英地铁模式：搜箱、打 NPC/玩家、钥匙开金库、到撤离点站住再走。

商业图常见再加：枪械附加包、自定义实体、Script UI。**对局状态机仍应留在函数里**，否则 `/reload` 和存档分离会把逻辑锁死在世界里的命令方块上。

## 子系统（交叉组合清单）

### 1. 匹配 / 大厅

| 元素 | 组合 |
| --- | --- |
| 排队区 | `x y z dx dy dz` + `tag add queued` |
| 开始 | 站在金块上：`execute as @a at @s if block ~ ~-1 ~ gold_block run tag @s add queued` 然后 `if entity @a[tag=queued] run function metro/match_start` |
| 或 NPC | scene 按钮 `tag @initiator add queued` 或直接 `function metro/match_start` |
| 人数 | `execute if entity @a[tag=queued,c=2]` 当“至少两人”；精确计数用假玩家 `operation` 累加或 Script |
| 模式 | `gamemode adventure`，`gamerule keepinventory …` 按你要不要掉包 |

`match_start` 交叉：改 `phase`、`teleport`、`spawnpoint`（死回大厅）、`tag in_round`、`inputpermission` 锁、`camera` 过场、`music play`。

### 2. 倒计时 / 过场

```
phase=1
inputpermission movement+camera disabled
camera set minecraft:free ease … pos … facing 站台
title times + title 数字
countdown += 1
matches 200 → begin_play
```

`begin_play`：解锁 input、`camera set minecraft:first_person`、`phase=2`、`playsound`。这是 **clock × permission × camera × title**，没有新语法。

### 3. 物资箱

两种成熟做法：

1. **玩家走进 AABB 且无 `crate1`** → `give` / `loot give` → `tag add crate1`（函数版官方奖励环）。
2. **真箱子** + `/loot insert <chest> loot tables/...` 在 `reset` / `on_station_loaded` 填好；玩家用原版开箱。回合用 `structure load` 把箱子复位。

`item_lock` in `/give` components 防止钥匙被丢掉（冒险图惯例）。

搜索“已搜过”用 tag 或假玩家分，不要 `execute if data`。

### 4. 钥匙房

```
execute as @a[tag=in_round,hasitem={item=gold_ingot,quantity=1..},tag=!metro_key] run function metro/on_key
```

门：

```
execute if entity @a[tag=metro_key,x=40,y=64,z=0,dx=2,dy=3,dz=2] run setblock 40 64 1 air
```

或 `structure load` 一块“开门”结构。金库：`hasitem` 钥匙 **and** 在门 AABB **and** `phase matches 2..3`。

### 5. 男团 / NPC 敌

- 对话男团：NPC + scene，按钮给任务 tag / 传送 / 开商店（`/give` 换 `clear` 货币）。
- 敌对“男团”：自定义实体或盔甲架 + `damage` / `event entity`；仇恨与枪械通常是附加包。命令侧只负责 **刷怪时机**（`phase_playing` + `play_tick matches`）和 **死亡发钥匙**（Script 或 `hasitem` 掉落物追踪）。
- 不要每刻 `summon`。用 `unless entity @e[type=npc,tag=metro_boss]` 限一次。

### 6. 撤离点

和平精英：到点 **坚守一段时间**。这是 **空间 AABB × 每刻 `scoreboard players add @s hold` × `matches 60..` × `unless` 离开清零**：

```
execute as @a[tag=in_round,tag=metro_key,tag=!extracted,x=64,y=64,z=0,dx=3,dy=3,dz=3] run scoreboard players add @s metro_hold 1
execute as @a[tag=in_round] unless entity @s[x=64,y=64,z=0,dx=3,dy=3,dz=3] run scoreboard players set @s metro_hold 0
execute as @a[tag=in_round,scores={metro_hold=60..},tag=!extracted] run function metro/do_extract
```

社区教程里的“可调撤离快慢”就是改 `60` 或改 `phase` 里窗口长度（`extract_timer`）。窗口本身是世界时钟，和玩家 hold **不是**同一个分。

抽出：`tag extracted`、`gamemode spectator` 或 `teleport` 回看台、`tellraw`、`scriptevent`。

全灭 / 时间到：`unless entity @a[tag=in_round,tag=!extracted,tag=!metro_dead]` → `phase 4`。

### 7. 死亡

基岩 dummy 没有死亡准则。约定：

- `spawnpoint @a[tag=in_round] 0 64 0`（大厅）
- 游戏中站内出生点不要设在大厅
- `phase` 为 2 或 3 时，还带着 `in_round` 却出现在大厅 AABB → `function metro/on_death`

`on_death`：`tag metro_dead`、`gamemode spectator`、清钥匙或不清（按规则）。`keepinventory false` 则包掉在站内，用实体清理函数扫 `type=item`。

### 8. 回合重置（保存的是结构，实行的是 load）

作者在造景后 **一次** `function metro/save_station`（`structure save … disk`）。

`reset_round`：

- `tag` 全摘、`clear`、`gamemode adventure`、`teleport` 大厅
- `inputpermission` 解锁
- `structure load metro_station …`（区块必须在 ticking area）
- `scoreboard players set metro phase 0` 及各类 timer 归零

这是“保存指令实行”在玩法上的含义：**逻辑保存在包里，关卡快照保存在世界 disk 结构里，每局实行 load**。

### 9. 演出层（可拆）

| 时刻 | 命令 |
| --- | --- |
| 进站 | `music play`, `fog push`, `camera` |
| 开箱 | `playsound random.chestopen`, `particle` |
| 撤离开启 | `titleraw`, `camerashake add` |
| 胜利 | `camera` replay, `hud hide` 做过场 |

全部在 `match_start` / `open_extract` / `do_extract` 里各一行，不要每刻推一次 fog。

## 性能与上限（地图翻车点）

- 每刻 `@e` 无半径 = 低端机死亡。始终带 `r=` / AABB / `c=`。
- 嵌套 function 计入 10000。重置用 `structure load` 而不是循环 `setblock`。
- 最多 10 个 ticking area。大厅 + 站台 + 撤离够用；不要给每个箱子一块。
- `tick.json` 在世界未加载完就跑：只允许 bootstrap + `if score inited`。对实体的工作放进 `on_join` / `on_area_loaded`。

## MincScript / 编译器要点

编译一局地铁逃生时，后端应发出：

1. `tick.json` 条目（否则“保存了函数”也不会实行）
2. `min_engine_version` ≥ 1.19.50
3. 仅 82 命令 + 官方选择器
4. 阶段枚举 → `execute if score metro phase matches N run function …`
5. 拒绝 Java `store`/`team`/`return`/`nbt=`

示例包 [`examples/bedrock-metro-escape/`](../../../../examples/bedrock-metro-escape/) 是最小可执行骨架：它故意用 `gold_ingot` 当钥匙、`give` 当箱子，避免依赖自定义战利品表。换表、换枪、换男团模型不改状态机。
