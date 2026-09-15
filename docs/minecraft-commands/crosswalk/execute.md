# Execute: Java vs Bedrock

Both editions chain subcommands then `run <command>`. That is the only safe shared slogan. The graphs differ.

## Subcommands

| Subcommand | Java | Bedrock |
| --- | --- | --- |
| `as` `at` `in` `positioned` `positioned as` | yes | yes |
| `rotated` `rotated as` `facing` `facing entity` | yes | yes |
| `align` `anchored` | yes | yes |
| `positioned over <heightmap>` | yes | **no** |
| `on <relation>` | yes | **no** |
| `summon <entity>` | yes | **no** |
| `store result\|success …` | yes | **no** |
| `if/unless block` | predicate | Block + optional blockStates |
| `if/unless blocks` | all\|masked | all\|masked |
| `if/unless entity` | yes | yes |
| `if/unless score` op / matches | yes | yes |
| `if/unless data` | yes | **no** |
| `if/unless predicate` | yes | **no** |
| `if/unless function` | yes | **no** |
| `if/unless items` / `slots` | yes | **no** |
| `if/unless biome` `dimension` `loaded` `stopwatch` | yes | **no** |

## Termination

- Java: `run` **or** an executable condition.
- Bedrock: official conditions have **optional** chainedCommand; modifiers require a chain.

## Forking

| | Java | Bedrock |
| --- | --- | --- |
| Evaluation order | breadth-first | depth-first |
| Nested `run execute` | no extra effect | no extra effect |
| Depth-first workaround | `run function` | native |

## Compile mapping (lossy)

| Java | Bedrock substitute |
| --- | --- |
| `execute store result score A obj` | often `scoreboard players …` after a test, or `scriptevent` |
| `execute if data` | `hasitem` / `testfor` / `execute if entity` |
| `execute if predicate` | function + `/testfor` / score |
| `execute on vehicle` | `ride` queries / tags |
| `execute summon marker run …` | `/summon` then `/execute as @e[type=marker,c=1]` (racy) |

Never silently drop `store`; fail compilation when targeting Bedrock.
