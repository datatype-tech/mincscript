const { test } = require("node:test");
const assert = require("node:assert/strict");
const { isMar, isMincb, openMar } = require("../src/mar");
const { decodeHeader } = require("../src/mincb");

function crc32(buf) {
  let c = ~0;
  for (let i = 0; i < buf.length; i++) {
    c ^= buf[i];
    for (let k = 0; k < 8; k++) {
      c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    }
  }
  return ~c >>> 0;
}

function u16(n) {
  const b = Buffer.alloc(2);
  b.writeUInt16LE(n);
  return b;
}
function u32(n) {
  const b = Buffer.alloc(4);
  b.writeUInt32LE(n);
  return b;
}

function zipStore(files) {
  const locals = [];
  const centrals = [];
  let offset = 0;
  for (const f of files) {
    const name = Buffer.from(f.name, "utf8");
    const data = Buffer.from(f.data);
    const crc = crc32(data);
    const local = Buffer.concat([
      Buffer.from([0x50, 0x4b, 0x03, 0x04]),
      u16(20),
      u16(0),
      u16(0),
      u16(0),
      u16(0),
      u32(crc),
      u32(data.length),
      u32(data.length),
      u16(name.length),
      u16(0),
      name,
      data,
    ]);
    const central = Buffer.concat([
      Buffer.from([0x50, 0x4b, 0x01, 0x02]),
      u16(20),
      u16(20),
      u16(0),
      u16(0),
      u16(0),
      u16(0),
      u32(crc),
      u32(data.length),
      u32(data.length),
      u16(name.length),
      u16(0),
      u16(0),
      u16(0),
      u16(0),
      u32(0),
      u32(offset),
      name,
    ]);
    locals.push(local);
    centrals.push(central);
    offset += local.length;
  }
  const body = Buffer.concat(locals);
  const dir = Buffer.concat(centrals);
  const eocd = Buffer.concat([
    Buffer.from([0x50, 0x4b, 0x05, 0x06]),
    u16(0),
    u16(0),
    u16(files.length),
    u16(files.length),
    u32(dir.length),
    u32(body.length),
    u16(0),
  ]);
  return Buffer.concat([body, dir, eocd]);
}

test("STORE mar exposes pack.mincb and Edition", () => {
  const mincb = Buffer.concat([
    Buffer.from("MINC"),
    Buffer.alloc(36),
  ]);
  mincb.writeUInt16LE(1, 4);
  mincb[8] = 1;
  const mar = zipStore([
    {
      name: "META-INF/MANIFEST.MF",
      data: "Manifest-Version: 1.0\nEdition: java\nGame-Version: 1.21.1\nPack: demo.shrine\n",
    },
    { name: "pack.mincb", data: mincb },
  ]);
  assert.equal(isMar(mar), true);
  assert.equal(isMincb(mar), false);
  const opened = openMar(mar);
  assert.equal(opened.edition, "java");
  assert.equal(opened.gameVersion, "1.21.1");
  assert.equal(opened.mincb.slice(0, 4).toString("ascii"), "MINC");
});

test("raw MINCB is accepted without a zip wrapper", () => {
  const mincb = Buffer.alloc(40);
  mincb.write("MINC", 0);
  mincb.writeUInt16LE(1, 4);
  mincb[8] = 1;
  assert.equal(isMincb(mincb), true);
  const opened = openMar(mincb);
  assert.ok(opened.mincb);
  const h = decodeHeader(mincb);
  assert.ok(h);
});
