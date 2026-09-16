//! Headless Java mini-server via flying-squid (PrismarineJS).
//! Falls back to the in-memory voxel world if the protocol server cannot boot.

const CANDIDATE_VERSIONS = ["1.21.1", "1.21.4", "1.20.4", "1.16.5"];

function tryCreate(options) {
  const flyingSquid = require("flying-squid");
  return flyingSquid.createMCServer(options);
}

function startMiniServer(opts = {}) {
  const port = opts.port || 25565;
  const requested = opts.version || "1.21.1";
  const versions = [requested, ...CANDIDATE_VERSIONS.filter((v) => v !== requested)];
  const errors = [];

  return new Promise((resolve) => {
    try {
      require.resolve("flying-squid");
    } catch {
      resolve({ ok: false, reason: "flying-squid not installed", port });
      return;
    }

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
          worldFolder: null,
          generation: { name: "superflat", options: {} },
          kickTimeout: 10000,
          plugins: {},
          "view-distance": 4,
          "everybody-op": true,
          "max-entities": 32,
          version,
          motd: "MincBot gold-shrine",
          "player-list-text": {
            header: { text: "MincBot" },
            footer: { text: "gold-shrine" },
          },
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
          ...extra,
        });
      };
      const fail = (e) => {
        if (settled) {
          return;
        }
        settled = true;
        clearTimeout(timer);
        stopServer({ serv });
        errors.push(e);
        tryNext(i + 1);
      };

      const timer = setTimeout(() => finish({ assumed: true }), 12000);
      if (typeof serv.waitForReady === "function") {
        serv.waitForReady(10000).then(() => finish()).catch(fail);
      }
      if (serv && typeof serv.on === "function") {
        serv.on("ready", () => finish());
        serv.on("listening", () => finish());
        serv.on("error", fail);
      }
    };
    tryNext(0);
  });
}

function stopServer(handle) {
  if (!handle || !handle.serv) {
    return;
  }
  const serv = handle.serv;
  try {
    if (typeof serv.destroy === "function") {
      Promise.resolve(serv.destroy()).catch(() => {});
      return;
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
