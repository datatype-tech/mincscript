//! Vanilla Java block ids, command-block modes, and isometric colors.

const FACING = ["down", "up", "north", "south", "west", "east"];

function normalizeBlock(s) {
  if (!s) {
    return "air";
  }
  let t = String(s).trim();
  const br = t.indexOf("[");
  if (br >= 0) {
    t = t.slice(0, br);
  }
  if (t.startsWith("minecraft:")) {
    t = t.slice(10);
  }
  return t || "air";
}

function facingName(id) {
  return FACING[id] || "up";
}

function commandBlockName(mode) {
  if (mode === 2) {
    return "repeating_command_block";
  }
  if (mode === 1) {
    return "chain_command_block";
  }
  return "command_block";
}

function alwaysActive(flags) {
  return (flags & 2) !== 0;
}

/** Approximate vanilla map colors — enough for a readable headless screenshot. */
const PALETTE = {
  air: null,
  stone: [125, 125, 125],
  stone_bricks: [122, 122, 122],
  grass_block: [79, 164, 53],
  dirt: [134, 96, 67],
  gold_block: [252, 210, 71],
  diamond_block: [74, 237, 209],
  emerald_block: [45, 186, 90],
  beacon: [214, 247, 255],
  glass: [180, 220, 230],
  red_wool: [192, 45, 45],
  orange_wool: [224, 120, 24],
  yellow_wool: [232, 196, 28],
  lime_wool: [112, 192, 28],
  chest: [138, 90, 43],
  command_block: [201, 160, 74],
  chain_command_block: [90, 138, 74],
  repeating_command_block: [122, 60, 176],
  oak_planks: [162, 130, 78],
  iron_block: [220, 220, 220],
  lapis_block: [37, 67, 168],
  coal_block: [18, 18, 18],
  water: [40, 90, 200],
};

function colorOf(name) {
  const n = normalizeBlock(name);
  if (PALETTE[n]) {
    return PALETTE[n];
  }
  if (n.endsWith("_wool")) {
    return [180, 180, 180];
  }
  if (n.endsWith("_command_block") || n === "command_block") {
    return PALETTE.command_block;
  }
  // stable hash so unknown blocks still read as distinct cubes
  let h = 0;
  for (let i = 0; i < n.length; i++) {
    h = (h * 33 + n.charCodeAt(i)) >>> 0;
  }
  return [80 + (h & 127), 80 + ((h >> 7) & 127), 80 + ((h >> 14) & 127)];
}

function isTransparent(name) {
  const n = normalizeBlock(name);
  return n === "air" || n === "glass" || n === "water";
}

module.exports = {
  normalizeBlock,
  facingName,
  commandBlockName,
  alwaysActive,
  colorOf,
  isTransparent,
  PALETTE,
};
