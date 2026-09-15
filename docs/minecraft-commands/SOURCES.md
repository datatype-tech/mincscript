# Sources and extraction

Pinned while writing this spec (2026-09-15):

| Source | Role | URL |
| --- | --- | --- |
| Vanilla brigadier tree | Complete Java command graph | https://github.com/misode/mcmeta/blob/summary/commands/data.json (game `26.3-rc-3`) |
| Mojang brigadier | Java dispatcher model | https://github.com/Mojang/brigadier |
| Minecraft.net primer | Official player-facing command intro | https://www.minecraft.net/en-us/article/minecraft-commands |
| Microsoft Learn command index | Official Bedrock command list | https://learn.microsoft.com/minecraft/creator/commands/commands |
| MicrosoftDocs/minecraft-creator | Official Bedrock per-command signatures | https://github.com/MicrosoftDocs/minecraft-creator/tree/main/creator/Commands/commands |
| Microsoft Learn `/execute` | Official Bedrock execute signatures | https://learn.microsoft.com/minecraft/creator/commands/commands/execute |
| Microsoft Learn target selectors | Official Bedrock selector parameters | `creator/Documents/TargetSelectors.md` |
| Microsoft Learn functions / tick.json | `.mcfunction` (no `/`), 10k cap, `tick.json` 20/s, runs before world load | `creator/Documents/FunctionsIntroduction.md`, `TickJsonIntroduction.md` |
| Microsoft Learn command blocks | Impulse/Chain/Repeat, Conditional, Always Active, reward loop | `creator/Documents/CommandBlocks.md` |
| Microsoft Learn tickingarea + schedule | Chunks must tick; max 10 areas; `on_area_loaded` | `creator/Documents/TickingAreaCommand.md` |
| Microsoft Learn scoreboard / in-world game / snowball / NPC | dummy objectives, fake players, tags as teams, `@initiator` | `ScoreboardIntroduction.md`, `CreateAnInWorldGame.md`, `CommandBlockSnowballFight.md`, `NPCDialogue.md` |
| Official structure command | `saveMode` `disk` vs `memory` | `creator/Commands/commands/structure.md` |
| Bedrock Wiki (BCC) | Execute logic gates, scoreboard timers, functions | https://wiki.bedrock.dev/commands/logic-gates |
| Minecraft Wiki | Dual-edition semantics, selectors, NBT, argument types | https://minecraft.wiki/w/Commands and linked pages |

## Refresh

```bash
# Java tree
curl -fsSL https://raw.githubusercontent.com/misode/mcmeta/summary/commands/data.json \
  -o /tmp/mc-src/mcmeta/commands.json
curl -fsSL https://raw.githubusercontent.com/misode/mcmeta/summary/versions/data.json \
  -o /tmp/mc-src/mcmeta/versions.json

# Official Bedrock markdown (sparse)
git clone --depth 1 --filter=blob:none --sparse \
  https://github.com/MicrosoftDocs/minecraft-creator.git /tmp/mc-src/msdocs
git -C /tmp/mc-src/msdocs sparse-checkout set creator/Commands creator/Documents

python3 tools/extract_minecraft_commands.py
```

Handwritten grammar pages are **not** overwritten by the extractor.
