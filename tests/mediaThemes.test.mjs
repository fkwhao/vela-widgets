import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import ts from "typescript";
async function load(file) {
  const source = readFileSync(new URL(`../src/features/media/${file}.ts`, import.meta.url), "utf8");
  const output = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext } }).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(output).toString("base64")}`);
}
const { paletteFromPixels, fallbackArtworkPalette } = await load("artworkPalette");
const { mediaWidgetDimensions } = await load("mediaThemes");
const luminance = value => value.match(/\d+/g).map(Number).reduce((sum, channel, index) => {
  const c = channel / 255;
  return sum + (c <= .04045 ? c / 12.92 : ((c + .055) / 1.055) ** 2.4) * [.2126,.7152,.0722][index];
}, 0);
test("cover palettes ignore transparent pixels, preserve cover hues and keep white text readable", () => {
  assert.deepEqual(paletteFromPixels([255,0,0,0,0,255,0,12]), fallbackArtworkPalette);
  const red = paletteFromPixels(Array.from({length:16}, () => [230,50,60,255]).flat());
  const blue = paletteFromPixels(Array.from({length:16}, () => [50,60,230,255]).flat());
  assert.notDeepEqual(red, blue);
  for (const pixels of [[255,255,255,255],[0,0,0,255],[255,255,0,255],[240,100,140,255,40,70,220,255]]) {
    for (const value of Object.values(paletteFromPixels(pixels))) assert.ok(1.05 / (luminance(value) + .05) >= 4.5);
  }
});
test("only the medium cover card uses a portrait native footprint", () => {
  assert.deepEqual(mediaWidgetDimensions("medium","atmosphere"), {width:170,height:364});
  for (const theme of ["default","vinyl","minimal"]) assert.deepEqual(mediaWidgetDimensions("medium",theme),{width:364,height:170});
  assert.deepEqual(mediaWidgetDimensions("small","atmosphere"),{width:170,height:170});
  assert.deepEqual(mediaWidgetDimensions("large","atmosphere"),{width:364,height:384});
});
