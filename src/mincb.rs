//! MINCB (Minc Command Binary) — `docs/language/05-mincb-and-compile.md`.

use crate::config::Edition;
use crate::extract::{BinaryInfo, SymbolKind};
use crate::layout::CbInstance;

const MAGIC: &[u8; 4] = b"MINC";
const FORMAT_MAJOR: u16 = 1;
const FORMAT_MINOR: u16 = 0;

const SID_SYMB: u32 = u32::from_le_bytes(*b"SYMB");
const SID_OBJT: u32 = u32::from_le_bytes(*b"OBJT");
const SID_TAGS: u32 = u32::from_le_bytes(*b"TAGS");
const SID_FUNC: u32 = u32::from_le_bytes(*b"FUNC");
const SID_CHAIN: u32 = u32::from_le_bytes(*b"CHAI");
const SID_CBLK: u32 = u32::from_le_bytes(*b"CBLK");
const SID_WBLK: u32 = u32::from_le_bytes(*b"WBLK");
const SID_CONT: u32 = u32::from_le_bytes(*b"CONT");
const SID_LINK: u32 = u32::from_le_bytes(*b"LINK");
const SID_META: u32 = u32::from_le_bytes(*b"META");

/// Fully lowered image written by `minc build`.
#[derive(Debug, Clone)]
pub struct MincbImage {
    pub edition: Edition,
    pub game_version: String,
    pub pack: String,
    pub origin: [i32; 3],
    pub score_revision: u32,
    pub symbols: Vec<SymbRec>,
    pub objectives: Vec<ObjtRec>,
    pub functions: Vec<FuncRec>,
    pub chains: Vec<ChainRec>,
    pub command_blocks: Vec<CblkRec>,
    pub world_blocks: Vec<WblkRec>,
    pub containers: Vec<ContRec>,
    pub links: Vec<LinkRec>,
    pub tags: Vec<u16>,
    pub tick_values: Vec<u16>,
    pub meta_json: String,
}

#[derive(Debug, Clone)]
pub struct SymbRec {
    pub id: u16,
    pub kind: u8,
    pub short: String,
    pub qualified: String,
}

#[derive(Debug, Clone)]
pub struct ObjtRec {
    pub symb: u16,
    pub dummy_only: u8,
    pub display: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FuncRec {
    pub symb: u16,
    pub path: String,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct ChainRec {
    pub name_symb: u16,
    pub layout: u8,
    pub facing: u8,
    pub origin: [i32; 3],
    pub length: u16,
    pub clock: u8,
    pub pack_mode: u8,
}

#[derive(Debug, Clone)]
pub struct CblkRec {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub facing: u8,
    pub mode: u8,
    pub flags: u8,
    pub delay_ticks: u32,
    pub command: String,
}

#[derive(Debug, Clone)]
pub struct WblkRec {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub block: String,
}

#[derive(Debug, Clone)]
pub struct ContRec {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub block: String,
    pub facing: u8,
    pub slots: Vec<ContSlot>,
}

#[derive(Debug, Clone)]
pub struct ContSlot {
    pub slot: u8,
    pub item: String,
    pub count: u16,
    pub data: i32,
    pub nbt: String,
}

#[derive(Debug, Clone)]
pub struct LinkRec {
    pub from_chain: u16,
    pub from_label: u16,
    pub to_chain: u16,
    pub kind: u8,
}

/// Encode extracted program facts without lowered commands (parser / unit tests).
pub fn encode(info: &BinaryInfo, edition: u8) -> Vec<u8> {
    let edition = match edition {
        1 => Edition::Java,
        _ => Edition::Bedrock,
    };
    encode_image(&image_from_extract(info, edition, "unspecified"))
}

pub fn image_from_extract(info: &BinaryInfo, edition: Edition, game_version: &str) -> MincbImage {
    let origin = info.origin.unwrap_or([0, 64, 0]);
    let origin = [origin[0] as i32, origin[1] as i32, origin[2] as i32];
    let mut symbols = Vec::new();
    for (i, sym) in info.symbols.iter().enumerate() {
        symbols.push(SymbRec {
            id: i as u16 + 1,
            kind: sym.kind as u8,
            short: sym.id.clone(),
            qualified: sym.qualified.clone(),
        });
    }
    let mut objectives = Vec::new();
    for (i, sym) in info.symbols.iter().enumerate() {
        if sym.kind == SymbolKind::Objective {
            objectives.push(ObjtRec {
                symb: i as u16 + 1,
                dummy_only: 1,
                display: None,
            });
        }
    }
    let mut chains = Vec::new();
    for chain in &info.chains {
        let name_symb = symbols
            .iter()
            .find(|s| s.qualified.ends_with(&format!(".chain.{}", chain.name)))
            .map(|s| s.id)
            .unwrap_or(0);
        let origin = chain.origin.unwrap_or([0, 0, 0]);
        chains.push(ChainRec {
            name_symb,
            layout: layout_id(chain.layout.as_deref()),
            facing: facing_id(chain.facing.as_deref()),
            origin: [origin[0] as i32, origin[1] as i32, origin[2] as i32],
            length: 0,
            clock: u8::from(chain.is_clock),
            pack_mode: if chain.pack_mode.as_deref() == Some("function") {
                1
            } else {
                0
            },
        });
    }
    let world_blocks = info
        .blocks
        .iter()
        .map(|b| WblkRec {
            x: b.at[0] as i32,
            y: b.at[1] as i32,
            z: b.at[2] as i32,
            block: b.block.clone(),
        })
        .collect();
    let containers: Vec<ContRec> = info
        .containers
        .iter()
        .map(|c| ContRec {
            x: c.at[0] as i32,
            y: c.at[1] as i32,
            z: c.at[2] as i32,
            block: c.kind.block_id().to_string(),
            facing: facing_id(c.facing.as_deref()),
            slots: c
                .slots
                .iter()
                .map(|s| ContSlot {
                    slot: s.slot as u8,
                    item: s.item.clone(),
                    count: s.count as u16,
                    data: s.data.unwrap_or(0) as i32,
                    nbt: nbt_from_slot(s.title.as_deref(), s.pages.as_deref()),
                })
                .collect(),
        })
        .collect();
    intern_container_ids(&mut symbols, &containers);
    let tags: Vec<u16> = symbols
        .iter()
        .filter(|s| s.kind == 1)
        .map(|s| s.id)
        .collect();
    let links = info
        .links
        .iter()
        .map(|l| LinkRec {
            from_chain: symb_id_ending(&symbols, &format!(".chain.{}", l.from_chain)),
            from_label: l
                .from_label
                .as_ref()
                .map(|lab| symb_id_ending(&symbols, &format!(".chain.{}.{}", l.from_chain, lab)))
                .unwrap_or(0),
            to_chain: symb_id_ending(&symbols, &format!(".chain.{}", l.to)),
            kind: 0,
        })
        .collect();
    let meta = serde_meta(info, edition, game_version, info.host_tick.as_deref());
    MincbImage {
        edition,
        game_version: game_version.to_string(),
        pack: info.pack.clone(),
        origin,
        score_revision: info.score_revision,
        symbols,
        objectives,
        functions: Vec::new(),
        chains,
        command_blocks: Vec::new(),
        world_blocks,
        containers,
        links,
        tags,
        tick_values: Vec::new(),
        meta_json: meta,
    }
}

fn intern_container_ids(symbols: &mut Vec<SymbRec>, containers: &[ContRec]) {
    for c in containers {
        intern_name(symbols, &c.block);
        for s in &c.slots {
            intern_name(symbols, &s.item);
        }
    }
}

/// Intern a block/item name as SYMB kind 6 and return its id.
pub fn intern_string(image: &mut MincbImage, name: &str) -> u16 {
    intern_name(&mut image.symbols, name)
}

fn intern_name(symbols: &mut Vec<SymbRec>, name: &str) -> u16 {
    if name.is_empty() {
        return 0;
    }
    if let Some(s) = symbols
        .iter()
        .find(|s| s.kind == 6 && (s.short == name || s.qualified == name))
    {
        return s.id;
    }
    let next = symbols.iter().map(|s| s.id).max().unwrap_or(0) + 1;
    let qualified = if name.contains(':') {
        name.to_string()
    } else {
        format!("minecraft:{name}")
    };
    symbols.push(SymbRec {
        id: next,
        kind: 6,
        short: name.to_string(),
        qualified,
    });
    next
}

fn lookup_intern(symbols: &[SymbRec], name: &str) -> u16 {
    symbols
        .iter()
        .find(|s| {
            s.kind == 6 && (s.short == name || s.qualified == name || s.qualified.ends_with(name))
        })
        .map(|s| s.id)
        .unwrap_or(0)
}

fn intern_name_of(symbols: &[SymbRec], id: u16) -> String {
    symbols
        .iter()
        .find(|s| s.id == id)
        .map(|s| s.short.clone())
        .unwrap_or_default()
}

fn inject_tick_values(meta: &str, ids: &[u16]) -> String {
    let ticks = ids
        .iter()
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(",");
    if let Some(idx) = meta.find("\"tick_values\"") {
        let rest = &meta[idx..];
        if let Some(br) = rest.find('[') {
            if let Some(end) = rest[br..].find(']') {
                let start = idx + br;
                let stop = idx + br + end + 1;
                let mut out = String::new();
                out.push_str(&meta[..start]);
                out.push('[');
                out.push_str(&ticks);
                out.push(']');
                out.push_str(&meta[stop..]);
                return out;
            }
        }
    }
    if meta.ends_with('}') {
        let mut out = meta.trim_end_matches('}').to_string();
        if !out.ends_with('{') && !out.ends_with(',') {
            out.push(',');
        }
        out.push_str(&format!("\"tick_values\":[{ticks}]}}"));
        return out;
    }
    meta.to_string()
}

fn nbt_from_slot(title: Option<&str>, pages: Option<&str>) -> String {
    match (title, pages) {
        (None, None) => String::new(),
        (title, pages) => format!(
            "{{\"title\":\"{}\",\"pages\":\"{}\"}}",
            title.unwrap_or(""),
            pages.unwrap_or("")
        ),
    }
}

fn serde_meta(
    info: &BinaryInfo,
    edition: Edition,
    game_version: &str,
    host: Option<&str>,
) -> String {
    let mut areas = String::from("[");
    for (i, a) in info.ticking_areas.iter().enumerate() {
        if i > 0 {
            areas.push(',');
        }
        let center = a.center.unwrap_or([0, 0, 0]);
        areas.push_str(&format!(
            "{{\"name\":\"{}\",\"center\":[{},{},{}],\"radius\":{},\"preload\":{}}}",
            a.name,
            center[0],
            center[1],
            center[2],
            a.radius.unwrap_or(0),
            a.preload
        ));
    }
    areas.push(']');
    let mut fills = String::from("[");
    for (i, f) in info.fills.iter().enumerate() {
        if i > 0 {
            fills.push(',');
        }
        fills.push_str(&format!(
            "{{\"from\":[{},{},{}],\"to\":[{},{},{}],\"block\":\"{}\",\"replace\":\"{}\"}}",
            f.from[0],
            f.from[1],
            f.from[2],
            f.to[0],
            f.to[1],
            f.to[2],
            f.block,
            f.replace.as_deref().unwrap_or("")
        ));
    }
    fills.push(']');
    let mut rules = String::from("[");
    for (i, (k, v)) in info.gamerules.iter().enumerate() {
        if i > 0 {
            rules.push(',');
        }
        rules.push_str(&format!("{{\"name\":\"{k}\",\"value\":\"{v}\"}}"));
    }
    rules.push(']');
    format!(
        "{{\"edition\":\"{}\",\"game_version\":\"{}\",\"host_tick\":\"{}\",\"dimension\":\"{}\",\"ticking_areas\":{},\"fills\":{},\"gamerules\":{},\"tick_values\":[{}]}}",
        edition.as_str(),
        game_version,
        host.unwrap_or(""),
        info.dimension.as_deref().unwrap_or("overworld"),
        areas,
        fills,
        rules,
        ""
    )
}

fn symb_id_ending(symbols: &[SymbRec], suffix: &str) -> u16 {
    symbols
        .iter()
        .find(|s| s.qualified.ends_with(suffix))
        .map(|s| s.id)
        .unwrap_or(0)
}

pub fn layout_id(layout: Option<&str>) -> u8 {
    match layout.unwrap_or("stack") {
        "linear" => 0,
        "stack" => 1,
        "snake" => 2,
        "box" => 3,
        "points" => 4,
        _ => 1,
    }
}

pub fn facing_id(facing: Option<&str>) -> u8 {
    match facing.unwrap_or("up") {
        "down" => 0,
        "up" => 1,
        "north" => 2,
        "south" => 3,
        "west" => 4,
        "east" => 5,
        _ => 1,
    }
}

pub fn facing_name(id: u8) -> &'static str {
    match id {
        0 => "down",
        1 => "up",
        2 => "north",
        3 => "south",
        4 => "west",
        5 => "east",
        _ => "up",
    }
}

pub fn encode_image(image: &MincbImage) -> Vec<u8> {
    let mut sections: Vec<(u32, u32, Vec<u8>)> = Vec::new();

    let mut symb = Vec::new();
    for s in &image.symbols {
        write_u16(&mut symb, s.id);
        symb.push(s.kind);
        write_str16(&mut symb, &s.short);
        write_str16(&mut symb, &s.qualified);
    }
    sections.push((SID_SYMB, image.symbols.len() as u32, symb));

    let mut objt = Vec::new();
    for o in &image.objectives {
        write_u16(&mut objt, o.symb);
        objt.push(o.dummy_only);
        match &o.display {
            Some(d) => {
                objt.push(1);
                write_str16(&mut objt, d);
            }
            None => objt.push(0),
        }
    }
    sections.push((SID_OBJT, image.objectives.len() as u32, objt));

    let tags: Vec<u16> = if image.tags.is_empty() {
        image
            .symbols
            .iter()
            .filter(|s| s.kind == 1)
            .map(|s| s.id)
            .collect()
    } else {
        image.tags.clone()
    };
    let mut tags_body = Vec::new();
    for id in &tags {
        write_u16(&mut tags_body, *id);
    }
    sections.push((SID_TAGS, tags.len() as u32, tags_body));

    let mut func = Vec::new();
    for f in &image.functions {
        write_u16(&mut func, f.symb);
        write_u32(&mut func, f.body.len() as u32);
        func.extend_from_slice(f.body.as_bytes());
    }
    sections.push((SID_FUNC, image.functions.len() as u32, func));

    let mut chain = Vec::new();
    for c in &image.chains {
        write_u16(&mut chain, c.name_symb);
        chain.push(c.layout);
        chain.push(c.facing);
        write_i32(&mut chain, c.origin[0]);
        write_i32(&mut chain, c.origin[1]);
        write_i32(&mut chain, c.origin[2]);
        write_u16(&mut chain, c.length);
        chain.push(c.clock);
        chain.push(c.pack_mode);
    }
    sections.push((SID_CHAIN, image.chains.len() as u32, chain));

    let mut cblk = Vec::new();
    for b in &image.command_blocks {
        write_i32(&mut cblk, b.x);
        write_i32(&mut cblk, b.y);
        write_i32(&mut cblk, b.z);
        cblk.push(b.facing);
        cblk.push(b.mode);
        cblk.push(b.flags);
        write_u32(&mut cblk, b.delay_ticks);
        write_u32(&mut cblk, b.command.len() as u32);
        cblk.extend_from_slice(b.command.as_bytes());
    }
    sections.push((SID_CBLK, image.command_blocks.len() as u32, cblk));

    let mut palette: Vec<String> = Vec::new();
    let mut wblk = Vec::new();
    for b in &image.world_blocks {
        let idx = intern(&mut palette, &b.block);
        write_i32(&mut wblk, b.x);
        write_i32(&mut wblk, b.y);
        write_i32(&mut wblk, b.z);
        write_u16(&mut wblk, idx);
        write_u32(&mut wblk, 0);
    }
    let mut wblk_full = Vec::new();
    write_u32(&mut wblk_full, palette.len() as u32);
    for p in &palette {
        write_str16(&mut wblk_full, p);
    }
    wblk_full.extend_from_slice(&wblk);
    sections.push((SID_WBLK, image.world_blocks.len() as u32, wblk_full));

    let mut cont = Vec::new();
    for c in &image.containers {
        write_i32(&mut cont, c.x);
        write_i32(&mut cont, c.y);
        write_i32(&mut cont, c.z);
        write_u16(&mut cont, lookup_intern(&image.symbols, &c.block));
        cont.push(c.facing);
        write_u16(&mut cont, c.slots.len() as u16);
        for s in &c.slots {
            cont.push(s.slot);
            write_u16(&mut cont, lookup_intern(&image.symbols, &s.item));
            write_u16(&mut cont, s.count);
            write_i32(&mut cont, s.data);
            write_u32(&mut cont, s.nbt.len() as u32);
            cont.extend_from_slice(s.nbt.as_bytes());
        }
    }
    sections.push((SID_CONT, image.containers.len() as u32, cont));

    let mut link = Vec::new();
    for l in &image.links {
        write_u16(&mut link, l.from_chain);
        write_u16(&mut link, l.from_label);
        write_u16(&mut link, l.to_chain);
        link.push(l.kind);
    }
    sections.push((SID_LINK, image.links.len() as u32, link));

    let meta_json = inject_tick_values(&image.meta_json, &image.tick_values);
    let mut meta = Vec::new();
    write_u32(&mut meta, meta_json.len() as u32);
    meta.extend_from_slice(meta_json.as_bytes());
    sections.push((SID_META, 1, meta));

    let mut flags = 0u8;
    if !image.functions.is_empty() {
        flags |= 1;
    }
    if !image.command_blocks.is_empty() {
        flags |= 2;
    }
    if !image.world_blocks.is_empty() || !image.containers.is_empty() {
        flags |= 4;
    }

    let table_bytes = 16 * sections.len();
    let header = 40 + table_bytes;
    let gv = encode_str16(&image.game_version);
    let pk = encode_str16(&image.pack);
    let gv_off = header as u32;
    let pk_off = gv_off + gv.len() as u32;
    let mut cursor = pk_off + pk.len() as u32;

    let mut table = Vec::new();
    let mut bodies = Vec::new();
    for (id, count, data) in &sections {
        write_u32(&mut table, *id);
        write_u32(&mut table, cursor);
        write_u32(&mut table, data.len() as u32);
        write_u32(&mut table, *count);
        cursor += data.len() as u32;
        bodies.extend_from_slice(data);
    }

    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    write_u16(&mut out, FORMAT_MAJOR);
    write_u16(&mut out, FORMAT_MINOR);
    out.push(image.edition.byte());
    out.push(flags);
    write_u16(&mut out, 0);
    write_u32(&mut out, gv_off);
    write_u32(&mut out, pk_off);
    write_i32(&mut out, image.origin[0]);
    write_i32(&mut out, image.origin[1]);
    write_i32(&mut out, image.origin[2]);
    write_u32(&mut out, image.score_revision);
    write_u32(&mut out, sections.len() as u32);
    debug_assert_eq!(out.len(), 40);
    out.extend_from_slice(&table);
    debug_assert_eq!(out.len(), header);
    out.extend_from_slice(&gv);
    out.extend_from_slice(&pk);
    out.extend_from_slice(&bodies);
    out
}

/// Read the pack name from a MINCB blob (spec header `pack_off`).
pub fn decode_pack(bytes: &[u8]) -> Option<String> {
    if bytes.len() < 40 || &bytes[0..4] != MAGIC {
        return None;
    }
    let pack_off = u32::from_le_bytes(bytes[16..20].try_into().ok()?) as usize;
    let (pack, _) = read_str16(bytes.get(pack_off..)?)?;
    Some(pack)
}

pub fn decode_header(bytes: &[u8]) -> Option<InspectHeader> {
    if bytes.len() < 40 || &bytes[0..4] != MAGIC {
        return None;
    }
    let major = u16::from_le_bytes(bytes[4..6].try_into().ok()?);
    let minor = u16::from_le_bytes(bytes[6..8].try_into().ok()?);
    let edition = bytes[8];
    let flags = bytes[9];
    let gv_off = u32::from_le_bytes(bytes[12..16].try_into().ok()?) as usize;
    let pack_off = u32::from_le_bytes(bytes[16..20].try_into().ok()?) as usize;
    let ox = i32::from_le_bytes(bytes[20..24].try_into().ok()?);
    let oy = i32::from_le_bytes(bytes[24..28].try_into().ok()?);
    let oz = i32::from_le_bytes(bytes[28..32].try_into().ok()?);
    let rev = u32::from_le_bytes(bytes[32..36].try_into().ok()?);
    let (game_version, _) = read_str16(bytes.get(gv_off..)?)?;
    let (pack, _) = read_str16(bytes.get(pack_off..)?)?;
    Some(InspectHeader {
        major,
        minor,
        edition,
        flags,
        game_version,
        pack,
        origin: [ox, oy, oz],
        score_revision: rev,
    })
}

#[derive(Debug, Clone)]
pub struct InspectHeader {
    pub major: u16,
    pub minor: u16,
    pub edition: u8,
    pub flags: u8,
    pub game_version: String,
    pub pack: String,
    pub origin: [i32; 3],
    pub score_revision: u32,
}

pub fn inspect_text(bytes: &[u8], info: Option<&BinaryInfo>, blocks: &[CbInstance]) -> String {
    let Some(h) = decode_header(bytes) else {
        return "not a MINCB file\n".into();
    };
    let edition = match h.edition {
        1 => "java",
        2 => "bedrock",
        n => return format!("unknown edition {n}\n"),
    };
    let mut out = format!(
        "MINCB format {}.{}\nedition {edition}\ngame_version {}\npack {}\norigin {} {} {}\nscore_revision {}\n",
        h.major, h.minor, h.game_version, h.pack, h.origin[0], h.origin[1], h.origin[2], h.score_revision
    );
    if let Some(info) = info {
        out.push_str("symbols:\n");
        for s in &info.symbols {
            out.push_str(&format!(
                "  {} {} {}\n",
                kind_name(s.kind),
                s.id,
                s.qualified
            ));
        }
        out.push_str("chains:\n");
        for c in &info.chains {
            out.push_str(&format!(
                "  {} layout={} facing={} origin={:?} clock={}\n",
                c.name,
                c.layout.as_deref().unwrap_or("-"),
                c.facing.as_deref().unwrap_or("-"),
                c.origin,
                c.is_clock
            ));
        }
        if !info.containers.is_empty() {
            out.push_str("containers:\n");
            for c in &info.containers {
                out.push_str(&format!(
                    "  {} {} at {:?}\n",
                    c.kind.block_id(),
                    c.name,
                    c.at
                ));
            }
        }
    } else if let Some(image) = decode_image(bytes) {
        out.push_str("symbols:\n");
        for s in &image.symbols {
            let kn = match s.kind {
                0 => "obj",
                1 => "tag",
                2 => "fake",
                3 => "fn",
                4 => "chain",
                5 => "label",
                6 => "id",
                _ => "?",
            };
            out.push_str(&format!("  {kn} {} {} (#{})\n", s.short, s.qualified, s.id));
        }
        out.push_str("chains:\n");
        let mut cursor = 0usize;
        for c in &image.chains {
            let name = symb_name(&image, c.name_symb);
            let len = c.length as usize;
            let slice = image
                .command_blocks
                .get(cursor..cursor + len)
                .unwrap_or(&[]);
            let mut min = c.origin;
            let mut max = c.origin;
            for b in slice {
                min = [min[0].min(b.x), min[1].min(b.y), min[2].min(b.z)];
                max = [max[0].max(b.x), max[1].max(b.y), max[2].max(b.z)];
            }
            out.push_str(&format!(
                "  {name} layout={} facing={} origin={:?} length={} clock={} bbox {:?}..{:?}\n",
                c.layout, c.facing, c.origin, c.length, c.clock, min, max
            ));
            if let Some(first) = slice.first() {
                out.push_str(&format!(
                    "    first_cb {} {} {} {}\n",
                    first.x, first.y, first.z, first.command
                ));
            }
            if let Some(last) = slice.last() {
                out.push_str(&format!(
                    "    last_cb {} {} {} {}\n",
                    last.x, last.y, last.z, last.command
                ));
            }
            cursor += len;
        }
        if !image.containers.is_empty() {
            out.push_str("containers:\n");
            for c in &image.containers {
                out.push_str(&format!(
                    "  {} at {} {} {} slots={}\n",
                    c.block,
                    c.x,
                    c.y,
                    c.z,
                    c.slots.len()
                ));
                for s in &c.slots {
                    out.push_str(&format!("    slot {} {} * {}\n", s.slot, s.item, s.count));
                }
            }
        }
        if !image.meta_json.is_empty() {
            out.push_str("meta ");
            out.push_str(&image.meta_json);
            out.push('\n');
        }
    }
    if let Some(first) = blocks.first() {
        out.push_str(&format!(
            "first_cb {} {} {} {}\n",
            first.x, first.y, first.z, first.command
        ));
    }
    if let Some(last) = blocks.last() {
        out.push_str(&format!(
            "last_cb {} {} {} {}\n",
            last.x, last.y, last.z, last.command
        ));
    }
    out
}

/// Decode a full MINCB image (header + every section).
pub fn decode_image(bytes: &[u8]) -> Option<MincbImage> {
    let h = decode_header(bytes)?;
    let edition = match h.edition {
        1 => Edition::Java,
        2 => Edition::Bedrock,
        _ => return None,
    };
    let count = u32::from_le_bytes(bytes.get(36..40)?.try_into().ok()?) as usize;
    let mut symbols = Vec::new();
    let mut objectives = Vec::new();
    let mut functions = Vec::new();
    let mut chains = Vec::new();
    let mut command_blocks = Vec::new();
    let mut world_blocks = Vec::new();
    let mut containers = Vec::new();
    let mut links = Vec::new();
    let mut tags = Vec::new();
    let mut meta_json = String::new();

    for i in 0..count {
        let off = 40 + i * 16;
        let id = u32::from_le_bytes(bytes.get(off..off + 4)?.try_into().ok()?);
        let offset = u32::from_le_bytes(bytes.get(off + 4..off + 8)?.try_into().ok()?) as usize;
        let size = u32::from_le_bytes(bytes.get(off + 8..off + 12)?.try_into().ok()?) as usize;
        let n = u32::from_le_bytes(bytes.get(off + 12..off + 16)?.try_into().ok()?) as usize;
        let body = bytes.get(offset..offset + size)?;
        if id == SID_SYMB {
            let mut p = 0usize;
            for _ in 0..n {
                let sid = u16::from_le_bytes(body.get(p..p + 2)?.try_into().ok()?);
                let kind = *body.get(p + 2)?;
                p += 3;
                let (short, used) = read_str16(body.get(p..)?)?;
                p += used;
                let (qualified, used) = read_str16(body.get(p..)?)?;
                p += used;
                symbols.push(SymbRec {
                    id: sid,
                    kind,
                    short,
                    qualified,
                });
            }
        } else if id == SID_OBJT {
            let mut p = 0usize;
            for _ in 0..n {
                let symb = u16::from_le_bytes(body.get(p..p + 2)?.try_into().ok()?);
                let dummy_only = *body.get(p + 2)?;
                p += 3;
                let has = *body.get(p)?;
                p += 1;
                let display = if has == 1 {
                    let (d, used) = read_str16(body.get(p..)?)?;
                    p += used;
                    Some(d)
                } else {
                    None
                };
                objectives.push(ObjtRec {
                    symb,
                    dummy_only,
                    display,
                });
            }
        } else if id == SID_TAGS {
            let mut p = 0usize;
            for _ in 0..n {
                let id = u16::from_le_bytes(body.get(p..p + 2)?.try_into().ok()?);
                p += 2;
                tags.push(id);
            }
        } else if id == SID_FUNC {
            let mut p = 0usize;
            for _ in 0..n {
                let symb = u16::from_le_bytes(body.get(p..p + 2)?.try_into().ok()?);
                p += 2;
                let blen = u32::from_le_bytes(body.get(p..p + 4)?.try_into().ok()?) as usize;
                p += 4;
                let body_s = std::str::from_utf8(body.get(p..p + blen)?)
                    .ok()?
                    .to_string();
                p += blen;
                functions.push(FuncRec {
                    symb,
                    path: String::new(),
                    body: body_s,
                });
            }
        } else if id == SID_CHAIN {
            let mut p = 0usize;
            for _ in 0..n {
                let name_symb = u16::from_le_bytes(body.get(p..p + 2)?.try_into().ok()?);
                p += 2;
                let layout = *body.get(p)?;
                let facing = *body.get(p + 1)?;
                p += 2;
                let ox = i32::from_le_bytes(body.get(p..p + 4)?.try_into().ok()?);
                let oy = i32::from_le_bytes(body.get(p + 4..p + 8)?.try_into().ok()?);
                let oz = i32::from_le_bytes(body.get(p + 8..p + 12)?.try_into().ok()?);
                p += 12;
                let length = u16::from_le_bytes(body.get(p..p + 2)?.try_into().ok()?);
                p += 2;
                let clock = *body.get(p)?;
                let pack_mode = *body.get(p + 1)?;
                p += 2;
                chains.push(ChainRec {
                    name_symb,
                    layout,
                    facing,
                    origin: [ox, oy, oz],
                    length,
                    clock,
                    pack_mode,
                });
            }
        } else if id == SID_CBLK {
            let mut p = 0usize;
            for _ in 0..n {
                let x = i32::from_le_bytes(body.get(p..p + 4)?.try_into().ok()?);
                let y = i32::from_le_bytes(body.get(p + 4..p + 8)?.try_into().ok()?);
                let z = i32::from_le_bytes(body.get(p + 8..p + 12)?.try_into().ok()?);
                p += 12;
                let facing = *body.get(p)?;
                let mode = *body.get(p + 1)?;
                let flags = *body.get(p + 2)?;
                p += 3;
                let delay_ticks = u32::from_le_bytes(body.get(p..p + 4)?.try_into().ok()?);
                p += 4;
                let clen = u32::from_le_bytes(body.get(p..p + 4)?.try_into().ok()?) as usize;
                p += 4;
                let command = std::str::from_utf8(body.get(p..p + clen)?)
                    .ok()?
                    .to_string();
                p += clen;
                command_blocks.push(CblkRec {
                    x,
                    y,
                    z,
                    facing,
                    mode,
                    flags,
                    delay_ticks,
                    command,
                });
            }
        } else if id == SID_WBLK {
            let pal_n = u32::from_le_bytes(body.get(0..4)?.try_into().ok()?) as usize;
            let mut p = 4usize;
            let mut palette = Vec::new();
            for _ in 0..pal_n {
                let (name, used) = read_str16(body.get(p..)?)?;
                p += used;
                palette.push(name);
            }
            for _ in 0..n {
                let x = i32::from_le_bytes(body.get(p..p + 4)?.try_into().ok()?);
                let y = i32::from_le_bytes(body.get(p + 4..p + 8)?.try_into().ok()?);
                let z = i32::from_le_bytes(body.get(p + 8..p + 12)?.try_into().ok()?);
                p += 12;
                let idx = u16::from_le_bytes(body.get(p..p + 2)?.try_into().ok()?);
                p += 2;
                let _state = u32::from_le_bytes(body.get(p..p + 4)?.try_into().ok()?);
                p += 4;
                world_blocks.push(WblkRec {
                    x,
                    y,
                    z,
                    block: palette.get(idx as usize).cloned().unwrap_or_default(),
                });
            }
        } else if id == SID_CONT {
            let mut p = 0usize;
            for _ in 0..n {
                let x = i32::from_le_bytes(body.get(p..p + 4)?.try_into().ok()?);
                let y = i32::from_le_bytes(body.get(p + 4..p + 8)?.try_into().ok()?);
                let z = i32::from_le_bytes(body.get(p + 8..p + 12)?.try_into().ok()?);
                p += 12;
                let block_id = u16::from_le_bytes(body.get(p..p + 2)?.try_into().ok()?);
                p += 2;
                let facing = *body.get(p)?;
                p += 1;
                let slot_n = u16::from_le_bytes(body.get(p..p + 2)?.try_into().ok()?);
                p += 2;
                let mut slots = Vec::new();
                for _ in 0..slot_n {
                    let slot = *body.get(p)?;
                    p += 1;
                    let item_id = u16::from_le_bytes(body.get(p..p + 2)?.try_into().ok()?);
                    p += 2;
                    let count = u16::from_le_bytes(body.get(p..p + 2)?.try_into().ok()?);
                    p += 2;
                    let data = i32::from_le_bytes(body.get(p..p + 4)?.try_into().ok()?);
                    p += 4;
                    let nlen = u32::from_le_bytes(body.get(p..p + 4)?.try_into().ok()?) as usize;
                    p += 4;
                    let nbt = std::str::from_utf8(body.get(p..p + nlen)?)
                        .ok()?
                        .to_string();
                    p += nlen;
                    slots.push(ContSlot {
                        slot,
                        item: intern_name_of(&symbols, item_id),
                        count,
                        data,
                        nbt,
                    });
                }
                containers.push(ContRec {
                    x,
                    y,
                    z,
                    block: intern_name_of(&symbols, block_id),
                    facing,
                    slots,
                });
            }
        } else if id == SID_LINK {
            let mut p = 0usize;
            for _ in 0..n {
                let from_chain = u16::from_le_bytes(body.get(p..p + 2)?.try_into().ok()?);
                let from_label = u16::from_le_bytes(body.get(p + 2..p + 4)?.try_into().ok()?);
                let to_chain = u16::from_le_bytes(body.get(p + 4..p + 6)?.try_into().ok()?);
                let kind = *body.get(p + 6)?;
                p += 7;
                links.push(LinkRec {
                    from_chain,
                    from_label,
                    to_chain,
                    kind,
                });
            }
        } else if id == SID_META {
            let len = u32::from_le_bytes(body.get(0..4)?.try_into().ok()?) as usize;
            meta_json = std::str::from_utf8(body.get(4..4 + len)?).ok()?.to_string();
        }
    }

    for f in &mut functions {
        if f.path.is_empty() {
            f.path = symbols
                .iter()
                .find(|s| s.id == f.symb)
                .map(|s| s.short.clone())
                .unwrap_or_default();
        }
    }
    if tags.is_empty() {
        tags = symbols
            .iter()
            .filter(|s| s.kind == 1)
            .map(|s| s.id)
            .collect();
    }
    let tick_values = parse_tick_values(&meta_json);

    Some(MincbImage {
        edition,
        game_version: h.game_version,
        pack: h.pack,
        origin: h.origin,
        score_revision: h.score_revision,
        symbols,
        objectives,
        functions,
        chains,
        command_blocks,
        world_blocks,
        containers,
        links,
        tags,
        tick_values,
        meta_json,
    })
}

pub fn symb_name(image: &MincbImage, id: u16) -> String {
    image
        .symbols
        .iter()
        .find(|s| s.id == id)
        .map(|s| {
            s.qualified
                .rsplit('.')
                .next()
                .unwrap_or(&s.short)
                .to_string()
        })
        .unwrap_or_else(|| id.to_string())
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

/// JSON snapshot of a MINCB image for MincBot / tooling.
pub fn dump_image_json(image: &MincbImage) -> String {
    let edition = image.edition.as_str();
    let mut out = String::from("{\n");
    out.push_str(&format!(
        "  \"edition\":\"{edition}\",\n  \"game_version\":\"{}\",\n  \"pack\":\"{}\",\n  \"origin\":[{},{},{}],\n  \"score_revision\":{},\n",
        json_escape(&image.game_version),
        json_escape(&image.pack),
        image.origin[0],
        image.origin[1],
        image.origin[2],
        image.score_revision
    ));
    out.push_str(&format!(
        "  \"meta\":{},\n",
        if image.meta_json.is_empty() {
            "{}".into()
        } else {
            image.meta_json.clone()
        }
    ));
    out.push_str("  \"world_blocks\":[\n");
    for (i, b) in image.world_blocks.iter().enumerate() {
        if i > 0 {
            out.push_str(",\n");
        }
        out.push_str(&format!(
            "    {{\"x\":{},\"y\":{},\"z\":{},\"block\":\"{}\"}}",
            b.x,
            b.y,
            b.z,
            json_escape(&b.block)
        ));
    }
    out.push_str("\n  ],\n  \"command_blocks\":[\n");
    for (i, b) in image.command_blocks.iter().enumerate() {
        if i > 0 {
            out.push_str(",\n");
        }
        out.push_str(&format!(
            "    {{\"x\":{},\"y\":{},\"z\":{},\"facing\":{},\"mode\":{},\"flags\":{},\"delay\":{},\"command\":\"{}\"}}",
            b.x,
            b.y,
            b.z,
            b.facing,
            b.mode,
            b.flags,
            b.delay_ticks,
            json_escape(&b.command)
        ));
    }
    out.push_str("\n  ],\n  \"containers\":[\n");
    for (i, c) in image.containers.iter().enumerate() {
        if i > 0 {
            out.push_str(",\n");
        }
        out.push_str(&format!(
            "    {{\"x\":{},\"y\":{},\"z\":{},\"block\":\"{}\",\"facing\":{},\"slots\":[",
            c.x,
            c.y,
            c.z,
            json_escape(&c.block),
            c.facing
        ));
        for (j, s) in c.slots.iter().enumerate() {
            if j > 0 {
                out.push(',');
            }
            out.push_str(&format!(
                "{{\"slot\":{},\"item\":\"{}\",\"count\":{}}}",
                s.slot,
                json_escape(&s.item),
                s.count
            ));
        }
        out.push_str("]}");
    }
    out.push_str("\n  ]\n}\n");
    out
}

/// Print FUNC bodies and CBLK command strings from a decoded image.
pub fn dump_commands_from_image(image: &MincbImage) -> String {
    let mut out = String::new();
    for f in &image.functions {
        out.push_str(&format!("# function {}\n", func_path(image, f)));
        out.push_str(&f.body);
        if !f.body.ends_with('\n') {
            out.push('\n');
        }
    }
    let mut cursor = 0usize;
    for chain in &image.chains {
        let name = symb_name(image, chain.name_symb);
        let len = chain.length as usize;
        let slice = image
            .command_blocks
            .get(cursor..cursor + len)
            .unwrap_or(&[]);
        out.push_str(&format!("# chain {name}\n"));
        for b in slice {
            out.push_str(&b.command);
            out.push('\n');
        }
        cursor += len;
    }
    if image.chains.is_empty() {
        for b in &image.command_blocks {
            out.push_str(&b.command);
            out.push('\n');
        }
    }
    out
}

pub fn inspect_chain_text(image: &MincbImage, chain_name: &str) -> String {
    let mut cursor = 0usize;
    for chain in &image.chains {
        let name = symb_name(image, chain.name_symb);
        let len = chain.length as usize;
        let slice = image
            .command_blocks
            .get(cursor..cursor + len)
            .unwrap_or(&[]);
        if name == chain_name {
            let mut min = chain.origin;
            let mut max = chain.origin;
            for b in slice {
                min = [min[0].min(b.x), min[1].min(b.y), min[2].min(b.z)];
                max = [max[0].max(b.x), max[1].max(b.y), max[2].max(b.z)];
            }
            let mut out = format!(
                "chain {name}\nlayout {} facing {} origin {} {} {}\nlength {} clock {} pack_mode {}\nbbox {} {} {} .. {} {} {}\n",
                chain.layout,
                chain.facing,
                chain.origin[0],
                chain.origin[1],
                chain.origin[2],
                chain.length,
                chain.clock,
                chain.pack_mode,
                min[0],
                min[1],
                min[2],
                max[0],
                max[1],
                max[2]
            );
            if let Some(first) = slice.first() {
                out.push_str(&format!(
                    "first_cb {} {} {} {}\n",
                    first.x, first.y, first.z, first.command
                ));
            }
            if let Some(last) = slice.last() {
                out.push_str(&format!(
                    "last_cb {} {} {} {}\n",
                    last.x, last.y, last.z, last.command
                ));
            }
            return out;
        }
        cursor += len;
    }
    format!("chain `{chain_name}` not found\n")
}

fn kind_name(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Objective => "obj",
        SymbolKind::Tag => "tag",
        SymbolKind::FakePlayer => "fake",
        SymbolKind::Function => "fn",
        SymbolKind::Chain => "chain",
        SymbolKind::Label => "label",
        SymbolKind::Intern => "id",
    }
}

fn parse_tick_values(meta: &str) -> Vec<u16> {
    let Some(idx) = meta.find("\"tick_values\"") else {
        return Vec::new();
    };
    let rest = &meta[idx..];
    let Some(br) = rest.find('[') else {
        return Vec::new();
    };
    let rest = &rest[br + 1..];
    let end = rest.find(']').unwrap_or(rest.len());
    rest[..end]
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect()
}

pub fn func_path(image: &MincbImage, f: &FuncRec) -> String {
    if !f.path.is_empty() {
        return f.path.clone();
    }
    image
        .symbols
        .iter()
        .find(|s| s.id == f.symb)
        .map(|s| s.short.clone())
        .unwrap_or_default()
}

fn intern(palette: &mut Vec<String>, name: &str) -> u16 {
    if let Some(i) = palette.iter().position(|p| p == name) {
        return i as u16;
    }
    palette.push(name.to_string());
    (palette.len() - 1) as u16
}

fn write_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn write_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn write_i32(out: &mut Vec<u8>, v: i32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn write_str16(out: &mut Vec<u8>, s: &str) {
    write_u16(out, s.len() as u16);
    out.extend_from_slice(s.as_bytes());
}

fn encode_str16(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    write_str16(&mut out, s);
    out
}

fn read_str16(bytes: &[u8]) -> Option<(String, usize)> {
    if bytes.len() < 2 {
        return None;
    }
    let n = u16::from_le_bytes(bytes[0..2].try_into().ok()?) as usize;
    let s = std::str::from_utf8(bytes.get(2..2 + n)?).ok()?;
    Some((s.to_string(), 2 + n))
}
