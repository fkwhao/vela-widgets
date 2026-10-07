type RGB = [number, number, number];
export interface ArtworkPalette { first: string; second: string; third: string }
const color = (rgb: RGB) => `rgb(${rgb.map(value => Math.round(value)).join(" ")})`;
// Muted fallback for missing, unreadable or completely transparent covers.
export const fallbackArtworkPalette: ArtworkPalette = { first: "rgb(89 72 89)", second: "rgb(58 77 96)", third: "rgb(106 76 89)" };
const luminance = (rgb: RGB) => rgb.reduce((sum, value, index) => {
  const channel = value / 255;
  return sum + (channel <= .04045 ? channel / 12.92 : ((channel + .055) / 1.055) ** 2.4) * [ .2126, .7152, .0722 ][index]!;
}, 0);
const distance = (a: RGB, b: RGB) => Math.sqrt(a.reduce((sum, value, index) => sum + (value - b[index]!) ** 2, 0));
function readableTone(rgb: RGB): string {
  // Keep the cover's hue while bounding brightness for white titles/controls (>= 4.5:1).
  let toned = rgb.map(value => value * .76 + 22 * .24) as RGB;
  while (luminance(toned) > .17) toned = toned.map(value => value * .94) as RGB;
  return color(toned);
}

export function paletteFromPixels(pixels: ArrayLike<number>): ArtworkPalette {
  const buckets = new Map<number, { rgb: RGB; count: number }>();
  for (let i = 0; i + 3 < pixels.length; i += 4) {
    if (pixels[i + 3]! < 128) continue;
    const rgb: RGB = [pixels[i]!, pixels[i + 1]!, pixels[i + 2]!];
    const key = (rgb[0] >> 5) * 64 + (rgb[1] >> 5) * 8 + (rgb[2] >> 5);
    const bucket = buckets.get(key) ?? { rgb: [0, 0, 0] as RGB, count: 0 };
    bucket.rgb = bucket.rgb.map((value, index) => value + rgb[index]!) as RGB;
    bucket.count++; buckets.set(key, bucket);
  }
  const candidates = Array.from(buckets.values()).map(bucket => {
    const rgb = bucket.rgb.map(value => value / bucket.count) as RGB;
    const max = Math.max(...rgb), min = Math.min(...rgb);
    const saturation = max ? (max - min) / max : 0;
    return { rgb, score: Math.sqrt(bucket.count) * (.25 + saturation) * (max < 32 || min > 235 ? .08 : 1) };
  }).sort((a, b) => b.score - a.score);
  if (!candidates.length) return { ...fallbackArtworkPalette };
  const first = candidates[0]!.rgb;
  const second = [...candidates].sort((a,b) => b.score * distance(b.rgb,first) - a.score * distance(a.rgb,first))[0]!.rgb;
  const third = [...candidates].sort((a,b) => b.score * Math.min(distance(b.rgb,first),distance(b.rgb,second)) - a.score * Math.min(distance(a.rgb,first),distance(a.rgb,second)))[0]!.rgb;
  return { first: readableTone(first), second: readableTone(second), third: readableTone(third) };
}
