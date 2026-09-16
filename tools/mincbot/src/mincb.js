//! MINCB decoder — same layout as `src/mincb.rs` / docs/language/05.

function u8(b, o) {
  return b[o];
}
function u16(b, o) {
  return b[o] | (b[o + 1] << 8);
}
function u32(b, o) {
  return (b[o] | (b[o + 1] << 8) | (b[o + 2] << 16) | (b[o + 3] << 24)) >>> 0;
}
function i32(b, o) {
  return u32(b, o) | 0;
}

function str16(b, o) {
  const n = u16(b, o);
  const s = b.slice(o + 2, o + 2 + n).toString("utf8");
  return { s, n: 2 + n };
}

const SID = {
  SYMB: Buffer.from("SYMB").readUInt32LE(0),
  OBJT: Buffer.from("OBJT").readUInt32LE(0),
  TAGS: Buffer.from("TAGS").readUInt32LE(0),
  FUNC: Buffer.from("FUNC").readUInt32LE(0),
  CHAI: Buffer.from("CHAI").readUInt32LE(0),
  CBLK: Buffer.from("CBLK").readUInt32LE(0),
  WBLK: Buffer.from("WBLK").readUInt32LE(0),
  CONT: Buffer.from("CONT").readUInt32LE(0),
  LINK: Buffer.from("LINK").readUInt32LE(0),
  META: Buffer.from("META").readUInt32LE(0),
};

function internName(symbols, id) {
  const s = symbols.find((x) => x.id === id);
  return s ? s.short : "";
}

function decodeHeader(bytes) {
  if (bytes.length < 40 || bytes.slice(0, 4).toString("ascii") !== "MINC") {
    return null;
  }
  const gvOff = u32(bytes, 12);
  const packOff = u32(bytes, 16);
  return {
    major: u16(bytes, 4),
    minor: u16(bytes, 6),
    edition: u8(bytes, 8),
    flags: u8(bytes, 9),
    game_version: str16(bytes, gvOff).s,
    pack: str16(bytes, packOff).s,
    origin: [i32(bytes, 20), i32(bytes, 24), i32(bytes, 28)],
    score_revision: u32(bytes, 32),
  };
}

function decodeImage(bytes) {
  const h = decodeHeader(bytes);
  if (!h) {
    return null;
  }
  const edition = h.edition === 1 ? "java" : h.edition === 2 ? "bedrock" : null;
  if (!edition) {
    return null;
  }
  const count = u32(bytes, 36);
  const symbols = [];
  const objectives = [];
  const functions = [];
  const chains = [];
  const command_blocks = [];
  const world_blocks = [];
  const containers = [];
  const links = [];
  const tags = [];
  let meta_json = "";

  for (let i = 0; i < count; i++) {
    const off = 40 + i * 16;
    const id = u32(bytes, off);
    const offset = u32(bytes, off + 4);
    const size = u32(bytes, off + 8);
    const n = u32(bytes, off + 12);
    const body = bytes.slice(offset, offset + size);
    let p = 0;
    if (id === SID.SYMB) {
      for (let k = 0; k < n; k++) {
        const sid = u16(body, p);
        const kind = u8(body, p + 2);
        p += 3;
        const short = str16(body, p);
        p += short.n;
        const qualified = str16(body, p);
        p += qualified.n;
        symbols.push({ id: sid, kind, short: short.s, qualified: qualified.s });
      }
    } else if (id === SID.OBJT) {
      for (let k = 0; k < n; k++) {
        const symb = u16(body, p);
        const dummy_only = u8(body, p + 2);
        p += 3;
        const has = u8(body, p);
        p += 1;
        let display = null;
        if (has === 1) {
          const d = str16(body, p);
          p += d.n;
          display = d.s;
        }
        objectives.push({ symb, dummy_only, display });
      }
    } else if (id === SID.TAGS) {
      for (let k = 0; k < n; k++) {
        tags.push(u16(body, p));
        p += 2;
      }
    } else if (id === SID.FUNC) {
      for (let k = 0; k < n; k++) {
        const symb = u16(body, p);
        p += 2;
        const blen = u32(body, p);
        p += 4;
        const body_s = body.slice(p, p + blen).toString("utf8");
        p += blen;
        functions.push({ symb, path: "", body: body_s });
      }
    } else if (id === SID.CHAI) {
      for (let k = 0; k < n; k++) {
        const name_symb = u16(body, p);
        p += 2;
        const layout = u8(body, p);
        const facing = u8(body, p + 1);
        p += 2;
        const origin = [i32(body, p), i32(body, p + 4), i32(body, p + 8)];
        p += 12;
        const length = u16(body, p);
        p += 2;
        const clock = u8(body, p);
        const pack_mode = u8(body, p + 1);
        p += 2;
        chains.push({ name_symb, layout, facing, origin, length, clock, pack_mode });
      }
    } else if (id === SID.CBLK) {
      for (let k = 0; k < n; k++) {
        const x = i32(body, p);
        const y = i32(body, p + 4);
        const z = i32(body, p + 8);
        p += 12;
        const facing = u8(body, p);
        const mode = u8(body, p + 1);
        const flags = u8(body, p + 2);
        p += 3;
        const delay = u32(body, p);
        p += 4;
        const clen = u32(body, p);
        p += 4;
        const command = body.slice(p, p + clen).toString("utf8");
        p += clen;
        command_blocks.push({ x, y, z, facing, mode, flags, delay, command });
      }
    } else if (id === SID.WBLK) {
      const palN = u32(body, 0);
      p = 4;
      const palette = [];
      for (let k = 0; k < palN; k++) {
        const name = str16(body, p);
        p += name.n;
        palette.push(name.s);
      }
      for (let k = 0; k < n; k++) {
        const x = i32(body, p);
        const y = i32(body, p + 4);
        const z = i32(body, p + 8);
        p += 12;
        const idx = u16(body, p);
        p += 2;
        p += 4;
        world_blocks.push({ x, y, z, block: palette[idx] || "air" });
      }
    } else if (id === SID.CONT) {
      for (let k = 0; k < n; k++) {
        const x = i32(body, p);
        const y = i32(body, p + 4);
        const z = i32(body, p + 8);
        p += 12;
        const block_id = u16(body, p);
        p += 2;
        const facing = u8(body, p);
        p += 1;
        const slotN = u16(body, p);
        p += 2;
        const slots = [];
        for (let s = 0; s < slotN; s++) {
          const slot = u8(body, p);
          p += 1;
          const item_id = u16(body, p);
          p += 2;
          const count = u16(body, p);
          p += 2;
          p += 4;
          const nlen = u32(body, p);
          p += 4;
          const nbt = body.slice(p, p + nlen).toString("utf8");
          p += nlen;
          slots.push({ slot, item: internName(symbols, item_id), count, nbt });
        }
        containers.push({
          x,
          y,
          z,
          block: internName(symbols, block_id),
          facing,
          slots,
        });
      }
    } else if (id === SID.LINK) {
      for (let k = 0; k < n; k++) {
        links.push({
          from_chain: u16(body, p),
          from_label: u16(body, p + 2),
          to_chain: u16(body, p + 4),
          kind: u8(body, p + 6),
        });
        p += 7;
      }
    } else if (id === SID.META) {
      const len = u32(body, 0);
      meta_json = body.slice(4, 4 + len).toString("utf8");
    }
  }

  for (const f of functions) {
    if (!f.path) {
      const s = symbols.find((x) => x.id === f.symb);
      f.path = s ? s.short : "";
    }
  }

  return {
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
    meta_json,
  };
}

function parseFills(meta) {
  if (!meta) {
    return [];
  }
  try {
    const obj = typeof meta === "string" ? JSON.parse(meta) : meta;
    if (!obj || !Array.isArray(obj.fills)) {
      return [];
    }
    return obj.fills
      .map((f) => ({
        from: Array.isArray(f.from) ? f.from.map(Number) : [0, 0, 0],
        to: Array.isArray(f.to) ? f.to.map(Number) : [0, 0, 0],
        block: f.block || "",
        replace: f.replace || "",
      }))
      .filter((f) => f.block);
  } catch {
    return parseFillsLoose(meta);
  }
}

function parseFillsLoose(meta) {
  const idx = meta.indexOf('"fills"');
  if (idx < 0) {
    return [];
  }
  const start = meta.indexOf("[", idx);
  if (start < 0) {
    return [];
  }
  let depth = 0;
  let end = -1;
  for (let i = start; i < meta.length; i++) {
    if (meta[i] === "[") {
      depth++;
    } else if (meta[i] === "]") {
      depth--;
      if (depth === 0) {
        end = i;
        break;
      }
    }
  }
  if (end < 0) {
    return [];
  }
  try {
    return JSON.parse(meta.slice(start, end + 1)).map((f) => ({
      from: f.from || [0, 0, 0],
      to: f.to || [0, 0, 0],
      block: f.block || "",
      replace: f.replace || "",
    }));
  } catch {
    return [];
  }
}

function jsonStr(obj, key) {
  const pat = `"${key}":"`;
  const i = obj.indexOf(pat);
  if (i < 0) {
    return undefined;
  }
  const rest = obj.slice(i + pat.length);
  const e = rest.indexOf('"');
  return e < 0 ? undefined : rest.slice(0, e);
}

function jsonI3(obj, key) {
  const pat = `"${key}":[`;
  const i = obj.indexOf(pat);
  if (i < 0) {
    return null;
  }
  const rest = obj.slice(i + pat.length);
  const e = rest.indexOf("]");
  const nums = rest
    .slice(0, e)
    .split(",")
    .map((s) => parseInt(s.trim(), 10))
    .filter((n) => !Number.isNaN(n));
  return nums.length === 3 ? nums : null;
}

module.exports = { decodeHeader, decodeImage, parseFills };
