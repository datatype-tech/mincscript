//! pngjs isometric + orthographic renderer for the headless voxel world.

const fs = require("fs");
const { PNG } = require("pngjs");
const { colorOf, isTransparent } = require("./blocks");

function blend(dst, i, r, g, b, a) {
  if (a >= 255) {
    dst[i] = r;
    dst[i + 1] = g;
    dst[i + 2] = b;
    dst[i + 3] = 255;
    return;
  }
  const aa = a / 255;
  const ia = 1 - aa;
  dst[i] = Math.round(dst[i] * ia + r * aa);
  dst[i + 1] = Math.round(dst[i + 1] * ia + g * aa);
  dst[i + 2] = Math.round(dst[i + 2] * ia + b * aa);
  dst[i + 3] = Math.min(255, dst[i + 3] + a);
}

function setPx(png, x, y, r, g, b, a = 255) {
  x = x | 0;
  y = y | 0;
  if (x < 0 || y < 0 || x >= png.width || y >= png.height) {
    return;
  }
  blend(png.data, (png.width * y + x) << 2, r, g, b, a);
}

function fillTri(png, x1, y1, x2, y2, x3, y3, r, g, b, a = 255) {
  const minX = Math.max(0, Math.min(x1, x2, x3) | 0);
  const maxX = Math.min(png.width - 1, Math.max(x1, x2, x3) | 0);
  const minY = Math.max(0, Math.min(y1, y2, y3) | 0);
  const maxY = Math.min(png.height - 1, Math.max(y1, y2, y3) | 0);
  const area = (x1 - x3) * (y2 - y3) - (x2 - x3) * (y1 - y3);
  if (area === 0) {
    return;
  }
  for (let y = minY; y <= maxY; y++) {
    for (let x = minX; x <= maxX; x++) {
      const w0 = (x - x3) * (y2 - y3) - (x2 - x3) * (y - y3);
      const w1 = (x - x1) * (y3 - y1) - (x3 - x1) * (y - y1);
      const w2 = (x - x2) * (y1 - y2) - (x1 - x2) * (y - y2);
      if (w0 >= 0 && w1 >= 0 && w2 >= 0) {
        setPx(png, x, y, r, g, b, a);
      } else if (w0 <= 0 && w1 <= 0 && w2 <= 0) {
        setPx(png, x, y, r, g, b, a);
      }
    }
  }
}

function fillQuad(png, pts, r, g, b, a = 255) {
  fillTri(png, pts[0][0], pts[0][1], pts[1][0], pts[1][1], pts[2][0], pts[2][1], r, g, b, a);
  fillTri(png, pts[0][0], pts[0][1], pts[2][0], pts[2][1], pts[3][0], pts[3][1], r, g, b, a);
}

function shade(rgb, k) {
  return [
    Math.max(0, Math.min(255, Math.round(rgb[0] * k))),
    Math.max(0, Math.min(255, Math.round(rgb[1] * k))),
    Math.max(0, Math.min(255, Math.round(rgb[2] * k))),
  ];
}

function drawCube(png, sx, sy, tw, th, rgb, alpha) {
  const hw = tw / 2;
  const hh = th / 4;
  const top = [
    [sx, sy - th / 2],
    [sx + hw, sy - hh],
    [sx, sy],
    [sx - hw, sy - hh],
  ];
  const left = [
    [sx - hw, sy - hh],
    [sx, sy],
    [sx, sy + th / 2],
    [sx - hw, sy + hh],
  ];
  const right = [
    [sx, sy],
    [sx + hw, sy - hh],
    [sx + hw, sy + hh],
    [sx, sy + th / 2],
  ];
  const [tr, tg, tb] = shade(rgb, 1.12);
  const [lr, lg, lb] = shade(rgb, 0.72);
  const [rr, rg, rb] = shade(rgb, 0.88);
  fillQuad(png, top, tr, tg, tb, alpha);
  fillQuad(png, left, lr, lg, lb, alpha);
  fillQuad(png, right, rr, rg, rb, alpha);
}

function skyFill(png) {
  for (let y = 0; y < png.height; y++) {
    const t = y / png.height;
    const r = Math.round(118 + (210 - 118) * t);
    const g = Math.round(168 + (230 - 168) * t);
    const b = Math.round(228 + (245 - 228) * t);
    for (let x = 0; x < png.width; x++) {
      const i = (png.width * y + x) << 2;
      png.data[i] = r;
      png.data[i + 1] = g;
      png.data[i + 2] = b;
      png.data[i + 3] = 255;
    }
  }
}

// 5x7 caps for overlay labels.
const FONT = {
  " ": [],
  A: ["01110", "10001", "10001", "11111", "10001", "10001", "10001"],
  B: ["11110", "10001", "11110", "10001", "10001", "10001", "11110"],
  C: ["01110", "10001", "10000", "10000", "10000", "10001", "01110"],
  D: ["11110", "10001", "10001", "10001", "10001", "10001", "11110"],
  E: ["11111", "10000", "11110", "10000", "10000", "10000", "11111"],
  F: ["11111", "10000", "11110", "10000", "10000", "10000", "10000"],
  G: ["01110", "10001", "10000", "10111", "10001", "10001", "01110"],
  H: ["10001", "10001", "11111", "10001", "10001", "10001", "10001"],
  I: ["11111", "00100", "00100", "00100", "00100", "00100", "11111"],
  J: ["00111", "00001", "00001", "00001", "00001", "10001", "01110"],
  K: ["10001", "10010", "11100", "10010", "10001", "10001", "10001"],
  L: ["10000", "10000", "10000", "10000", "10000", "10000", "11111"],
  M: ["10001", "11011", "10101", "10101", "10001", "10001", "10001"],
  N: ["10001", "11001", "10101", "10011", "10001", "10001", "10001"],
  O: ["01110", "10001", "10001", "10001", "10001", "10001", "01110"],
  P: ["11110", "10001", "10001", "11110", "10000", "10000", "10000"],
  R: ["11110", "10001", "10001", "11110", "10100", "10010", "10001"],
  S: ["01111", "10000", "10000", "01110", "00001", "00001", "11110"],
  T: ["11111", "00100", "00100", "00100", "00100", "00100", "00100"],
  U: ["10001", "10001", "10001", "10001", "10001", "10001", "01110"],
  V: ["10001", "10001", "10001", "10001", "10001", "01010", "00100"],
  W: ["10001", "10001", "10001", "10101", "10101", "11011", "10001"],
  Y: ["10001", "10001", "01010", "00100", "00100", "00100", "00100"],
  ".": ["00000", "00000", "00000", "00000", "00000", "00100", "00100"],
  "-": ["00000", "00000", "00000", "11111", "00000", "00000", "00000"],
  ":": ["00000", "00100", "00100", "00000", "00100", "00100", "00000"],
  "0": ["01110", "10001", "10011", "10101", "11001", "10001", "01110"],
  "1": ["00100", "01100", "00100", "00100", "00100", "00100", "01110"],
  "2": ["01110", "10001", "00001", "00010", "00100", "01000", "11111"],
  "3": ["11110", "00001", "00001", "01110", "00001", "00001", "11110"],
  "4": ["00010", "00110", "01010", "10010", "11111", "00010", "00010"],
  "5": ["11111", "10000", "11110", "00001", "00001", "10001", "01110"],
  "6": ["01110", "10000", "11110", "10001", "10001", "10001", "01110"],
  "7": ["11111", "00001", "00010", "00100", "00100", "00100", "00100"],
  "8": ["01110", "10001", "10001", "01110", "10001", "10001", "01110"],
  "9": ["01110", "10001", "10001", "01111", "00001", "00001", "01110"],
};

function drawText(png, x, y, text, rgb = [20, 24, 32], scale = 2) {
  let cx = x;
  for (const ch of text.toUpperCase()) {
    const glyph = FONT[ch];
    if (!glyph) {
      cx += 6 * scale;
      continue;
    }
    for (let gy = 0; gy < 7; gy++) {
      for (let gx = 0; gx < 5; gx++) {
        if (glyph[gy][gx] === "1") {
          for (let oy = 0; oy < scale; oy++) {
            for (let ox = 0; ox < scale; ox++) {
              setPx(png, cx + gx * scale + ox, y + gy * scale + oy, rgb[0], rgb[1], rgb[2]);
            }
          }
        }
      }
    }
    cx += 6 * scale;
  }
}

function renderIsometric(world, opts = {}) {
  const tw = opts.tw || 18;
  const th = opts.th || 18;
  const pad = opts.pad || 48;
  const caption = opts.caption || "MINCBOT  GOLD SHRINE";
  let { min, max } = world.bounds();
  const blocks = world.entries();
  if (blocks.length === 0) {
    min = [0, 0, 0];
    max = [0, 0, 0];
  }
  const iso = (x, y, z) => {
    const sx = (x - z) * (tw / 2);
    const sy = (x + z) * (th / 4) - y * (th / 2);
    return [sx, sy];
  };
  let minSx = Infinity;
  let minSy = Infinity;
  let maxSx = -Infinity;
  let maxSy = -Infinity;
  for (const b of blocks) {
    const [sx, sy] = iso(b.x, b.y, b.z);
    minSx = Math.min(minSx, sx - tw, sx + tw);
    maxSx = Math.max(maxSx, sx + tw);
    minSy = Math.min(minSy, sy - th);
    maxSy = Math.max(maxSy, sy + th);
  }
  const width = Math.max(320, Math.ceil(maxSx - minSx + pad * 2));
  const height = Math.max(240, Math.ceil(maxSy - minSy + pad * 2 + 28));
  const png = new PNG({ width, height });
  skyFill(png);
  const ox = pad - minSx;
  const oy = pad + 22 - minSy;
  const sorted = blocks.slice().sort((a, b) => {
    const da = a.x + a.z;
    const db = b.x + b.z;
    if (da !== db) {
      return da - db;
    }
    return a.y - b.y;
  });
  for (const b of sorted) {
    const rgb = colorOf(b.name);
    if (!rgb) {
      continue;
    }
    const [sx, sy] = iso(b.x, b.y, b.z);
    const alpha = isTransparent(b.name) ? 110 : 255;
    drawCube(png, sx + ox, sy + oy, tw, th, rgb, alpha);
  }
  // beacon beam
  const beam = blocks.find((b) => b.name === "beacon");
  if (beam) {
    for (let y = beam.y + 1; y <= beam.y + 18; y++) {
      const [sx, sy] = iso(beam.x, y, beam.z);
      drawCube(png, sx + ox, sy + oy, tw, th, [200, 240, 255], 70);
    }
  }
  drawText(png, 12, 8, caption, [18, 28, 48], 2);
  return png;
}

function renderFront(world, opts = {}) {
  const tile = opts.tile || 14;
  const zFocus = opts.z != null ? opts.z : 1;
  const caption = opts.caption || "MINC LETTERS";
  const blocks = world.entries().filter((b) => Math.abs(b.z - zFocus) <= 1);
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  for (const b of blocks) {
    minX = Math.min(minX, b.x);
    minY = Math.min(minY, b.y);
    maxX = Math.max(maxX, b.x);
    maxY = Math.max(maxY, b.y);
  }
  if (!Number.isFinite(minX)) {
    minX = 0;
    minY = 0;
    maxX = 8;
    maxY = 8;
  }
  const width = (maxX - minX + 3) * tile + 24;
  const height = (maxY - minY + 3) * tile + 40;
  const png = new PNG({ width, height });
  skyFill(png);
  const sorted = blocks.slice().sort((a, b) => a.z - b.z || a.y - b.y);
  for (const b of sorted) {
    const rgb = colorOf(b.name);
    if (!rgb) {
      continue;
    }
    const px = 12 + (b.x - minX) * tile;
    const py = 28 + (maxY - b.y) * tile;
    const k = b.z === zFocus ? 1 : 0.55;
    const [r, g, bb] = shade(rgb, k);
    for (let y = 0; y < tile - 1; y++) {
      for (let x = 0; x < tile - 1; x++) {
        setPx(png, px + x, py + y, r, g, bb);
      }
    }
  }
  drawText(png, 12, 6, caption, [18, 28, 48], 2);
  return png;
}

function writePng(png, file) {
  fs.mkdirSync(require("path").dirname(file), { recursive: true });
  fs.writeFileSync(file, PNG.sync.write(png));
  return file;
}

module.exports = { renderIsometric, renderFront, writePng, drawText };
