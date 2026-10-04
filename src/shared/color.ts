export type HSV = { h: number; s: number; v: number };
export const clamp = (value: number, max = 100): number => Math.max(0, Math.min(max, Number.isFinite(value) ? value : 0));

export function normalizeHex(value: string): string | null {
  const raw = value.trim().replace(/^#/, '');
  if (/^[\da-f]{3}$/i.test(raw)) return `#${[...raw].map(c => c + c).join('').toLowerCase()}`;
  return /^[\da-f]{6}$/i.test(raw) ? `#${raw.toLowerCase()}` : null;
}

export function hexToRgb(hex: string): [number, number, number] {
  const value = normalizeHex(hex);
  if (!value) throw new Error('Invalid hex color');
  return [1, 3, 5].map(i => Number.parseInt(value.slice(i, i + 2), 16)) as [number, number, number];
}

export function rgbToHex(r: number, g: number, b: number): string {
  return `#${[r, g, b].map(c => Math.round(clamp(c, 255)).toString(16).padStart(2, '0')).join('')}`;
}

export function hexToHsv(hex: string, fallbackHue = 0): HSV {
  const [r, g, b] = hexToRgb(hex).map(c => c / 255);
  const max = Math.max(r!, g!, b!), min = Math.min(r!, g!, b!), delta = max - min;
  let h = fallbackHue;
  if (delta) {
    h = max === r ? ((g! - b!) / delta) % 6 : max === g ? (b! - r!) / delta + 2 : (r! - g!) / delta + 4;
    h = (h * 60 + 360) % 360;
  }
  return { h, s: max ? delta / max * 100 : 0, v: max * 100 };
}

export function hsvToHex({ h, s, v }: HSV): string {
  const hue = ((h % 360) + 360) % 360 / 60;
  const saturation = clamp(s) / 100, value = clamp(v) / 100;
  const chroma = value * saturation, x = chroma * (1 - Math.abs(hue % 2 - 1)), m = value - chroma;
  const channels = hue < 1 ? [chroma, x, 0] : hue < 2 ? [x, chroma, 0] : hue < 3 ? [0, chroma, x] : hue < 4 ? [0, x, chroma] : hue < 5 ? [x, 0, chroma] : [chroma, 0, x];
  return rgbToHex(...channels.map(c => (c + m) * 255) as [number, number, number]);
}
