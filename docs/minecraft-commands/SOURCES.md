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
