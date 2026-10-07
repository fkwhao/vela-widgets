import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import ts from "typescript";
const source = readFileSync(new URL("../src/features/media/media.ts", import.meta.url), "utf8");
const output = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext } }).outputText;
const { mediaPosition, mediaTime, mediaSource, safeArtwork } = await import(`data:text/javascript;base64,${Buffer.from(output).toString("base64")}`);
const session = (overrides = {}) => ({ positionMs: 10000, durationMs: 60000, playbackStatus: "playing", updatedAt: 100000, playbackRate: 1, ...overrides });

test("media progress projects from the player's timestamp, including playback rate and wake-up", () => {
  assert.equal(mediaPosition(session(), 105000), 15000);
  assert.equal(mediaPosition(session({ playbackRate: 2 }), 105000), 20000);
  assert.equal(mediaPosition(session(), 1000000), 60000);
  assert.equal(mediaPosition(session(), 90000), 10000);
});
test("paused and stopped media retain position; live streams and missing timestamps don't invent progress", () => {
  for (const playbackStatus of ["paused", "stopped", "closed"]) assert.equal(mediaPosition(session({ playbackStatus }), 105000), 10000);
  assert.equal(mediaPosition(session({ durationMs: 0 }), 105000), 0);
  assert.equal(mediaPosition(session({ updatedAt: 0 }), 105000), 10000);
  assert.equal(mediaPosition(null, 105000), 0);
});
test("invalid timeline values remain bounded and duration labels support long media", () => {
  assert.equal(mediaPosition(session({ playbackRate: NaN, positionMs: -50000 }), 105000), 0);
  assert.equal(mediaPosition(session({ durationMs: NaN }), 105000), 0);
  assert.equal(mediaTime(-1), "0:00");
  assert.equal(mediaTime(125000), "2:05");
  assert.equal(mediaTime(3661000), "1:01:01");
  assert.equal(mediaTime(Infinity), "0:00");
});
test("cover rendering never fetches URLs or accepts SVG and malformed data", () => {
  assert.equal(safeArtwork("data:image/png;base64,aGVsbG8="), "data:image/png;base64,aGVsbG8=");
  for (const value of [null, "https://example.com/cover.png", "file:///cover.png", "data:image/svg+xml;base64,PHN2Zz4=", "data:image/png;base64,***"]) assert.equal(safeArtwork(value), null);
});
test("media sources have readable names and preserve unknown player names", () => {
  assert.equal(mediaSource("Spotify.exe"), "Spotify");
  assert.equal(mediaSource("C:\\Apps\\msedge.exe"), "Microsoft Edge");
  assert.equal(mediaSource("Custom.Package!Player"), "Player");
  assert.equal(mediaSource("Custom.exe"), "Custom");
});
