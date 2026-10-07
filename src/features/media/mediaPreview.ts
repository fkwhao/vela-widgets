import type { MediaSession } from "./media";

// Local illustration used only in settings/fixtures. No player data or network requests.
export function createMediaPreviewSession(): MediaSession {
  const canvas = document.createElement("canvas");
  canvas.width = canvas.height = 512;
  const context = canvas.getContext("2d");
  if (context) {
    context.fillStyle = "#c66a46"; context.fillRect(0, 0, 512, 512);
    context.fillStyle = "#efd5a9"; context.beginPath(); context.arc(354, 160, 84, 0, Math.PI * 2); context.fill();
    context.fillStyle = "#67745a"; context.beginPath(); context.moveTo(0, 330); context.quadraticCurveTo(190, 130, 512, 310); context.lineTo(512, 512); context.lineTo(0, 512); context.fill();
    context.fillStyle = "#283f36"; context.beginPath(); context.moveTo(0, 398); context.quadraticCurveTo(250, 270, 512, 424); context.lineTo(512, 512); context.lineTo(0, 512); context.fill();
    context.fillStyle = "#f5eddc"; context.font = "46px Georgia"; context.fillText("AFTERGLOW", 30, 65);
    context.font = "16px Georgia"; context.fillText("SLOW DAYS / SIDE A", 32, 96);
  }
  return { id: "theme-preview", source: "Vela", title: "落日之后", artist: "Slow Days", album: "Afterglow",
    artwork: context ? canvas.toDataURL("image/png") : null, playbackStatus: "paused", positionMs: 72000, durationMs: 228000,
    updatedAt: 0, playbackRate: 1, controls: { play: true, pause: true, previous: true, next: true, seek: true }, volume: { level: 65, muted: false } };
}
