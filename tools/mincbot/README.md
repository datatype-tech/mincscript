//! MincBot — Java-edition MAR importer (FastBuilder-style, chunked).
//!
//! MincBot is a **mineflayer** bot. It reads a `.mar` (JAR-like ZIP, default) or a
//! raw `.mincb`, checks that the archive is Java and matches the server version
//! family, then imports world blocks the way FastBuilder / WorldEdit do:
//!
//! 1. Group cells by chunk (`x>>4`, `z>>4`)
//! 2. Spiral chunks from the pack origin
//! 3. Greedy-merge each chunk into cuboids (`/fill`)
//! 4. Place containers, then command-block chains last
//!
//! Live target is a **flying-squid** mini Java server (offline, creative). If the
//! protocol server cannot boot in this environment, the same plan is applied to an
//! in-memory voxel world and rendered headless with pngjs — that is the
//! screenshot verification path.
//!
//! Bedrock is not supported yet.
//!
//! Open-source code this is specialized from:
//! - [mineflayer](https://github.com/PrismarineJS/mineflayer) — Java bot
//! - [flying-squid](https://github.com/PrismarineJS/flying-squid) — headless Java server
//! - FastBuilder / PhoenixBuilder chunk spiral + fill-first install
//! - WorldEdit / FAWE 3D greedy cuboids
//!
//! ```bash
//! cargo run --bin minc -- build examples/gold-shrine --out examples/gold-shrine/dist
//! node tools/mincbot/src/index.js demo --no-server --out tools/mincbot/out
//! node tools/mincbot/src/index.js check examples/gold-shrine/dist/gold-shrine.mar --version 1.21.1
//! ```
