const { test } = require("node:test");
const assert = require("node:assert/strict");
const { compatible } = require("../src/version");

test("java 1.21.1 matches 1.21.4 family", () => {
  const r = compatible({ edition: "java", gameVersion: "1.21.1" }, "1.21.4");
  assert.equal(r.ok, true);
});

test("strict patch mismatch is rejected", () => {
  const r = compatible({ edition: "java", gameVersion: "1.21.1" }, "1.21.4", {
    strict: true,
  });
  assert.equal(r.ok, false);
});

test("bedrock archive is rejected", () => {
  const r = compatible({ edition: "bedrock", gameVersion: "1.21.70" }, "1.21.1");
  assert.equal(r.ok, false);
  assert.match(r.reason, /Java-only/);
});

test("1.20 vs 1.21 family is rejected", () => {
  const r = compatible({ edition: "java", gameVersion: "1.21.1" }, "1.20.4");
  assert.equal(r.ok, false);
});
