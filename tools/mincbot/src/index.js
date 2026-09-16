#!/usr/bin/env node
//! MincBot — Java-edition MAR importer (mineflayer + flying-squid + voxel world).

const fs = require("fs");
const path = require("path");
const { spawnSync } = require("child_process");
const { loadArchive, gateArchive } = require("./load");
const { planImport } = require("./plan");
const { importImage, applyPlanViaBot, applyOpToFlyingSquid, commandsForPlan } = require("./importer");
const { seedGrass } = require("./world");
const { startMiniServer, stopServer } = require("./server");
const { connectMincBot, disconnect } = require("./bot");
const { renderIsometric, renderFront, writePng: writePngFile } = require("./render");
const { verifyGoldShrine, verifyPlan } = require("./verify");

function repoRoot() {
  return path.resolve(__dirname, "../../..");
}

function parseArgs(argv) {
  const out = { _: [], flags: {} };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a.startsWith("--")) {
      const key = a.slice(2);
      const nxt = argv[i + 1];
      if (!nxt || nxt.startsWith("--")) {
        out.flags[key] = true;
      } else {
        out.flags[key] = nxt;
        i++;
      }
    } else {
      out._.push(a);
    }
  }
  return out;
}

function usage() {
  return `MincBot — Java MAR importer (Bedrock not supported)

  mincbot demo [--out dir] [--port 25565] [--strict] [--no-server]
  mincbot import <file.mar|.mincb> [--version 1.21.1] [--out dir]
  mincbot check <file.mar|.mincb> [--version 1.21.1] [--strict]
  mincbot screenshot <file.mar|.mincb> [--out dir]
`;
}

function ensureGoldShrineMar(root) {
  const mar = path.join(root, "examples/gold-shrine/dist/gold-shrine.mar");
  if (fs.existsSync(mar)) {
    return mar;
  }
  const r = spawnSync(
    "cargo",
    ["run", "--quiet", "--bin", "minc", "--", "build", "examples/gold-shrine", "--out", "examples/gold-shrine/dist"],
    { cwd: root, encoding: "utf8" }
  );
  if (r.status !== 0) {
    throw new Error(`minc build gold-shrine failed:\n${r.stderr || r.stdout}`);
  }
  if (!fs.existsSync(mar)) {
    throw new Error("gold-shrine.mar was not written");
  }
  return mar;
}

function copyArtifact(src, destDir, name) {
  if (!destDir || !fs.existsSync(src)) {
    return null;
  }
  try {
    fs.mkdirSync(destDir, { recursive: true });
    const dest = path.join(destDir, name);
    fs.copyFileSync(src, dest);
    return dest;
  } catch {
    return null;
  }
}

async function runImport(file, flags) {
  const loaded = loadArchive(file);
  const targetVersion = flags.version || loaded.gameVersion || "1.21.1";
  const gate = gateArchive(loaded, targetVersion, { strict: !!flags.strict });
  const plan = planImport(loaded.image);
  const planCheck = verifyPlan(plan);
  const { world } = importImage(loaded.image);
  seedGrass(world);

  const outDir = path.resolve(flags.out || path.join(__dirname, "../out"));
  fs.mkdirSync(outDir, { recursive: true });

  const report = {
    bot: "MincBot",
    file,
    kind: loaded.kind,
    edition: loaded.edition,
    gameVersion: loaded.gameVersion,
    pack: loaded.pack,
    targetVersion,
    gate,
    plan: planCheck,
    server: { ok: false },
    botSession: { ok: false },
    verify: null,
    screenshots: {},
  };

  let serverHandle = null;
  let bot = null;
  if (!flags["no-server"]) {
    try {
      serverHandle = await startMiniServer({
        port: Number(flags.port) || 25565,
        version: targetVersion,
      });
      report.server = {
        ok: !!serverHandle.ok,
        version: serverHandle.version,
        reason: serverHandle.reason || null,
        assumed: !!serverHandle.assumed,
      };
      if (serverHandle.ok) {
        let mcData = null;
        try {
          mcData = require("minecraft-data")(serverHandle.version);
        } catch {
          mcData = null;
        }
        let injected = 0;
        for (const op of plan.ops) {
          injected += applyOpToFlyingSquid(serverHandle.serv, mcData, op);
        }
        report.server.injected = injected;
        try {
          bot = await connectMincBot({
            host: "127.0.0.1",
            port: serverHandle.port,
            version: serverHandle.version,
          });
          report.botSession = { ok: true, username: "MincBot" };
          report.botSession.chat = await applyPlanViaBot(bot, plan, { pauseMs: 0 });
        } catch (e) {
          report.botSession = { ok: false, reason: e.message || String(e) };
        }
      }
    } catch (e) {
      report.server = { ok: false, reason: e.message || String(e) };
    }
  } else {
    report.server = { ok: false, reason: "disabled" };
  }

  const { VoxelWorld } = require("./world");
  const artifactDir = process.env.CURSOR_ARTIFACTS_DIR || "/opt/cursor/artifacts";
  const before = new VoxelWorld();
  before.fill([-4, 61, -4], [30, 61, 20], "grass_block");
  const beforePng = renderIsometric(before, { caption: "MINCBOT  EMPTY WORLD" });
  report.screenshots.before = writePngFile(
    beforePng,
    path.join(outDir, "mincbot_shrine_before.png")
  );
  const iso = renderIsometric(world, {
    caption: `MINCBOT  ${loaded.pack || "PACK"}  ${loaded.gameVersion}`,
  });
  const front = renderFront(world, { caption: "MINC  WOOL LETTERS" });
  report.screenshots.iso = writePngFile(iso, path.join(outDir, "mincbot_shrine_isometric.png"));
  report.screenshots.front = writePngFile(front, path.join(outDir, "mincbot_shrine_letters.png"));
  copyArtifact(report.screenshots.before, artifactDir, "mincbot_shrine_before.png");
  copyArtifact(report.screenshots.iso, artifactDir, "mincbot_shrine_isometric.png");
  copyArtifact(report.screenshots.front, artifactDir, "mincbot_shrine_letters.png");

  // empty-ish grass pad “before” is less useful after seedGrass; render a second
  // isometric cropped tighter after import — already have iso.
  const shrine = verifyGoldShrine(world);
  report.verify = shrine;
  report.ok = gate.ok && planCheck.ok && shrine.ok;

  const reportPath = path.join(outDir, "mincbot_import_report.json");
  fs.writeFileSync(reportPath, JSON.stringify(report, null, 2));
  report.screenshots.report = reportPath;

  const cmds = commandsForPlan(plan);
  fs.writeFileSync(path.join(outDir, "mincbot_fill_commands.txt"), cmds.join("\n") + "\n");

  disconnect(bot);
  stopServer(serverHandle);

  return report;
}

async function demo(flags) {
  const root = repoRoot();
  const extra = flags._ || [];
  const mar = extra[0] || ensureGoldShrineMar(root);
  return runImport(mar, flags);
}

async function main(argv) {
  const args = parseArgs(argv);
  const cmd = args._[0] || "demo";
  const rest = { _: args._.slice(1), flags: args.flags };
  if (cmd === "-h" || cmd === "help") {
    process.stdout.write(usage());
    return 0;
  }
  try {
    if (cmd === "demo") {
      const report = await demo({ ...rest.flags, _: rest._ });
      printReport(report);
      return report.ok ? 0 : 1;
    }
    if (cmd === "import" || cmd === "screenshot") {
      const file = rest._[0];
      if (!file) {
        process.stderr.write("missing file.mar\n");
        return 2;
      }
      const report = await runImport(path.resolve(file), rest.flags);
      printReport(report);
      return report.ok ? 0 : 1;
    }
    if (cmd === "check") {
      const file = rest._[0];
      if (!file) {
        process.stderr.write("missing file.mar\n");
        return 2;
      }
      const loaded = loadArchive(path.resolve(file));
      const gate = gateArchive(loaded, rest.flags.version || loaded.gameVersion, {
        strict: !!rest.flags.strict,
      });
      process.stdout.write(JSON.stringify({ ...loaded, image: undefined, opened: undefined, gate }, null, 2) + "\n");
      return gate.ok ? 0 : 1;
    }
    process.stderr.write(usage());
    return 2;
  } catch (e) {
    process.stderr.write(String(e && e.stack ? e.stack : e) + "\n");
    return 1;
  }
}

function printReport(report) {
  const lines = [
    `MincBot  file=${report.file}`,
    `  kind=${report.kind} edition=${report.edition} game=${report.gameVersion} pack=${report.pack}`,
    `  version-gate: ${report.gate.ok ? "ok" : "REJECT"} ${report.gate.reason || ""}`.trim(),
    `  plan: ${report.plan.plan.chunkCount} chunks, ${report.plan.plan.cuboids} cuboids, ${report.plan.plan.terrainCells} cells, ${report.plan.plan.compression}x compression`,
    `  server: ${report.server.ok ? `flying-squid ${report.server.version}` : `memory ${report.server.reason || ""}`}`.trim(),
    `  bot: ${report.botSession.ok ? "MincBot spawned" : report.botSession.reason || "skipped"}`,
    `  verify: ${report.verify.ok ? "PASS" : "FAIL"} cells=${report.verify.cells}`,
    `  screenshots: ${report.screenshots.before}`,
    `             ${report.screenshots.iso}`,
    `             ${report.screenshots.front}`,
  ];
  process.stdout.write(lines.join("\n") + "\n");
}

if (require.main === module) {
  main(process.argv.slice(2)).then(
    (code) => process.exit(code),
    (e) => {
      console.error(e);
      process.exit(1);
    }
  );
}

module.exports = { runImport, demo, ensureGoldShrineMar, parseArgs };
