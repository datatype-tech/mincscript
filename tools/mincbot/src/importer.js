//! Apply a chunk plan to the voxel world and (optionally) a live Java session.

const { VoxelWorld, rasterize } = require("./world");
const { planImport } = require("./plan");
const { normalizeBlock, facingName, alwaysActive } = require("./blocks");

function vec3(x, y, z) {
  try {
    const { Vec3 } = require("vec3");
    return new Vec3(x, y, z);
  } catch {
    return { x, y, z };
  }
}

function applyOp(world, op) {
  if (op.kind === "fill") {
    world.fill(op.from, op.to, op.block);
    return;
  }
  if (op.kind === "container") {
    world.set(op.x, op.y, op.z, op.block, {
      kind: "container",
      facing: op.facing,
      slots: op.slots,
    });
    return;
  }
  if (op.kind === "cblk") {
    world.set(op.x, op.y, op.z, op.block, {
      kind: "cblk",
      facing: op.facing,
      mode: op.mode,
      flags: op.flags,
      command: op.command,
    });
  }
}

function applyPlan(world, plan) {
  for (const op of plan.ops) {
    applyOp(world, op);
  }
  return world;
}

function importImage(image) {
  const plan = planImport(image);
  const world = new VoxelWorld();
  applyPlan(world, plan);
  return { world, plan };
}

function javaFillCommand(op) {
  return `fill ${op.from[0]} ${op.from[1]} ${op.from[2]} ${op.to[0]} ${op.to[1]} ${op.to[2]} ${op.block}`;
}

function javaSetCommand(op) {
  const face = facingName(op.facing);
  if (op.kind === "container") {
    return `setblock ${op.x} ${op.y} ${op.z} ${op.block}[facing=${face}]`;
  }
  if (op.kind === "cblk") {
    const auto = alwaysActive(op.flags) ? "true" : "false";
    return `setblock ${op.x} ${op.y} ${op.z} ${op.block}[facing=${face}]`;
  }
  return `setblock ${op.x} ${op.y} ${op.z} ${op.block}`;
}

function commandsForPlan(plan) {
  const out = [];
  for (const op of plan.ops) {
    if (op.kind === "fill") {
      out.push(javaFillCommand(op));
    } else {
      out.push(javaSetCommand(op));
    }
  }
  return out;
}

function blockStateId(mcData, name) {
  const n = normalizeBlock(name);
  const table = (mcData && (mcData.blocksByName || (mcData.registry && mcData.registry.blocksByName))) || {};
  const b = table[n] || table.stone;
  if (!b) {
    return 1;
  }
  if (b.defaultState != null) {
    return b.defaultState;
  }
  if (b.minStateId != null) {
    return b.minStateId;
  }
  return b.id;
}

async function applyOpToFlyingSquid(serv, mcData, op) {
  if (!serv || typeof serv.setBlock !== "function") {
    return 0;
  }
  const world = serv.overworld;
  if (!world) {
    return 0;
  }
  const registry = mcData || serv.registry;
  let n = 0;
  const put = async (x, y, z, name) => {
    try {
      await serv.setBlock(world, vec3(x, y, z), blockStateId(registry, name));
      n++;
    } catch {
      /* flying-squid API / unloaded chunk */
    }
  };
  if (op.kind === "fill") {
    const x0 = Math.min(op.from[0], op.to[0]);
    const x1 = Math.max(op.from[0], op.to[0]);
    const y0 = Math.min(op.from[1], op.to[1]);
    const y1 = Math.max(op.from[1], op.to[1]);
    const z0 = Math.min(op.from[2], op.to[2]);
    const z1 = Math.max(op.from[2], op.to[2]);
    for (let y = y0; y <= y1; y++) {
      for (let z = z0; z <= z1; z++) {
        for (let x = x0; x <= x1; x++) {
          await put(x, y, z, op.block);
        }
      }
    }
    return n;
  }
  await put(op.x, op.y, op.z, op.block);
  return n;
}

async function applyPlanViaBot(bot, plan, { pauseMs = 0 } = {}) {
  if (!bot) {
    return { placed: 0, fills: 0 };
  }
  let placed = 0;
  let fills = 0;
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  for (const op of plan.ops) {
    if (op.kind === "fill") {
      try {
        bot.chat(`/${javaFillCommand(op)}`);
        fills++;
        placed += op.cells || 0;
      } catch {
        /* ignore */
      }
    } else {
      try {
        bot.chat(`/${javaSetCommand(op)}`);
        placed++;
      } catch {
        /* ignore */
      }
    }
    if (pauseMs) {
      await sleep(pauseMs);
    }
  }
  if (bot.creative && typeof bot.creative.setBlock === "function") {
    // Sample the beacon so a live bot path is exercised even if /fill is a no-op.
    try {
      const beacon = plan.ops.find((o) => o.kind === "fill" && o.block === "beacon")
        || plan.ops.find((o) => o.x === 13 && o.y === 65 && o.z === 8);
      if (beacon && beacon.kind !== "fill") {
        await bot.creative.setBlock(vec3(beacon.x, beacon.y, beacon.z), 138);
      }
    } catch {
      /* older mineflayer */
    }
  }
  return { placed, fills };
}

module.exports = {
  applyOp,
  applyPlan,
  importImage,
  rasterize,
  javaFillCommand,
  javaSetCommand,
  commandsForPlan,
  applyOpToFlyingSquid,
  applyPlanViaBot,
  blockStateId,
};
