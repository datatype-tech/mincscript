const { test } = require("node:test");
const assert = require("node:assert/strict");
const { greedyCuboidsInChunk, spiralChunks, planImport } = require("../src/plan");

test("greedy cuboid merges a 4x1x4 gold pad in one chunk", () => {
  const grid = new Map();
  for (let z = 0; z < 4; z++) {
    for (let x = 0; x < 4; x++) {
      grid.set(`${x},64,${z}`, "gold_block");
    }
  }
  const cuboids = greedyCuboidsInChunk(grid, 0, 0);
  assert.equal(cuboids.length, 1);
  assert.deepEqual(cuboids[0].from, [0, 64, 0]);
  assert.deepEqual(cuboids[0].to, [3, 64, 3]);
  assert.equal(cuboids[0].cells, 16);
});

test("greedy cuboids do not cross chunk borders", () => {
  const grid = new Map();
  for (let x = 14; x <= 17; x++) {
    grid.set(`${x},70,0`, "stone_bricks");
  }
  const c0 = greedyCuboidsInChunk(grid, 0, 0);
  const c1 = greedyCuboidsInChunk(grid, 1, 0);
  assert.equal(c0.length, 1);
  assert.deepEqual(c0[0].to, [15, 70, 0]);
  assert.equal(c1.length, 1);
  assert.deepEqual(c1[0].from, [16, 70, 0]);
});

test("spiral starts at origin chunk", () => {
  const s = spiralChunks(3, 5, 1);
  assert.deepEqual(s[0], [3, 5]);
  assert.ok(s.some(([x, z]) => x === 4 && z === 5));
});

test("planImport fills then containers then command blocks", () => {
  const image = {
    origin: [0, 64, 0],
    meta_json: `{"fills":[{"from":[0,62,0],"to":[2,62,2],"block":"gold_block","replace":""}]}`,
    world_blocks: [{ x: 0, y: 64, z: 0, block: "diamond_block" }],
    containers: [{ x: 1, y: 64, z: 1, block: "chest", facing: 3, slots: [] }],
    command_blocks: [
      { x: 2, y: 64, z: 0, facing: 1, mode: 2, flags: 2, delay: 0, command: "say hi" },
    ],
  };
  const plan = planImport(image);
  assert.ok(plan.chunkCount >= 1);
  assert.ok(plan.cuboids >= 1);
  assert.ok(plan.cuboids < plan.terrainCells);
  const kinds = plan.ops.map((o) => o.kind);
  const fillAt = kinds.lastIndexOf("fill");
  const contAt = kinds.indexOf("container");
  const cblkAt = kinds.indexOf("cblk");
  assert.ok(fillAt < contAt);
  assert.ok(contAt < cblkAt);
});
