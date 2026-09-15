# Bedrock inventory: `/give` `/clear` `/replaceitem` `/loot`

## `/give` (official)

```
/give <player: target> <itemName: Item> [amount: int] [data: int] [components: json]
```

Wiki agrees. `amount` 1–32767, `data` 0–32767. Components JSON subset only.

## `/clear`

Official page in the 82-command set. Typical BE:

```
clear [player] [item] [data] [maxCount]
```

`data` `-1` often means “any aux”. Confirm with `/help clear` when implementing; official markdown is the required overload list in `generated-syntax.md`.

## `/replaceitem` (Bedrock-only; Java uses `/item`)

```
replaceitem (block|entity) … <slotId> <itemName> [amount] [data] [components]
```

Slots use `slot.weapon.mainhand`, `slot.armor.chest`, `slot.hotbar.0`, `slot.inventory.0`, `slot.container.0`, etc.

## `/loot`

Present on both editions with **different** destinations. Official Bedrock `loot.md` overloads are in `generated-syntax.md`. Do not accept Java `loot replace entity @s hotbar.0 kill @e[limit=1]`.

## `/enchant`

Enchants the **held** item (`/enchant <player> <id> [level]`). Cannot target arbitrary slots (use `/replaceitem` after).
