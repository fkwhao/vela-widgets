import { onUnmounted, ref, watch, type Ref } from "vue";
import { fallbackArtworkPalette, paletteFromPixels, type ArtworkPalette } from "./artworkPalette";
import { safeArtwork } from "./media";

const cache = new Map<string, Promise<ArtworkPalette>>();
function readPalette(artwork: string): Promise<ArtworkPalette> {
  const cached = cache.get(artwork);
  if (cached) return cached;
  const pending = new Promise<ArtworkPalette>(resolve => {
    const image = new Image();
    image.onload = () => {
      try {
        const canvas = document.createElement("canvas");
        canvas.width = canvas.height = 32;
        const context = canvas.getContext("2d", { willReadFrequently: true });
        if (!context) { resolve({ ...fallbackArtworkPalette }); return; }
        context.drawImage(image, 0, 0, 32, 32);
        resolve(paletteFromPixels(context.getImageData(0, 0, 32, 32).data));
      } catch { resolve({ ...fallbackArtworkPalette }); }
    };
    image.onerror = () => resolve({ ...fallbackArtworkPalette });
    image.src = artwork;
  });
  if (cache.size >= 3) cache.delete(cache.keys().next().value!);
  cache.set(artwork, pending);
  return pending;
}

export function useArtworkPalette(artwork: Ref<string | null>) {
  const palette = ref<ArtworkPalette>({ ...fallbackArtworkPalette });
  let revision = 0;
  watch(artwork, async value => {
    const current = ++revision;
    palette.value = { ...fallbackArtworkPalette };
    const safe = safeArtwork(value);
    if (!safe) return;
    const result = await readPalette(safe);
    if (revision === current) palette.value = result;
  }, { immediate: true });
  onUnmounted(() => { revision++; });
  return palette;
}
