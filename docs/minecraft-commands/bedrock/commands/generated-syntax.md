# Bedrock Edition command usages (generated)

Official signatures from Mojang's `@minecraft/api-docs-generator` markdown
([MicrosoftDocs/minecraft-creator](https://github.com/MicrosoftDocs/minecraft-creator/tree/main/creator/Commands/commands)),
cross-checked against Minecraft Wiki Syntax sections where a page exists.

This file is generated; do not edit by hand.

Root commands: **82**.

## `aimassist`

- Permission: Game Directors
- Requires cheats: Yes

### Official


## `allowlist`

- Permission: Owner
- Requires cheats: Yes

### Official

- `/allowlist <action: AllowListAction> [name: string]`

## `camera`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/camera <players: target> set <preset: CameraPresets> ease <easeTime: float> <easeType: Easing> pos <position: x y z> rot <xRot: rotation> <yRot: rotation>`
- `/camera <players: target> set <preset: CameraPresets> ease <easeTime: float> <easeType: Easing> pos <position: x y z> facing <lookAtEntity: target>`
- `/camera <players: target> set <preset: CameraPresets> ease <easeTime: float> <easeType: Easing> pos <position: x y z> facing <lookAtPosition: x y z>`
- `/camera <players: target> set <preset: CameraPresets> ease <easeTime: float> <easeType: Easing> pos <position: x y z>`
- `/camera <players: target> set <preset: CameraPresets> ease <easeTime: float> <easeType: Easing> rot <xRot: rotation> <yRot: rotation>`
- `/camera <players: target> set <preset: CameraPresets> ease <easeTime: float> <easeType: Easing> facing <lookAtEntity: target>`
- `/camera <players: target> set <preset: CameraPresets> ease <easeTime: float> <easeType: Easing> facing <lookAtPosition: x y z>`
- `/camera <players: target> set <preset: CameraPresets> ease <easeTime: float> <easeType: Easing> [default: default]`
- `/camera <players: target> set <preset: CameraPresets> pos <position: x y z> rot <xRot: rotation> <yRot: rotation>`
- `/camera <players: target> set <preset: CameraPresets> pos <position: x y z> facing <lookAtEntity: target>`
- `/camera <players: target> set <preset: CameraPresets> pos <position: x y z> facing <lookAtPosition: x y z>`
- `/camera <players: target> attach_to_entity <entity: target>`
- `/camera <players: target> detach_from_entity`
- `/camera <players: target> play_spline <name: string>`
- `/camera <players: target> target_entity <entity: target>`
- `/camera <players: target> target_entity <entity: target> target_center_offset <xTargetCenterOffset: float> <yTargetCenterOffset: float> <zTargetCenterOffset: float>`
- `/camera <players: target> remove_target`
- `/camera <players: target> set <preset: CameraPresets> view_offset <xViewOffset: float> <yViewOffset: float>`
- `/camera <players: target> set <preset: CameraPresets> entity_offset <xEntityOffset: float> <yEntityOffset: float> <zEntityOffset: float>`
- `/camera <players: target> set <preset: CameraPresets> rot <xRot: rotation> <yRot: rotation> view_offset <xViewOffset: float> <yViewOffset: float>`
- `/camera <players: target> set <preset: CameraPresets> rot <xRot: rotation> <yRot: rotation> entity_offset <xEntityOffset: float> <yEntityOffset: float> <zEntityOffset: float>`
- `/camera <players: target> set <preset: CameraPresets> view_offset <xViewOffset: float> <yViewOffset: float> entity_offset <xEntityOffset: float> <yEntityOffset: float> <zEntityOffset: float>`
- `/camera <players: target> set <preset: CameraPresets> rot <xRot: rotation> <yRot: rotation> view_offset <xViewOffset: float> <yViewOffset: float> entity_offset <xEntityOffset: float> <yEntityOffset: float> <zEntityOffset: float>`
- `/camera <players: target> set <preset: CameraPresets> ease <easeTime: float> <easeType: Easing> view_offset <xViewOffset: float> <yViewOffset: float>`
- `/camera <players: target> set <preset: CameraPresets> ease <easeTime: float> <easeType: Easing> entity_offset <xEntityOffset: float> <yEntityOffset: float> <zEntityOffset: float>`
- `/camera <players: target> set <preset: CameraPresets> ease <easeTime: float> <easeType: Easing> rot <xRot: rotation> <yRot: rotation> view_offset <xViewOffset: float> <yViewOffset: float>`

## `camerashake`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/camerashake add <player: target> [intensity: float] [seconds: float] [shakeType: CameraShakeType]`
- `/camerashake stop [player: target]`

## `changesetting`

- Permission: Owner
- Requires cheats: Yes

### Official

- `/changesetting allow-cheats <value: Boolean>`
- `/changesetting difficulty <value: Difficulty>`
- `/changesetting difficulty <value: int>`

## `clear`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/clear [player: target] [itemName: Item] [data: int] [maxCount: int]`

### Wiki (Bedrock Syntax section)

- `clear [player: target] [itemName: Item] [data: int] [maxCount: int]`

## `clearspawnpoint`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/clearspawnpoint [player: target]`

## `clone`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/clone <begin: x y z> <end: x y z> <destination: x y z> [maskMode: MaskMode] [cloneMode: CloneMode]`
- `/clone <begin: x y z> <end: x y z> <destination: x y z> filtered <cloneMode: CloneMode> <tileName: Block> [blockStates: block properties]`

### Wiki (Bedrock Syntax section)

- `clone [maskMode: MaskMode] [cloneMode: CloneMode]`
- `clone filtered [blockStates: block states]`

## `controlscheme`

- Permission: Game Directors
- Requires cheats: Yes

### Official


## `damage`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/damage <target: target> <amount: int> [cause: DamageCause]`
- `/damage <target: target> <amount: int> <cause: DamageCause> entity <damager: target>`

### Wiki (Bedrock Syntax section)

- `damage entity`
- `damage [cause: DamageCause]`

## `daylock`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/daylock [lock: Boolean]`

## `deop`

- Permission: Admin
- Requires cheats: No

### Official

- `/deop <player: target>`

### Wiki (Bedrock Syntax section)

- `deop`

## `dialogue`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/dialogue open <npc: target> <player: target> [sceneName: string]`
- `/dialogue change <npc: target> <sceneName: string> [players: target]`

## `difficulty`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/difficulty <difficulty: Difficulty>`
- `/difficulty <difficulty: int>`

### Wiki (Bedrock Syntax section)

- `difficulty`
- `difficulty`

## `effect`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/effect <player: target> clear [effect: Effect]`
- `/effect <player: target> <effect: Effect> [seconds: int] [amplifier: int] [hideParticles: Boolean]`

### Wiki (Bedrock Syntax section)

- `effect [seconds: int] [amplifier: int] [hideParticles: Boolean]`
- `effect infinite [amplifier: int] [hideParticles: Boolean]`
- `effect clear [effect: Effect]`

## `enchant`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/enchant <player: target> <enchantmentName: Enchant> [level: int]`
- `/enchant <player: target> <enchantmentId: int> [level: int]`

## `event`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/event entity <target: target> <eventName: EntityEvents>`

## `execute`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/execute as <origin: target> <chainedCommand: executechainedoption_0>`
- `/execute at <origin: target> <chainedCommand: executechainedoption_0>`
- `/execute in <dimension: Dimension> <chainedCommand: executechainedoption_0>`
- `/execute positioned <position: x y z> <chainedCommand: executechainedoption_0>`
- `/execute positioned as <origin: target> <chainedCommand: executechainedoption_0>`
- `/execute rotated <yaw: rotation> <pitch: rotation> <chainedCommand: executechainedoption_0>`
- `/execute rotated as <origin: target> <chainedCommand: executechainedoption_0>`
- `/execute facing <position: x y z> <chainedCommand: executechainedoption_0>`
- `/execute facing entity <origin: target> <anchor: ActorLocation> <chainedCommand: executechainedoption_0>`
- `/execute align <axes: string> <chainedCommand: executechainedoption_0>`
- `/execute anchored <anchored: ActorLocation> <chainedCommand: executechainedoption_0>`
- `/execute <subcommand: Option_If_Unless> block <position: x y z> <block: Block> [chainedCommand: executechainedoption_0]`
- `/execute <subcommand: Option_If_Unless> block <position: x y z> <block: Block> <blockStates: block properties> [chainedCommand: executechainedoption_0]`
- `/execute <subcommand: Option_If_Unless> blocks <begin: x y z> <end: x y z> <destination: x y z> <scan mode: BlocksScanMode> [chainedCommand: executechainedoption_0]`
- `/execute <subcommand: Option_If_Unless> entity <target: target> [chainedCommand: executechainedoption_0]`
- `/execute <subcommand: Option_If_Unless> score <target: target> <objective: ScoreboardObjectives> <operation: compareoperator> <source: target> <objective: ScoreboardObjectives> [chainedCommand: executechainedoption_0]`
- `/execute <subcommand: Option_If_Unless> score <target: target> <objective: ScoreboardObjectives> matches <range: fullintegerrange> [chainedCommand: executechainedoption_0]`
- `/execute run <command: codebuilderargs>`

## `fill`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/fill <from: x y z> <to: x y z> <tileName: Block> <blockStates: block properties> [oldBlockHandling: FillMode]`
- `/fill <from: x y z> <to: x y z> <tileName: Block> [oldBlockHandling: FillMode]`
- `/fill <from: x y z> <to: x y z> <tileName: Block> <blockStates: block properties> replace [replaceTileName: Block] [replaceBlockStates: block properties]`
- `/fill <from: x y z> <to: x y z> <tileName: Block> replace [replaceTileName: Block] [replaceBlockStates: block properties]`

### Wiki (Bedrock Syntax section)

- `fill [oldBlockHandling: FillMode]`
- `fill [oldBlockHandling: FillMode]`
- `fill replace [replaceTileName: Block] [replaceBlockStates: block states]`
- `fill replace [replaceTileName: Block] [replaceBlockStates: block states]`

## `fog`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/fog <victim: target> push <fogId: string> <userProvidedId: string>`
- `/fog <victim: target> <mode: delete> <userProvidedId: string>`

## `function`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/function <name: filepath>`

### Wiki (Bedrock Syntax section)

- `function`

## `gamemode`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/gamemode <gameMode: GameMode> [player: target]`
- `/gamemode <gameMode: int> [player: target]`

### Wiki (Bedrock Syntax section)

- `gamemode [player: target]`
- `gamemode [player: target]`

## `gamerule`

- Permission: Game Directors
- Requires cheats: No

### Official

- `/gamerule`
- `/gamerule playerwaypoints <value: playerwaypointsValues>`

### Wiki (Bedrock Syntax section)

- `gamerule [value: Boolean]`
- `gamerule [value: int]`

## `gametest`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/gametest runthis`
- `/gametest run <testName: GameTestName> [rotationSteps: int]`
- `/gametest run <testName: GameTestName> <stopOnFailure: Boolean> <repeatCount: int> [rotationSteps: int]`
- `/gametest runset [tag: GameTestTag] [rotationSteps: int]`
- `/gametest runsetuntilfail [tag: GameTestTag] [rotationSteps: int]`
- `/gametest clearall`
- `/gametest pos`
- `/gametest create <testName: string> [width: int] [height: int] [depth: int]`
- `/gametest runthese`
- `/gametest stopall`

## `give`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/give <player: target> <itemName: Item> [amount: int] [data: int] [components: json]`

### Wiki (Bedrock Syntax section)

- `give [amount: int] [data: int] [components: json]`

## `help`

- Permission: Any
- Requires cheats: No

### Official

- `/help [command: CommandName]`
- `/help <page: int>`

### Wiki (Bedrock Syntax section)

- `help`
- `help [page: int]`

## `hud`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/hud <target: target> <visible: HudVisibility> [hud_element: HudElement]`

## `inputpermission`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/inputpermission set <targets: target> <permission: permission> <state: state>`
- `/inputpermission query <targets: target> <permission: permission> [state: state]`

## `kick`

- Permission: Game Directors
- Requires cheats: No

### Official

- `/kick <name: target> <reason: message>`

### Wiki (Bedrock Syntax section)

- `kick [reason: message]`

## `kill`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/kill [target: target]`

### Wiki (Bedrock Syntax section)

- `kill [target: target]`
- `kill @s`
- `kill`
- `kill Steve`
- `kill @e[type=item]`
- `kill @e[type=!player]`
- `kill @e[distance=..10,type=creeper]`
- `kill @e[r=10,type=creeper]`
- `kill @e[type=arrow,nbt={inBlockState:{Name:"minecraft:target"}}]`

## `list`

- Permission: Any
- Requires cheats: No

### Official

- `/list`

### Wiki (Bedrock Syntax section)

- `list`

## `locate`

- Permission: Game Directors
- Requires cheats: Yes

### Official


### Wiki (Bedrock Syntax section)

- `/locate structure [useNewChunksOnly: Boolean]`
- `/locate biome`

## `loot`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/loot spawn <position: x y z> loot <loot_table: string> [<tool>|mainhand|offhand: Tool]`
- `/loot spawn <position: x y z> kill <entity: target> [<tool>|mainhand|offhand: Tool]`
- `/loot spawn <position: x y z> mine <TargetBlockPosition: x y z> [<tool>|mainhand|offhand: Tool]`
- `/loot give <players: target> loot <loot_table: string> [<tool>|mainhand|offhand: Tool]`
- `/loot give <players: target> kill <entity: target> [<tool>|mainhand|offhand: Tool]`
- `/loot give <players: target> mine <TargetBlockPosition: x y z> [<tool>|mainhand|offhand: Tool]`
- `/loot insert <position: x y z> loot <loot_table: string> [<tool>|mainhand|offhand: Tool]`
- `/loot insert <position: x y z> kill <entity: target> [<tool>|mainhand|offhand: Tool]`
- `/loot insert <position: x y z> mine <TargetBlockPosition: x y z> [<tool>|mainhand|offhand: Tool]`
- `/loot replace entity <entity: target> <slotType: EntityEquipmentSlot> <slotId: int> <count: int> loot <loot_table: string> [<tool>|mainhand|offhand: Tool]`
- `/loot replace entity <entity: target> <slotType: EntityEquipmentSlot> <slotId: int> loot <loot_table: string> [<tool>|mainhand|offhand: Tool]`
- `/loot replace entity <entity: target> <slotType: EntityEquipmentSlot> <slotId: int> <count: int> kill <entity: target> [<tool>|mainhand|offhand: Tool]`
- `/loot replace entity <entity: target> <slotType: EntityEquipmentSlot> <slotId: int> kill <entity: target> [<tool>|mainhand|offhand: Tool]`
- `/loot replace entity <entity: target> <slotType: EntityEquipmentSlot> <slotId: int> <count: int> mine <TargetBlockPosition: x y z> [<tool>|mainhand|offhand: Tool]`
- `/loot replace entity <entity: target> <slotType: EntityEquipmentSlot> <slotId: int> mine <TargetBlockPosition: x y z> [<tool>|mainhand|offhand: Tool]`
- `/loot replace block <position: x y z> slot.container <slotId: int> <count: int> loot <loot_table: string> [<tool>|mainhand|offhand: Tool]`
- `/loot replace block <position: x y z> slot.container <slotId: int> loot <loot_table: string> [<tool>|mainhand|offhand: Tool]`
- `/loot replace block <position: x y z> slot.container <slotId: int> <count: int> kill <entity: target> [<tool>|mainhand|offhand: Tool]`
- `/loot replace block <position: x y z> slot.container <slotId: int> kill <entity: target> [<tool>|mainhand|offhand: Tool]`
- `/loot replace block <position: x y z> slot.container <slotId: int> <count: int> mine <TargetBlockPosition: x y z> [<tool>|mainhand|offhand: Tool]`

### Wiki (Bedrock Syntax section)

- `loot`
- `loot ["|mainhand|offhand": string]`
- `loot_table: string`
- `loot_table: string`

## `me`

- Permission: Any
- Requires cheats: No

### Official

- `/me <message: message>`

## `mobevent`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/mobevent <event: MobEvent> [value: Boolean]`

## `music`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/music queue <trackName: string> [volume: float] [fadeSeconds: float] [repeatMode: MusicRepeatMode]`
- `/music play <trackName: string> [volume: float] [fadeSeconds: float] [repeatMode: MusicRepeatMode]`
- `/music stop [fadeSeconds: float]`
- `/music volume <volume: float>`

## `op`

- Permission: Admin
- Requires cheats: No

### Official

- `/op <player: target>`

### Wiki (Bedrock Syntax section)

- `op`

## `packstack`

- Permission: Any
- Requires cheats: No

### Official


## `particle`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/particle <effect: string> [position: x y z]`

### Wiki (Bedrock Syntax section)

- `particle [position: x y z]`

## `permission`

- Permission: Owner
- Requires cheats: Yes

### Official

- `/permission <action: PermissionsAction>`

## `place`

- Permission: Admin
- Requires cheats: Yes

### Official


## `playanimation`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/playanimation <entity: target> <animation: string> [next_state: string] [blend_out_time: float] [stop_expression: string] [controller: string]`

## `playsound`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/playsound <sound: string> [player: target] [position: x y z] [volume: float] [pitch: float] [minimumVolume: float]`

### Wiki (Bedrock Syntax section)

- `playsound [player: target] [position: x y z] [volume: float] [pitch: float] [minimumVolume: float]`

## `project`

- Permission: Game Directors
- Requires cheats: No

### Official


## `recipe`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/recipe give <player: target> <recipe: UnlockableRecipeValues>`
- `/recipe take <player: target> <recipe: UnlockableRecipeValues>`

## `reload`

- Permission: Admin
- Requires cheats: Yes

### Official

- `/reload [all: reload_all]`

## `reloadconfig`

- Permission: Owner
- Requires cheats: Yes

### Official


## `reloadpacketlimitconfig`

- Permission: Owner
- Requires cheats: Yes

### Official


## `replaceitem`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/replaceitem block <position: x y z> slot.container <slotId: int> <itemName: Item> [amount: int] [data: int] [components: json]`
- `/replaceitem entity <target: target> <slotType: EntityEquipmentSlot> <slotId: int> <itemName: Item> [amount: int] [data: int] [components: json]`
- `/replaceitem block <position: x y z> slot.container <slotId: int> <oldItemHandling: ReplaceMode> <itemName: Item> [amount: int] [data: int] [components: json]`
- `/replaceitem entity <target: target> <slotType: EntityEquipmentSlot> <slotId: int> <oldItemHandling: ReplaceMode> <itemName: Item> [amount: int] [data: int] [components: json]`

## `ride`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/ride <riders: target> start_riding <ride: target> [teleportRules: TeleportRules] [howToFill: FillType]`
- `/ride <riders: target> stop_riding`
- `/ride <rides: target> evict_riders`
- `/ride <rides: target> summon_rider <entityType: EntityType> [spawnEvent: string] [nameTag: string]`
- `/ride <riders: target> summon_ride <entityType: EntityType> [rideRules: RideRules] [spawnEvent: string] [nameTag: string]`

### Wiki (Bedrock Syntax section)

- `ride start_riding [teleportRules: TeleportRules] [howToFill: FillType]`
- `ride stop_riding`
- `ride evict_riders`
- `ride summon_rider [spawnEvent: string] [nameTag: string]`
- `ride summon_ride [rideRules: RideRules] [spawnEvent: string] [nameTag: string]`
- `riders: target`
- `rides: target`
- `ride: target`
- `ride: target`
- `ride: target`
- `riders: target`
- `ride: target`
- `riders: target`
- `riders: target`
- `ride: target`
- `riders: target`
- `riders: target`
- `rides: target`
- `rides: target`
- `riders: target`
- `/ride @s mount @e[type=minecraft:skeleton_horse,limit=1]`
- `/ride @s mount @n[type=arrow]`
- `/ride @a[tag=A] summon_ride arrow`
- `/ride @a[tag=A] summon_ride creeper reassign_rides minecraft:become_charged`
- `/ride @e[type=chicken] summon_rider zombie minecraft:as_baby_jockey`

## `save`

- Permission: Owner
- Requires cheats: Yes

### Official

- `/save <mode: SaveMode>`

## `say`

- Permission: Game Directors
- Requires cheats: No

### Official

- `/say <message: message>`

### Wiki (Bedrock Syntax section)

- `say`

## `schedule`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/schedule delay add <function: filepath> <time: int> [DelayMode: DelayMode]`
- `/schedule delay add <function: filepath> <time: time> [DelayMode: DelayMode]`
- `/schedule delay clear <function: filepath>`
- `/schedule clear <function: filepath>`
- `/schedule on_area_loaded add <from: x y z> <to: x y z> <function: filepath>`
- `/schedule on_area_loaded add circle <center: x y z> <radius: int> <function: filepath>`
- `/schedule on_area_loaded add tickingarea <name: string> <function: filepath>`
- `/schedule on_area_loaded clear tickingarea <name: string> [function: filepath]`
- `/schedule on_area_loaded clear function <function: filepath>`

### Wiki (Bedrock Syntax section)

- `schedule on_area_loaded add`
- `schedule on_area_loaded add circle`
- `schedule on_area_loaded add tickingarea`
- `schedule delay add [append{{!}}replace]`
- `schedule delay clear`
- `schedule clear`
- `schedule on_area_loaded clear function`
- `schedule on_area_loaded clear tickingarea [function: filepath]`

## `scoreboard`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/scoreboard objectives add <objective: ScoreboardObjectives> dummy [displayName: string]`
- `/scoreboard objectives remove <objective: ScoreboardObjectives>`
- `/scoreboard objectives list`
- `/scoreboard objectives setdisplay <displaySlot: ScoreboardDisplaySlotSortable> [objective: ScoreboardObjectives] [sortOrder: ScoreboardSortOrder]`
- `/scoreboard objectives setdisplay belowname [objective: ScoreboardObjectives]`
- `/scoreboard players list [playername: targets]`
- `/scoreboard players reset <player: targets> [objective: ScoreboardObjectives]`
- `/scoreboard players test <player: targets> <objective: ScoreboardObjectives> <min: wildcard int> [max: wildcard int]`
- `/scoreboard players random <player: targets> <objective: ScoreboardObjectives> <min: int> <max: int>`
- `/scoreboard players <action: ScoreboardPlayersNumAction> <player: targets> <objective: ScoreboardObjectives> <count: int>`
- `/scoreboard players operation <targetName: targets> <targetObjective: ScoreboardObjectives> <operation: operator> <selector: targets> <objective: ScoreboardObjectives>`

## `script`

- Permission: Admin
- Requires cheats: Yes

### Official

- `/script debugger listen <port: int>`
- `/script debugger connect [host: string] [port: int]`
- `/script debugger close`

## `scriptevent`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/scriptevent <messageId: string> <message: message>`

## `sendshowstoreoffer`

- Permission: Owner
- Requires cheats: Yes

### Official

- `/sendshowstoreoffer <player: target> <redirectType: RedirectLocation> <offerId: string>`
- `/sendshowstoreoffer <player: target> server`

## `setblock`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/setblock <position: x y z> <tileName: Block> <blockStates: block properties> [oldBlockHandling: SetBlockMode]`

### Wiki (Bedrock Syntax section)

- `setblock [destroy|keep|replace]`
- `setblock [destroy|keep|replace]`

## `setmaxplayers`

- Permission: Host
- Requires cheats: Yes

### Official

- `/setmaxplayers <maxPlayers: int>`

## `setworldspawn`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/setworldspawn [spawnPoint: x y z]`

### Wiki (Bedrock Syntax section)

- `setworldspawn [spawnPoint: x y z]`

## `spawnpoint`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/spawnpoint [player: target] [spawnPos: x y z]`

### Wiki (Bedrock Syntax section)

- `spawnpoint [player: target] [spawnPos: x y z]`

## `spreadplayers`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/spreadplayers <x: rotation> <z: rotation> <spreadDistance: float> <maxRange: float> <victim: target> [maxHeight: rotation]`

### Wiki (Bedrock Syntax section)

- `spreadplayers [maxHeight: value]`

## `stop`

- Permission: Owner
- Requires cheats: Yes

### Official

- `/stop`

## `stopsound`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/stopsound <player: target> [sound: string]`

### Wiki (Bedrock Syntax section)

- `stopsound [sound: string]`

## `structure`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/structure save <name: string> <from: x y z> <to: x y z> [saveMode: StructureSaveMode]`
- `/structure save <name: string> <from: x y z> <to: x y z> [includeEntities: Boolean] [saveMode: StructureSaveMode] [includeBlocks: Boolean]`
- `/structure delete <name: string>`

## `summon`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/summon <entityType: EntityType> [spawnPos: x y z] [yRot: rotation] [xRot: rotation] [spawnEvent: EntityEvents] [nameTag: string]`

## `tag`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/tag <entity: targets> <action: TagChangeAction> <name: TagValues>`
- `/tag <entity: targets> list`

### Wiki (Bedrock Syntax section)

- `tag add`
- `tag remove`
- `tag list`

## `teleport`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/teleport <destination: x y z> [checkForBlocks: Boolean]`
- `/teleport <destination: x y z> [yRot: rotation] [xRot: rotation] [checkForBlocks: Boolean]`
- `/teleport <destination: x y z> facing <lookAtPosition: x y z> [checkForBlocks: Boolean]`
- `/teleport <destination: x y z> facing <lookAtEntity: target> [checkForBlocks: Boolean]`
- `/teleport <victim: target> <destination: x y z> [yRot: rotation] [xRot: rotation] [checkForBlocks: Boolean]`
- `/teleport <victim: target> <destination: x y z> [checkForBlocks: Boolean]`
- `/teleport <victim: target> <destination: x y z> facing <lookAtPosition: x y z> [checkForBlocks: Boolean]`
- `/teleport <victim: target> <destination: x y z> facing <lookAtEntity: target> [checkForBlocks: Boolean]`
- `/teleport <destination: target>`
- `/teleport <victim: target> <destination: target> [checkForBlocks: Boolean]`

### Wiki (Bedrock Syntax section)

- `teleport`
- `teleport [checkForBlocks: Boolean]`
- `teleport [checkForBlocks: Boolean]`
- `teleport [checkForBlocks: Boolean]`
- `teleport [yRot: value] [xRot: value] [checkForBlocks: Boolean]`
- `teleport facing [checkForBlocks: Boolean]`
- `teleport facing [checkForBlocks: Boolean]`
- `teleport [yRot: value] [xRot: value] [checkForBlocks: Boolean]`
- `teleport facing [checkForBlocks: Boolean]`
- `teleport facing [checkForBlocks: Boolean]`
- `teleport Alice`
- `teleport @a @s`
- `teleport 100 ~3 100`
- `teleport ^ ^ ^1`

## `tell`

- Permission: Any
- Requires cheats: No

### Official

- `/tell <target: target> <message: message>`

## `tellraw`

- Permission: Game Directors
- Requires cheats: No

### Official

- `/tellraw <target: target> <raw json message: json>`

## `testfor`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/testfor <victim: target>`

## `testforblock`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/testforblock <position: x y z> <tileName: Block> [blockStates: block properties]`

## `testforblocks`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/testforblocks <begin: x y z> <end: x y z> <destination: x y z> [mode: TestForBlocksMode]`

## `tickingarea`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/tickingarea add <from: x y z> <to: x y z> [name: string] [preload: Boolean]`
- `/tickingarea add circle <center: x y z> <radius: int> [name: string] [preload: Boolean]`
- `/tickingarea remove <position: x y z>`
- `/tickingarea remove <name: string>`
- `/tickingarea remove_all`
- `/tickingarea list [all-dimensions: AllDimensions]`
- `/tickingarea preload <position: x y z> [preload: Boolean]`
- `/tickingarea preload <name: string> [preload: Boolean]`

## `time`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/time add <amount: int>`
- `/time set <amount: int>`
- `/time set <time: TimeSpec>`
- `/time query <time: TimeQuery>`

### Wiki (Bedrock Syntax section)

- `time add`
- `time>`
- `time query`
- `time set`
- `time set <time: TimeSpec>`

## `title`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/title <player: target> clear`
- `/title <player: target> reset`
- `/title <player: target> <titleLocation: TitleSet> <titleText: message>`
- `/title <player: target> times <fadeIn: int> <stay: int> <fadeOut: int>`

## `titleraw`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/titleraw <player: target> clear`
- `/titleraw <player: target> reset`
- `/titleraw <player: target> <titleLocation: TitleRawSet> <raw json titleText: json>`
- `/titleraw <player: target> times <fadeIn: int> <stay: int> <fadeOut: int>`

## `toggledownfall`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/toggledownfall`

## `transfer`

- Permission: Owner
- Requires cheats: Yes

### Official


### Wiki (Bedrock Syntax section)

- `transfer`

## `weather`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/weather <type: WeatherType> [duration: int]`
- `/weather query`

### Wiki (Bedrock Syntax section)

- `weather [duration: int]`
- `weather query`

## `wsserver`

- Permission: Admin
- Requires cheats: Yes

### Official

- `/wsserver <serverUri: text>`

## `xp`

- Permission: Game Directors
- Requires cheats: Yes

### Official

- `/xp <amount: int> [player: target]`

