//! FastBuilder / WorldEdit-style chunk importer plan.
//!
//! Provenance (open-source algorithms, specialized for MINCB/MAR):
//! - FastBuilder / PhoenixBuilder: chunk-keyed import, spiral from origin,
//!   fill-first then special blocks.
//! - WorldEdit / FastAsyncWorldEdit: 3D greedy cuboids (expand X, then Z, then Y)
//!   so a platform is one `/fill` instead of thousands of setblocks.
//! - PrismarineJS mineflayer: the live Java bot that executes the plan.
//!
//! Install order (docs/language placer): WBLK/fills → CONT → CBLK.

const { parseFills } = require("./mincb");
const { normalizeBlock, commandBlockName } = require("./blocks");

function chunkOf(x, z) {
  return [x >> 4, z >> 4];
}

function chunkKey(cx, cz) {
  return `${cx},${cz}`;
}

function inChunk(x, z, cx, cz) {
  return x >> 4 === cx && z >> 4 === cz;
}

/** Rings of chunks around the origin chunk (Chebyshev), FastBuilder-style. */
function spiralChunks(ocx, ocz, radius) {
  const out = [[ocx, ocz]];
  for (let r = 1; r <= radius; r++) {
    for (let x = ocx - r; x <= ocx + r; x++) {
      out.push([x, ocz - r]);
    }
    for (let z = ocz - r + 1; z <= ocz + r; z++) {
      out.push([ocx + r, z]);
    }
    for (let x = ocx + r - 1; x >= ocx - r; x--) {
      out.push([x, ocz + r]);
    }
    for (let z = ocz + r - 1; z > ocz - r; z--) {
      out.push([ocx - r, z]);
    }
  }
  return out;
}

function cellCount(from, to) {
  return (
    (Math.abs(to[0] - from[0]) + 1) *
    (Math.abs(to[1] - from[1]) + 1) *
    (Math.abs(to[2] - from[2]) + 1)
  );
}

/**
 * 3D greedy meshing inside one 16×Y×16 chunk.
 * `grid` is Map "x,y,z" → block name. Only terrain (no NBT extras).
 */
function greedyCuboidsInChunk(grid, cx, cz) {
  const visited = new Set();
  const cuboids = [];
  const keys = [...grid.keys()].sort();
  for (const k of keys) {
    if (visited.has(k)) {
      continue;
    }
    const [x, y, z] = k.split(",").map(Number);
    if (!inChunk(x, z, cx, cz)) {
      continue;
    }
    const block = grid.get(k);
    if (!block) {
      continue;
    }
    let x2 = x;
    while (
      inChunk(x2 + 1, z, cx, cz) &&
      grid.get(`${x2 + 1},${y},${z}`) === block &&
      !visited.has(`${x2 + 1},${y},${z}`)
    ) {
      x2++;
    }
    let z2 = z;
    expandZ: while (true) {
      const nz = z2 + 1;
      if (!inChunk(x, nz, cx, cz)) {
        break;
      }
      for (let xi = x; xi <= x2; xi++) {
        const kk = `${xi},${y},${nz}`;
        if (grid.get(kk) !== block || visited.has(kk)) {
          break expandZ;
        }
      }
      z2 = nz;
    }
    let y2 = y;
    expandY: while (true) {
      const ny = y2 + 1;
      for (let zi = z; zi <= z2; zi++) {
        for (let xi = x; xi <= x2; xi++) {
          const kk = `${xi},${ny},${zi}`;
          if (grid.get(kk) !== block || visited.has(kk)) {
            break expandY;
          }
        }
      }
      y2 = ny;
    }
    for (let yi = y; yi <= y2; yi++) {
      for (let zi = z; zi <= z2; zi++) {
        for (let xi = x; xi <= x2; xi++) {
          visited.add(`${xi},${yi},${zi}`);
        }
      }
    }
    cuboids.push({
      kind: "fill",
      from: [x, y, z],
      to: [x2, y2, z2],
      block,
      chunk: [cx, cz],
      cells: cellCount([x, y, z], [x2, y2, z2]),
    });
  }
  return cuboids;
}

function terrainGrid(image) {
  const grid = new Map();
  const put = (x, y, z, name) => {
    const n = normalizeBlock(name);
    if (n === "air") {
      grid.delete(`${x},${y},${z}`);
    } else {
      grid.set(`${x},${y},${z}`, n);
    }
  };
  for (const f of parseFills(image.meta_json)) {
    const x0 = Math.min(f.from[0], f.to[0]);
    const x1 = Math.max(f.from[0], f.to[0]);
    const y0 = Math.min(f.from[1], f.to[1]);
    const y1 = Math.max(f.from[1], f.to[1]);
    const z0 = Math.min(f.from[2], f.to[2]);
    const z1 = Math.max(f.from[2], f.to[2]);
    for (let y = y0; y <= y1; y++) {
      for (let z = z0; z <= z1; z++) {
        for (let x = x0; x <= x1; x++) {
          put(x, y, z, f.block);
        }
      }
    }
  }
  for (const b of image.world_blocks || []) {
    put(b.x, b.y, b.z, b.block);
  }
  return grid;
}

function usedChunks(grid, extras) {
  const set = new Set();
  for (const k of grid.keys()) {
    const [x, , z] = k.split(",").map(Number);
    set.add(chunkKey(x >> 4, z >> 4));
  }
  for (const e of extras) {
    set.add(chunkKey(e.x >> 4, e.z >> 4));
  }
  return [...set].map((s) => s.split(",").map(Number));
}

function radiusNeeded(chunks, ocx, ocz) {
  let r = 0;
  for (const [cx, cz] of chunks) {
    r = Math.max(r, Math.max(Math.abs(cx - ocx), Math.abs(cz - ocz)));
  }
  return r;
}

/**
 * Build an ordered import plan: spiral chunks, greedy fills, then chests, then command blocks.
 */
function planImport(image) {
  const origin = image.origin || [0, 64, 0];
  const ocx = origin[0] >> 4;
  const ocz = origin[2] >> 4;
  const grid = terrainGrid(image);
  const containers = image.containers || [];
  const cblks = image.command_blocks || [];
  const extras = [
    ...containers.map((c) => ({ x: c.x, z: c.z })),
    ...cblks.map((c) => ({ x: c.x, z: c.z })),
  ];
  const present = new Set(usedChunks(grid, extras).map(([a, b]) => chunkKey(a, b)));
  const radius = radiusNeeded([...present].map((s) => s.split(",").map(Number)), ocx, ocz);
  const ordered = spiralChunks(ocx, ocz, radius).filter(([cx, cz]) =>
    present.has(chunkKey(cx, cz))
  );

  const ops = [];
  let cuboids = 0;
  let fillCells = 0;
  for (const [cx, cz] of ordered) {
    const chunkOps = greedyCuboidsInChunk(grid, cx, cz);
    cuboids += chunkOps.length;
    for (const op of chunkOps) {
      fillCells += op.cells;
      ops.push(op);
    }
  }
  for (const [cx, cz] of ordered) {
    for (const c of containers) {
      if (inChunk(c.x, c.z, cx, cz)) {
        ops.push({
          kind: "container",
          x: c.x,
          y: c.y,
          z: c.z,
          block: normalizeBlock(c.block),
          facing: c.facing,
          slots: c.slots || [],
          chunk: [cx, cz],
        });
      }
    }
  }
  for (const [cx, cz] of ordered) {
    for (const b of cblks) {
      if (inChunk(b.x, b.z, cx, cz)) {
        ops.push({
          kind: "cblk",
          x: b.x,
          y: b.y,
          z: b.z,
          block: commandBlockName(b.mode),
          facing: b.facing,
          mode: b.mode,
          flags: b.flags,
          delay: b.delay,
          command: b.command,
          chunk: [cx, cz],
        });
      }
    }
  }

  const terrainCells = grid.size;
  return {
    origin,
    originChunk: [ocx, ocz],
    chunks: ordered,
    chunkCount: ordered.length,
    cuboids,
    fillCells,
    terrainCells,
    compression: cuboids === 0 ? 1 : terrainCells / cuboids,
    containerCount: containers.length,
    commandBlockCount: cblks.length,
    ops,
  };
}

module.exports = {
  chunkOf,
  chunkKey,
  spiralChunks,
  greedyCuboidsInChunk,
  planImport,
  cellCount,
};
