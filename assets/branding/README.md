# Vela icon

The approved modular desktop spirit uses a dark, near-square body with two eyes
and a detached upper-right module on a yellow-green rounded tile.

- `vela-approved.png`: the original approved concept.
- `vela-icon.png`: the production source with transparent exterior corners.
- `../../src-tauri/icons/vela.ico`: the Windows executable and installer icon.
- `../../public/vela-icon.png`: the browser and About-page icon.

Generated with the built-in image generation tool. Production edit prompt:
remove only the exterior black corners and preserve the approved character,
colors, layout and rounded tile, with genuine alpha transparency outside it.

To regenerate platform formats, run from the project root:

```powershell
npm run tauri -- icon assets/branding/vela-icon.png -o artifacts/icon-assets
Copy-Item artifacts/icon-assets/icon.ico src-tauri/icons/vela.ico
Copy-Item artifacts/icon-assets/128x128.png public/vela-icon.png
```
