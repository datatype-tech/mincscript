# Illustrative sketch: 地铁逃生 (not compiled)

This is what a programmer would type. It is **not** a buildable project yet.

## `minc.toml`

```toml
[project]
name = "metro-escape"
pack = "metro.escape"
score_revision = 1

[target]
edition = "bedrock"
game_version = "1.21.70"

[world]
origin = [0, 64, 0]

[chains]
default_layout = "stack"
default_facing = "up"
clock = "Play"
```

## `src/logic/Match.mcs`

```java
pack metro.escape;

public enum Phase { LOBBY, COUNTDOWN, PLAY, EXTRACT, END }

public class Match {
    public static int phase;
    public static int countdown;
    public static int playTick;
}

public class Runner {
    private int keys;
    private int hold;
    private boolean extracted;
    private boolean dead;
    private boolean inRound;

    public static Runner of(Player p) { /* 编译器内建：同一实体 */ }

    public void queueFromGold() {
        if (block(BlockPos.here().up(-1)) == Blocks.GOLD_BLOCK) {
            this.inRound = true; // 简化：真实项目会拆 queued / inRound
        }
    }

    public void tickHold(Region extract) {
        if (!this.inRound || this.extracted || this.dead) {
            return;
        }
        if (this.keys >= 1 && Player.self().in(extract)) {
            this.hold += 1;
            if (this.hold >= 60) {
                this.extracted = true;
                Player.self().gamemode(GameMode.SPECTATOR);
                title(Player.self(), Title.TITLE, "撤离成功");
            }
        } else {
            this.hold = 0;
        }
    }
}
```

## `src/chains/Play.chain.mcs`

```java
pack metro.escape;

@Chain("Play")
@Repeat
@AlwaysActive
public chain Play {
    if (Match.phase == Phase.PLAY) {
        foreach (Player p : Players.all()) {
            as (p) at (p) {
                Runner.of(p).tickHold(ExtractZone.BOX);
            }
        }
        Match.playTick += 1;
        if (Match.playTick == 1200) {
            Match.phase = Phase.EXTRACT;
            title(Players.all(), Title.TITLE, "撤离点已开启");
        }
    }
}
```

`keys` stays private: the chain never writes `p.keys`. Loot/key is a public method on `Runner` called from a crate chain.

## `src/controller.mcs`

```java
pack metro.escape;

@Controller
world MetroEscape {
    origin (0, 64, 0);

    tickingArea "metro_station" circle (32, 64, 0) r=4 preload;

    chain Lobby   at (0, 70, 0) layout linear facing east;
    chain Play    at (2, 64, 0) layout stack  facing up;
    chain Extract at (4, 64, 0) layout stack  facing up;

    clock Play;
    host tick = functions;

    include place Station;
}
```

`minc layout Play --layout stack --origin 2 70 0 --facing up` only rewrites the `chain Play` line.

## `src/world/Station.place.mcs`

```java
pack metro.escape.world;

place Station {
    gold_block at (0, 63, 0);
    chest crate1 at (32, 64, 4) facing west {
        slot 0: iron_ingot * 4;
        slot 2: gold_ingot * 1;
    }
}
```

After `minc build`, `dist/metro-escape.mincb` holds: objective ids for `Match.phase` / `Runner.keys` / …, stacked CBs at `(2,64,0)` going up, the gold block, and the chest slots. The placer puts that in the chunk; `tick.json` still runs `@OnTick` / packed functions if `host tick = functions`.
