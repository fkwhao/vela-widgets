import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import ts from 'typescript';

const source = readFileSync(new URL('../src/shared/color.ts', import.meta.url), 'utf8');
const output = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext } }).outputText;
const { normalizeHex, hexToRgb, rgbToHex, hexToHsv, hsvToHex } = await import(`data:text/javascript;base64,${Buffer.from(output).toString('base64')}`);

test('color input accepts short and full hex, and rejects incomplete or invalid values', () => {
  assert.equal(normalizeHex(' #AbC '), '#aabbcc');
  assert.equal(normalizeHex('A1B2C3'), '#a1b2c3');
  for (const value of ['', '#ab', '#abcd', '#1234567', '#gg0000', 'red']) assert.equal(normalizeHex(value), null);
  assert.deepEqual(hexToRgb('#00ff80'), [0, 255, 128]);
  assert.equal(rgbToHex(-5, 300, 127.6), '#00ff80');
});

test('picker conversion preserves RGB colors, hue wraparound, and grayscale hue', () => {
  for (const color of ['#000000', '#ffffff', '#808080', '#ff0000', '#00ff00', '#0000ff', '#3b67b8', '#aabbcc', '#fefefe']) {
    assert.equal(hsvToHex(hexToHsv(color)), color);
  }
  assert.equal(hexToHsv('#808080', 240).h, 240);
  assert.equal(hexToHsv('#000000', 120).h, 120);
  assert.equal(hsvToHex({ h: 360, s: 100, v: 100 }), '#ff0000');
  assert.equal(hsvToHex({ h: -120, s: 100, v: 100 }), '#0000ff');
});
