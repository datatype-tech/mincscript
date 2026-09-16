//! STORE-only ZIP / `.mar` reader (JAR layout). No deflate.

function u16(buf, o) {
  return buf[o] | (buf[o + 1] << 8);
}

function u32(buf, o) {
  return (
    buf[o] |
    (buf[o + 1] << 8) |
    (buf[o + 2] << 16) |
    (buf[o + 3] << 24)
  ) >>> 0;
}

const LOCAL = Buffer.from([0x50, 0x4b, 0x03, 0x04]);
const CENTRAL = Buffer.from([0x50, 0x4b, 0x01, 0x02]);
const EOCD = Buffer.from([0x50, 0x4b, 0x05, 0x06]);

function isMar(buf) {
  return buf.length >= 4 && (buf[0] === 0x50 && buf[1] === 0x4b);
}

function isMincb(buf) {
  return buf.length >= 4 && buf.slice(0, 4).toString("ascii") === "MINC";
}

function readZipStore(buf) {
  const files = [];
  let i = 0;
  while (i + 30 <= buf.length) {
    if (buf.slice(i, i + 4).equals(EOCD) || buf.slice(i, i + 4).equals(CENTRAL)) {
      break;
    }
    if (!buf.slice(i, i + 4).equals(LOCAL)) {
      throw new Error("not a zip/mar archive");
    }
    const method = u16(buf, i + 8);
    if (method !== 0) {
      throw new Error("mar uses STORE compression only");
    }
    const size = u32(buf, i + 22);
    const nameLen = u16(buf, i + 26);
    const extra = u16(buf, i + 28);
    const nameAt = i + 30;
    const dataAt = nameAt + nameLen + extra;
    if (dataAt + size > buf.length) {
      throw new Error("truncated mar");
    }
    const name = buf.slice(nameAt, nameAt + nameLen).toString("utf8");
    files.push({ name, data: buf.slice(dataAt, dataAt + size) });
    i = dataAt + size;
  }
  if (files.length === 0) {
    throw new Error("empty mar");
  }
  return files;
}

function manifestField(text, key) {
  const prefix = `${key}: `;
  const line = text.split(/\r?\n/).find((l) => l.startsWith(prefix));
  return line ? line.slice(prefix.length).trim() : undefined;
}

function openMar(buf) {
  if (isMincb(buf)) {
    return { mincb: buf, manifest: "", edition: undefined, gameVersion: undefined, pack: undefined };
  }
  const files = readZipStore(buf);
  const mincb = files.find((f) => f.name === "pack.mincb");
  if (!mincb) {
    throw new Error("mar missing pack.mincb");
  }
  const mf = files.find((f) => f.name === "META-INF/MANIFEST.MF");
  const manifest = mf ? mf.data.toString("utf8") : "";
  return {
    mincb: mincb.data,
    manifest,
    edition: manifestField(manifest, "Edition"),
    gameVersion: manifestField(manifest, "Game-Version"),
    pack: manifestField(manifest, "Pack"),
  };
}

module.exports = { isMar, isMincb, openMar, manifestField };
