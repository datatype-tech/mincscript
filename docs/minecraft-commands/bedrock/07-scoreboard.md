# Bedrock `/scoreboard`

Official overloads (generated `scoreboard.md`):

```
/scoreboard objectives add <objective: ScoreboardObjectives> dummy [displayName: string]
/scoreboard objectives remove <objective: ScoreboardObjectives>
/scoreboard objectives list
/scoreboard objectives setdisplay <displaySlot: ScoreboardDisplaySlotSortable> [objective] [sortOrder]
/scoreboard objectives setdisplay belowname [objective]
/scoreboard players list [playername: targets]
/scoreboard players reset <player: targets> [objective]
/scoreboard players test <player> <objective> <min: wildcard int> [max: wildcard int]
/scoreboard players random <player> <objective> <min: int> <max: int>
/scoreboard players <action: ScoreboardPlayersNumAction> <player> <objective> <count: int>
/scoreboard players operation <targetName> <targetObjective> <operation> <selector> <objective>
```

`ScoreboardPlayersNumAction` is `set` / `add` / `remove` (in-game `/help`). `dummy` is the only addable criterion. Display slots: `sidebar`, `list`, `belowname`.

`players test` is Bedrock-only (Java uses `execute if score`). Wildcard `*` for min/max means unbounded.

Selector after scores: `@a[scores={objectiveA=10..}]` (official).
