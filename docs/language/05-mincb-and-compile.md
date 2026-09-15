# MINCB binary and compilation

`minc build` writes **`dist/<project>.mincb`**: one edition-neutral placement file. It is **not** a `.jar` and not a Minecraft world. A later placer (same CLI or a companion) turns it into:

- Java: datapack + optional structure NBT
- Bedrock: behavior pack functions + `.mcstructure` / world chunk edits

The binary is the **source of truth** for “this chunk’s command blocks, fixed blocks, and chests.” Functions can be embedded or referenced.

## Pipeline (do not implement in this spec drop)

```
*.mcs / .chain.mcs / controller / place
        │
        ▼
   parse (Java-like)     edition + game_version from minc.toml
        │
        ▼
   type check            private fields, Seq vs int, @JavaOnly
        │
        ▼
   lower to Command IR   one IR opcode per game command, edition-specific print
        │
        ├─ function bodies   (.mcfunction / datapack)
        ├─ CB graph           layout → (x,y,z,facing,mode,…)
        └─ world ops          setblock / container NBT
        ▼
   MINCB write
```

Command IR printing **must** use the grammars in `docs/minecraft-commands/`. Bedrock `execute` is depth-first; Java is breadth-first — different walkers.

## File format (v1)

Little-endian. Integers are two’s complement.

```
Offset  Size  Field
0       4     magic  4D 49 4E 43   "MINC"
4       2     format_major        1
6       2     format_minor        0
8       1     edition              1 = java, 2 = bedrock
9       1     flags                bit0 = has_functions, bit1 = has_cb, bit2 = has_world
10      2     reserved
12      4     game_version_off    offset of UTF-8 version string ("1.21.70")
16      4     pack_off            UTF-8 pack id
20      12    origin               i32 x, y, z (world)
32      4     score_revision
36      4     section_count
40      …     section table
```

Each section:

```
u32 id        (SYMB, OBJT, TAGS, FUNC, CHAIN, CBLK, WBLK, CONT, LINK, META)
u32 offset
u32 size
u32 count
```

### `SYMB`

Array of `{ u16 id, u8 kind, string qualified }`.

- `kind` 0 = objective, 1 = tag, 2 = fake player, 3 = function path, 4 = chain name

### `OBJT`

Objectives to `add` on install: `{ u16 symb, u8 dummy_only, string display? }`. Bedrock: always dummy. Java may later add criteria ids (v2).

### `FUNC`

If `emit.functions`:

```
u16 symb_id
u32 byte_len
u8[] utf8     // 无前导 / 的 mcfunction 文本，\n 分隔
```

Bedrock `tick.json` is not inside FUNC; it is `META.tick_values[]` of symb ids.

### `CHAIN`

One record per `.chain.mcs`:

```
u16 name_symb
u8  layout          0 linear 1 stack 2 snake 3 box 4 points
u8  facing          0..5  (down up north south west east — same as MC facing)
i32 ox, oy, oz      first block (absolute world)
u16 length          number of CBLK entries that belong to this chain (contiguous slice)
u8  clock           1 if Repeat clock
u8  pack_mode       0 one-cb-per-command  1 single CB + function
```

### `CBLK` (command blocks)

```
i32 x, y, z
u8  facing
u8  mode            0 impulse 1 chain 2 repeat
u8  flags           bit0 conditional, bit1 always_active, bit2 last
u32 delay_ticks
u32 cmd_len
u8[] utf8 command    // 无斜杠或有斜杠：CB 里通常带 / ，函数里不带。
                    // 规范：二进制里 **不** 存前导 / ，安装器按宿主补。
```

This is the “区块内指令怎么存放” section. Chunk is implied by `x>>4, z>>4`.

### `WBLK` (fixed blocks)

```
i32 x, y, z
u16 block_id        // interned string in SYMB or a local palette
u32 state_bits      // packed states or NBT offset
```

Palette header: list of `minecraft:gold_block` etc.

### `CONT` (chests and other containers)

```
i32 x, y, z
u16 block_id        // chest, barrel, …
u8  facing
u16 slot_count
repeat slot_count:
    u8 slot
    u16 item_id
    u16 count
    i32 data            // Bedrock aux; Java = 0
    u32 nbt_len         // SNBT or Bedrock component JSON
    u8[] nbt
```

### `LINK`

```
u16 from_chain
u16 from_label
u16 to_chain
u8  kind            0 function 1 redstone_block
```

### `META`

JSON (UTF-8) for leftovers: `tick_values`, `ticking_areas[]`, `gamerules[]`, `host_tick`, `dimension`. Kept as JSON so v1 can grow without a new major.

## Install order (placer)

1. Add objectives (`OBJT`)
2. `forceload` / `tickingarea` (`META`)
3. `WBLK` then `CONT` then `CBLK` (command blocks last so they are not overwritten)
4. Copy `FUNC` into the pack folder
5. Write `tick.json` / datapack tick tag
6. Do **not** run gameplay chains until load/`@OnLoad`

## Debugging

```
minc inspect dist/metro-escape.mincb
```

Prints edition, version, origin, chain bounding boxes, first/last CB command, chest slots.

```
minc dump --commands
```

Prints the exact strings that will sit in CBs / functions, for diffing against `docs/minecraft-commands`.

## Size and limits

- Warn if a chain `length > 100` (hard to see in Creative)
- Error if a packed function would exceed `limits.function_command_limit`
- Error if Bedrock ticking areas would exceed 10
- Error if `box` / `snake` exceeds `bound`

## Stability

MINCB `format_major` bump = placer must upgrade. `score_revision` bump = wipe dummy scores (document in changelog). Same `SYMB` ids for unchanged fields.

## What is not in the binary

- Source `.mcs`
- Comments
- Java-only IR when `edition = bedrock` (never written)
- Player data (inventories of players)

---

## Implementation order (when coding starts; not now)

1. `minc.toml` parse + `minc new`
2. Classes + private `int`/`boolean` + `minc check`
3. `foreach` / `if` / `as` lowering to Bedrock execute (then Java)
4. Chain flatten + `stack`/`linear` layout
5. Controller coordinates
6. MINCB write/read + `inspect`
7. `place` / `chest`
8. Placer into a real world (last)

Until then, this directory is the contract.
