//! Place MINCB world ops into Java structure NBT / Bedrock `.mcstructure`.
//!
//! Command blocks are written last so scenery cannot overwrite them.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::config::Edition;
use crate::mincb::{facing_name, MincbImage};
use crate::nbt::{self, Endian, Value};

/// Bedrock block version integer stored on each palette entry.
const BEDROCK_BLOCK_VERSION: i32 = 18168865;

#[derive(Clone)]
struct Cell {
    block: String,
    facing: Option<u8>,
    conditional: bool,
    mode: Option<u8>,
    command: Option<String>,
    auto: bool,
    delay: u32,
    slots: Vec<(u8, String, u16, i32, String)>,
}

pub fn write_structure_files(
    out_dir: &Path,
    image: &MincbImage,
) -> Result<BTreeMap<PathBuf, Vec<u8>>, String> {
    let root = build_volume(image);
    let mut written = BTreeMap::new();
    match image.edition {
        Edition::Bedrock => {
            let bytes = encode_mcstructure(image, &root);
            let file = out_dir.join(format!("{}.mcstructure", sanitize(&image.pack)));
            if let Some(parent) = file.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(&file, &bytes).map_err(|e| e.to_string())?;
            written.insert(file, bytes);
        }
        Edition::Java => {
            let raw = encode_java_nbt(image, &root);
            let bytes = nbt::gzip(&raw)?;
            let ns = image.pack.as_str();
            let file = out_dir.join(format!(
                "data/{ns}/structures/{}.nbt",
                sanitize(&image.pack)
            ));
            if let Some(parent) = file.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(&file, &bytes).map_err(|e| e.to_string())?;
            written.insert(file, bytes);
        }
    }
    Ok(written)
}

struct Volume {
    origin: [i32; 3],
    size: [i32; 3],
    cells: BTreeMap<[i32; 3], Cell>,
}

fn build_volume(image: &MincbImage) -> Volume {
    let mut cells: BTreeMap<[i32; 3], Cell> = BTreeMap::new();
    for b in &image.world_blocks {
        cells.insert(
            [b.x, b.y, b.z],
            Cell {
                block: mc_id(&b.block),
                facing: None,
                conditional: false,
                mode: None,
                command: None,
                auto: false,
                delay: 0,
                slots: Vec::new(),
            },
        );
    }
    for c in &image.containers {
        cells.insert(
            [c.x, c.y, c.z],
            Cell {
                block: mc_id(&c.block),
                facing: Some(c.facing),
                conditional: false,
                mode: None,
                command: None,
                auto: false,
                delay: 0,
                slots: c
                    .slots
                    .iter()
                    .map(|s| (s.slot, mc_id(&s.item), s.count, s.data, s.nbt.clone()))
                    .collect(),
            },
        );
    }
    for b in &image.command_blocks {
        cells.insert(
            [b.x, b.y, b.z],
            Cell {
                block: cb_block(b.mode),
                facing: Some(b.facing),
                conditional: (b.flags & 1) != 0,
                mode: Some(b.mode),
                command: Some(b.command.clone()),
                auto: (b.flags & 2) != 0,
                delay: b.delay_ticks,
                slots: Vec::new(),
            },
        );
    }
    if cells.is_empty() {
        return Volume {
            origin: image.origin,
            size: [1, 1, 1],
            cells,
        };
    }
    let mut min = [i32::MAX; 3];
    let mut max = [i32::MIN; 3];
    for [x, y, z] in cells.keys() {
        min = [min[0].min(*x), min[1].min(*y), min[2].min(*z)];
        max = [max[0].max(*x), max[1].max(*y), max[2].max(*z)];
    }
    Volume {
        origin: min,
        size: [
            max[0] - min[0] + 1,
            max[1] - min[1] + 1,
            max[2] - min[2] + 1,
        ],
        cells,
    }
}

fn encode_java_nbt(image: &MincbImage, vol: &Volume) -> Vec<u8> {
    let mut palette: Vec<(String, Vec<(String, String)>)> = Vec::new();
    let mut blocks = Vec::new();
    for (pos, cell) in &vol.cells {
        let props = java_props(cell);
        let key = (cell.block.clone(), props.clone());
        let state = if let Some(i) = palette.iter().position(|p| *p == key) {
            i as i32
        } else {
            palette.push(key);
            (palette.len() - 1) as i32
        };
        let local = [
            Value::Int(pos[0] - vol.origin[0]),
            Value::Int(pos[1] - vol.origin[1]),
            Value::Int(pos[2] - vol.origin[2]),
        ];
        let mut rec = vec![
            ("pos".into(), Value::List(local.to_vec())),
            ("state".into(), Value::Int(state)),
        ];
        if let Some(nbt) = java_block_nbt(cell) {
            rec.push(("nbt".into(), nbt));
        }
        blocks.push(Value::Compound(rec));
    }
    let pal_vals: Vec<Value> = palette
        .into_iter()
        .map(|(name, props)| {
            let mut pairs = vec![("Name".into(), Value::String(name))];
            if !props.is_empty() {
                pairs.push((
                    "Properties".into(),
                    Value::Compound(
                        props
                            .into_iter()
                            .map(|(k, v)| (k, Value::String(v)))
                            .collect(),
                    ),
                ));
            }
            Value::Compound(pairs)
        })
        .collect();
    let root = Value::Compound(vec![
        ("DataVersion".into(), Value::Int(5022)),
        ("size".into(), Value::int_list(vol.size)),
        ("palette".into(), Value::List(pal_vals)),
        ("blocks".into(), Value::List(blocks)),
        ("entities".into(), Value::List(Vec::new())),
        (
            "author".into(),
            Value::String(format!("mincscript:{}", image.pack)),
        ),
    ]);
    nbt::encode_named("", &root, Endian::Big)
}

fn encode_mcstructure(_image: &MincbImage, vol: &Volume) -> Vec<u8> {
    let sx = vol.size[0] as usize;
    let sy = vol.size[1] as usize;
    let sz = vol.size[2] as usize;
    let mut palette: Vec<Value> = Vec::new();
    let mut indices = vec![-1i32; sx * sy * sz];
    let mut pos_data: Vec<(String, Value)> = Vec::new();
    for (pos, cell) in &vol.cells {
        let pal = bedrock_palette_entry(cell);
        let idx = if let Some(i) = palette.iter().position(|p| nbt_eq(p, &pal)) {
            i as i32
        } else {
            palette.push(pal);
            (palette.len() - 1) as i32
        };
        let lx = (pos[0] - vol.origin[0]) as usize;
        let ly = (pos[1] - vol.origin[1]) as usize;
        let lz = (pos[2] - vol.origin[2]) as usize;
        let linear = lx * sy * sz + ly * sz + lz;
        if linear < indices.len() {
            indices[linear] = idx;
        }
        if let Some(ent) = bedrock_block_entity(cell) {
            pos_data.push((linear.to_string(), ent));
        }
    }
    let layer1 = vec![-1i32; indices.len()];
    let structure = Value::Compound(vec![
        (
            "block_indices".into(),
            Value::List(vec![
                Value::List(indices.into_iter().map(Value::Int).collect()),
                Value::List(layer1.into_iter().map(Value::Int).collect()),
            ]),
        ),
        (
            "palette".into(),
            Value::Compound(vec![(
                "default".into(),
                Value::Compound(vec![
                    ("block_palette".into(), Value::List(palette)),
                    ("block_position_data".into(), Value::Compound(pos_data)),
                ]),
            )]),
        ),
        ("entities".into(), Value::List(Vec::new())),
    ]);
    let root = Value::Compound(vec![
        ("format_version".into(), Value::Int(1)),
        ("size".into(), Value::int_list(vol.size)),
        ("structure_world_origin".into(), Value::int_list(vol.origin)),
        ("structure".into(), structure),
    ]);
    nbt::encode_named("", &root, Endian::Little)
}

fn java_props(cell: &Cell) -> Vec<(String, String)> {
    let mut props = Vec::new();
    if let Some(f) = cell.facing {
        if cell.mode.is_some() || !cell.slots.is_empty() {
            props.push(("facing".into(), facing_name(f).to_string()));
        }
    }
    if cell.mode.is_some() {
        props.push((
            "conditional".into(),
            if cell.conditional {
                "true".into()
            } else {
                "false".into()
            },
        ));
    }
    props
}

fn java_block_nbt(cell: &Cell) -> Option<Value> {
    if let Some(cmd) = &cell.command {
        return Some(Value::Compound(vec![
            ("id".into(), Value::String("minecraft:command_block".into())),
            ("Command".into(), Value::String(cmd.clone())),
            ("auto".into(), Value::Byte(i8::from(cell.auto))),
            (
                "conditionMet".into(),
                Value::Byte(i8::from(cell.conditional)),
            ),
            ("TrackOutput".into(), Value::Byte(0)),
            ("UpdateLastExecution".into(), Value::Byte(1)),
        ]));
    }
    if cell.slots.is_empty() {
        return None;
    }
    let items: Vec<Value> = cell
        .slots
        .iter()
        .map(|(slot, item, count, _, _)| {
            Value::Compound(vec![
                ("Slot".into(), Value::Byte(*slot as i8)),
                ("id".into(), Value::String(item.clone())),
                ("count".into(), Value::Int(*count as i32)),
            ])
        })
        .collect();
    Some(Value::Compound(vec![("Items".into(), Value::List(items))]))
}

fn bedrock_palette_entry(cell: &Cell) -> Value {
    let mut states = Vec::new();
    if let Some(f) = cell.facing {
        states.push(("facing_direction".into(), Value::Int(i32::from(f))));
    }
    if cell.mode.is_some() {
        states.push((
            "conditional_bit".into(),
            Value::Byte(i8::from(cell.conditional)),
        ));
    }
    Value::Compound(vec![
        ("name".into(), Value::String(cell.block.clone())),
        ("states".into(), Value::Compound(states)),
        ("version".into(), Value::Int(BEDROCK_BLOCK_VERSION)),
    ])
}

fn bedrock_block_entity(cell: &Cell) -> Option<Value> {
    if let Some(cmd) = &cell.command {
        let mode = i32::from(cell.mode.unwrap_or(0));
        let body = Value::Compound(vec![
            ("id".into(), Value::String("CommandBlock".into())),
            ("Command".into(), Value::String(cmd.clone())),
            ("auto".into(), Value::Byte(i8::from(cell.auto))),
            (
                "conditionalMode".into(),
                Value::Byte(i8::from(cell.conditional)),
            ),
            ("LPCommandMode".into(), Value::Int(mode)),
            ("TickDelay".into(), Value::Int(cell.delay as i32)),
            ("TrackOutput".into(), Value::Byte(0)),
            ("ExecuteOnFirstTick".into(), Value::Byte(1)),
            ("Version".into(), Value::Int(36)),
        ]);
        return Some(Value::Compound(vec![("block_entity_data".into(), body)]));
    }
    if cell.slots.is_empty() {
        return None;
    }
    let items: Vec<Value> = cell
        .slots
        .iter()
        .map(|(slot, item, count, data, _)| {
            Value::Compound(vec![
                ("Slot".into(), Value::Byte(*slot as i8)),
                ("Name".into(), Value::String(item.clone())),
                ("Count".into(), Value::Byte(*count as i8)),
                ("Damage".into(), Value::Short(*data as i16)),
            ])
        })
        .collect();
    Some(Value::Compound(vec![(
        "block_entity_data".into(),
        Value::Compound(vec![("Items".into(), Value::List(items))]),
    )]))
}

fn cb_block(mode: u8) -> String {
    match mode {
        2 => "minecraft:repeating_command_block".into(),
        1 => "minecraft:chain_command_block".into(),
        _ => "minecraft:command_block".into(),
    }
}

fn mc_id(name: &str) -> String {
    if name.contains(':') {
        name.to_string()
    } else {
        format!("minecraft:{name}")
    }
}

fn sanitize(s: &str) -> String {
    s.replace('.', "_")
}

fn nbt_eq(a: &Value, b: &Value) -> bool {
    format!("{a:?}") == format!("{b:?}")
}
