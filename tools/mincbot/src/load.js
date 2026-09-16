//! Load a `.mar` (default) or raw `.mincb` into a decoded image + version gate.

const fs = require("fs");
const { openMar, isMar, isMincb } = require("./mar");
const { decodeImage } = require("./mincb");
const { compatible } = require("./version");

function loadArchive(file) {
  const buf = fs.readFileSync(file);
  const kind = isMincb(buf) ? "mincb" : isMar(buf) ? "mar" : "unknown";
  if (kind === "unknown") {
    throw new Error(`${file} is not a .mar ZIP or MINCB binary`);
  }
  const opened = openMar(buf);
  const image = decodeImage(opened.mincb);
  if (!image) {
    throw new Error("invalid MINCB payload");
  }
  if (opened.edition) {
    image.edition = String(opened.edition).toLowerCase();
  }
  if (opened.gameVersion) {
    image.game_version = opened.gameVersion;
  }
  if (opened.pack) {
    image.pack = opened.pack;
  }
  return {
    file,
    kind,
    opened,
    image,
    edition: image.edition,
    gameVersion: image.game_version,
    pack: image.pack,
  };
}

function gateArchive(loaded, serverVersion, opts = {}) {
  return compatible(
    { edition: loaded.edition, gameVersion: loaded.gameVersion },
    serverVersion || loaded.gameVersion,
    opts
  );
}

module.exports = { loadArchive, gateArchive };
