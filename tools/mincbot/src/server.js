//! Headless Java mini-server via flying-squid (PrismarineJS).
//! Falls back to the in-memory voxel world if the protocol server cannot boot.

const fs = require("fs");
const os = require("os");
const path = require("path");

const CANDIDATE_VERSIONS = ["1.21.1", "1.20.4", "1.20.1", "1.16.5", "1.8.9"];

function tryCreate(options) {
  const flyingSquid = require("flying-squid");
  const create = flyingSquid.createMCServer || flyingSquid;
  return create(options);
}

function startMiniServer(opts = {}) {
  const port = opts.port || 25565;
  const requested = opts.version || "1.21.1";
  const worldFolder =
    opts.worldFolder || path.join(os.tmpdir(), `mincbot-world-${process.pid}`);
  fs.mkdirSync(worldFolder, { recursive: true });

  const versions = [requested, ...CANDIDATE_VERSIONS.filter((v) => v !== requested)];
  const errors = [];

  return new Promise((resolve) => {
    const tryNext = (i) => {
      if (i >= versions.length) {
        resolve({
          ok: false,
          reason: errors.map((e) => e.message || String(e)).join("; ") || "flying-squid failed",
          port,
        });
        return;
      }
      const version = versions[i];
      let serv;
      try {
        serv = tryCreate({
          port,
          "max-players": 8,
          "online-mode": false,
          logging: false,
          gameMode: 1,
          difficulty: 0,
          worldFolder,
          generation: {
            name: "superflat",
            options: { worldHeight: 80 },
          },
          kickTimeout: 10000,
          plugins: {},
          "view-distance": 6,
          "everybody-op": true,
          "max-entities": 50,
          version,
          motd: "MincBot gold-shrine",
        });
      } catch (e) {
        errors.push(e);
        tryNext(i + 1);
        return;
      }

      let settled = false;
      const finish = (extra = {}) => {
        if (settled) {
          return;
        }
        settled = true;
        clearTimeout(timer);
        resolve({
          ok: true,
          serv,
          version,
          port,
          worldFolder,
          ...extra,
        });
      };

      const timer = setTimeout(() => finish({ assumed: true }), 8000);
      if (serv && typeof serv.on === "function") {
        serv.on("listening", () => finish());
        serv.on("error", (e) => {
          if (settled) {
            return;
          }
          settled = true;
          clearTimeout(timer);
          try {
            stopServer({ serv });
          } catch {
            /* ignore */
          }
          errors.push(e);
          tryNext(i + 1);
        });
      }
    };
    try {
      require.resolve("flying-squid");
    } catch (e) {
      resolve({ ok: false, reason: "flying-squid not installed", port });
      return;
    }
    tryNext(0);
  });
}

function stopServer(handle) {
  if (!handle || !handle.serv) {
    return;
  }
  const serv = handle.serv;
  try {
    if (typeof serv.quit === "function") {
      serv.quit();
    }
  } catch {
    /* ignore */
  }
  try {
    if (serv._server && typeof serv._server.close === "function") {
      serv._server.close();
    }
  } catch {
    /* ignore */
  }
}

module.exports = { startMiniServer, stopServer, CANDIDATE_VERSIONS };
