const { test } = require("node:test");
const assert = require("node:assert/strict");
const { importImage } = require("../src/importer");
const { verifyGoldShrine, verifyPlan } = require("../src/verify");
const { renderIsometric, renderFront } = require("../src/render");
const { seedGrass } = require("../src/world");

function shrineImage() {
  return {
    edition: "java",
    game_version: "1.21.1",
    pack: "demo.shrine",
    origin: [0, 64, 0],
    meta_json:
      '{"fills":[{"from":[0,62,0],"to":[26,62,16],"block":"stone_bricks","replace":""},{"from":[0,63,0],"to":[26,63,16],"block":"gold_block","replace":""},{"from":[12,63,7],"to":[14,63,9],"block":"diamond_block","replace":""}]}',
    world_blocks: [
      { x: 13, y: 64, z: 8, block: "emerald_block" },
      { x: 13, y: 65, z: 8, block: "beacon" },
      { x: 13, y: 66, z: 8, block: "glass" },
      { x: 0, y: 64, z: 0, block: "diamond_block" },
      { x: 26, y: 64, z: 0, block: "diamond_block" },
      { x: 0, y: 64, z: 16, block: "diamond_block" },
      { x: 26, y: 64, z: 16, block: "diamond_block" },
      { x: 2, y: 69, z: 1, block: "red_wool" },
      { x: 10, y: 69, z: 1, block: "orange_wool" },
      { x: 14, y: 69, z: 1, block: "yellow_wool" },
      { x: 21, y: 69, z: 1, block: "lime_wool" },
    ],
    containers: [
      {
        x: 13,
        y: 64,
        z: 14,
        block: "chest",
        facing: 3,
        slots: [{ slot: 0, item: "golden_apple", count: 1 }],
      },
    ],
    command_blocks: [
      { x: 26, y: 64, z: 8, facing: 1, mode: 2, flags: 2, delay: 0, command: 'title @a title "MINC"' },
    ],
  };
}

test("chunked import of a shrine-shaped image verifies", () => {
  const image = shrineImage();
  const { world, plan } = importImage(image);
  const planV = verifyPlan(plan);
  assert.equal(planV.ok, true, JSON.stringify(planV));
  const v = verifyGoldShrine(world, { letterMin: 1 });
  assert.equal(v.ok, true, JSON.stringify(v.failed));
  assert.ok(plan.terrainCells > 50, JSON.stringify(planV));
  assert.ok(plan.compression > 2, JSON.stringify(planV));
});

test("isometric and front renders produce non-empty pngs", () => {
  const { world } = importImage(shrineImage());
  seedGrass(world);
  const iso = renderIsometric(world, { caption: "MINCBOT TEST" });
  const front = renderFront(world, { caption: "MINC" });
  assert.ok(iso.width > 100 && iso.height > 80);
  assert.ok(front.width > 40 && front.height > 40);
  const i = (iso.width * 20 + 12) << 2;
  assert.ok(iso.data[i + 3] === 255);
});
