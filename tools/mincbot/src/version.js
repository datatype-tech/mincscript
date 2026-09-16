//! Target-edition / game-version gate. Java only (MincBot does not speak Bedrock).

function parseVer(s) {
  if (!s) {
    return null;
  }
  const m = String(s).trim().match(/^(\d+)\.(\d+)(?:\.(\d+))?/);
  if (!m) {
    return null;
  }
  return { maj: +m[1], min: +m[2], pat: m[3] ? +m[3] : 0, raw: s };
}

function compatible({ edition, gameVersion }, serverVersion, opts = {}) {
  if (!edition) {
    return { ok: false, reason: "MAR/MINCB missing Edition" };
  }
  if (edition !== "java") {
    return {
      ok: false,
      reason: `MincBot is Java-only; archive targets ${edition}`,
    };
  }
  const want = parseVer(gameVersion);
  const have = parseVer(serverVersion);
  if (!want || !have) {
    return { ok: false, reason: "cannot parse Game-Version" };
  }
  if (opts.strict) {
    if (want.raw !== have.raw && !(want.maj === have.maj && want.min === have.min && want.pat === have.pat)) {
      return {
        ok: false,
        reason: `strict version mismatch: mar ${want.raw} vs server ${have.raw}`,
      };
    }
    return { ok: true };
  }
  if (want.maj !== have.maj || want.min !== have.min) {
    return {
      ok: false,
      reason: `game version family mismatch: mar ${want.maj}.${want.min} vs server ${have.maj}.${have.min}`,
    };
  }
  return { ok: true };
}

module.exports = { parseVer, compatible };
