//! Structural checks on the imported voxel world.

function at(world, x, y, z) {
  const b = world.get(x, y, z);
  return b ? b.name : "air";
}

function expect(checks, world, x, y, z, name) {
  const got = at(world, x, y, z);
  const ok = got === name;
  checks.push({ x, y, z, expect: name, got, ok });
  return ok;
}

function verifyGoldShrine(world, opts = {}) {
  const letterMin = opts.letterMin != null ? opts.letterMin : 10;
  const checks = [];
  expect(checks, world, 1, 62, 1, "stone_bricks");
  expect(checks, world, 1, 63, 1, "gold_block");
  expect(checks, world, 13, 63, 8, "diamond_block");
  expect(checks, world, 13, 64, 8, "emerald_block");
  expect(checks, world, 13, 65, 8, "beacon");
  expect(checks, world, 13, 66, 8, "glass");
  expect(checks, world, 0, 64, 0, "diamond_block");
  expect(checks, world, 26, 64, 16, "diamond_block");
  expect(checks, world, 2, 69, 1, "red_wool");
  expect(checks, world, 10, 69, 1, "orange_wool");
  expect(checks, world, 14, 69, 1, "yellow_wool");
  expect(checks, world, 21, 69, 1, "lime_wool");
  expect(checks, world, 13, 64, 14, "chest");
  const cb = at(world, 26, 64, 8);
  const cbOk = cb.includes("command_block");
  checks.push({ x: 26, y: 64, z: 8, expect: "command_block*", got: cb, ok: cbOk });

  const counts = {
    gold_block: world.count("gold_block"),
    stone_bricks: world.count("stone_bricks"),
    red_wool: world.count("red_wool"),
    orange_wool: world.count("orange_wool"),
    yellow_wool: world.count("yellow_wool"),
    lime_wool: world.count("lime_wool"),
    beacon: world.count("beacon"),
    chest: world.count("chest"),
  };
  const countChecks = [
    { name: "gold_block", min: 400, got: counts.gold_block },
    { name: "stone_bricks", min: 400, got: counts.stone_bricks },
    { name: "red_wool", min: letterMin, got: counts.red_wool },
    { name: "orange_wool", min: letterMin, got: counts.orange_wool },
    { name: "yellow_wool", min: letterMin, got: counts.yellow_wool },
    { name: "lime_wool", min: letterMin, got: counts.lime_wool },
    { name: "beacon", min: 1, got: counts.beacon },
    { name: "chest", min: 1, got: counts.chest },
  ].map((c) => ({ ...c, ok: c.got >= c.min }));

  const failed = [...checks, ...countChecks].filter((c) => !c.ok);
  return {
    ok: failed.length === 0,
    failed,
    checks,
    countChecks,
    counts,
    cells: world.blocks.size,
  };
}

function verifyPlan(plan) {
  const checks = [];
  const okChunks = plan.chunkCount >= 1;
  checks.push({ name: "chunked", ok: okChunks, got: plan.chunkCount });
  const okCuboids = plan.cuboids >= 1 && plan.cuboids < plan.terrainCells;
  checks.push({
    name: "greedy-cuboids",
    ok: okCuboids,
    cuboids: plan.cuboids,
    cells: plan.terrainCells,
    compression: plan.compression,
  });
  const order = plan.ops.map((o) => o.kind);
  const firstCblk = order.indexOf("cblk");
  const lastFill = order.lastIndexOf("fill");
  const lastCont = order.lastIndexOf("container");
  const orderOk =
    (firstCblk === -1 || lastFill === -1 || lastFill < firstCblk) &&
    (lastCont === -1 || firstCblk === -1 || lastCont < firstCblk);
  checks.push({ name: "install-order", ok: orderOk, order: [...new Set(order)] });
  return { ok: checks.every((c) => c.ok), checks, plan: summarizePlan(plan) };
}

function summarizePlan(plan) {
  return {
    chunkCount: plan.chunkCount,
    chunks: plan.chunks,
    cuboids: plan.cuboids,
    terrainCells: plan.terrainCells,
    compression: Number(plan.compression.toFixed(2)),
    containerCount: plan.containerCount,
    commandBlockCount: plan.commandBlockCount,
    ops: plan.ops.length,
  };
}

module.exports = { verifyGoldShrine, verifyPlan, summarizePlan };
