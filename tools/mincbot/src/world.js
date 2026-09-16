//! Sparse voxel world — the headless Java “game” MincBot imports into.

const { normalizeBlock, commandBlockName } = require("./blocks");
const { parseFills } = require("./mincb");

function key(x, y, z) {
  return `${x},${y},${z}`;
}

class VoxelWorld {
  constructor() {
    this.blocks = new Map();
  }

  set(x, y, z, name, extra) {
    const n = normalizeBlock(name);
    if (n === "air") {
      this.blocks.delete(key(x, y, z));
      return;
    }
    this.blocks.set(key(x, y, z), { name: n, extra: extra || null });
  }

  get(x, y, z) {
    return this.blocks.get(key(x, y, z)) || null;
  }

  has(x, y, z) {
    return this.blocks.has(key(x, y, z));
  }

  fill(from, to, name, extra) {
    const x0 = Math.min(from[0], to[0]);
    const x1 = Math.max(from[0], to[0]);
    const y0 = Math.min(from[1], to[1]);
    const y1 = Math.max(from[1], to[1]);
    const z0 = Math.min(from[2], to[2]);
    const z1 = Math.max(from[2], to[2]);
    for (let y = y0; y <= y1; y++) {
      for (let z = z0; z <= z1; z++) {
        for (let x = x0; x <= x1; x++) {
          this.set(x, y, z, name, extra);
        }
      }
    }
  }

  count(name) {
    const n = normalizeBlock(name);
    let c = 0;
    for (const b of this.blocks.values()) {
      if (b.name === n) {
        c++;
      }
    }
    return c;
  }

  bounds() {
    if (this.blocks.size === 0) {
      return { min: [0, 0, 0], max: [0, 0, 0] };
    }
    let minX = Infinity;
    let minY = Infinity;
    let minZ = Infinity;
    let maxX = -Infinity;
    let maxY = -Infinity;
    let maxZ = -Infinity;
    for (const k of this.blocks.keys()) {
      const [x, y, z] = k.split(",").map(Number);
      if (x < minX) minX = x;
      if (y < minY) minY = y;
      if (z < minZ) minZ = z;
      if (x > maxX) maxX = x;
      if (y > maxY) maxY = y;
      if (z > maxZ) maxZ = z;
    }
    return { min: [minX, minY, minZ], max: [maxX, maxY, maxZ] };
  }

  entries() {
    const out = [];
    for (const [k, b] of this.blocks) {
      const [x, y, z] = k.split(",").map(Number);
      out.push({ x, y, z, name: b.name, extra: b.extra });
    }
    return out;
  }
}

function seedGrass(world, pad = 4) {
  const { min, max } = world.bounds();
  const x0 = min[0] - pad;
  const x1 = max[0] + pad;
  const z0 = min[2] - pad;
  const z1 = max[2] + pad;
  const gy = Math.min(min[1] - 1, 61);
  for (let z = z0; z <= z1; z++) {
    for (let x = x0; x <= x1; x++) {
      if (!world.has(x, gy, z)) {
        world.set(x, gy, z, "grass_block");
      }
    }
  }
}

/**
 * Rasterize a decoded MINCB image into voxels.
 * Order matches the placer spec: fills / WBLK, then containers, then CBLK.
 */
function rasterize(image) {
  const world = new VoxelWorld();
  for (const f of parseFills(image.meta_json)) {
    world.fill(f.from, f.to, f.block);
  }
  for (const b of image.world_blocks || []) {
    world.set(b.x, b.y, b.z, b.block);
  }
  for (const c of image.containers || []) {
    world.set(c.x, c.y, c.z, c.block, { kind: "container", ...c });
  }
  for (const b of image.command_blocks || []) {
    world.set(b.x, b.y, b.z, commandBlockName(b.mode), {
      kind: "cblk",
      ...b,
    });
  }
  return world;
}

module.exports = { VoxelWorld, rasterize, seedGrass, key };
