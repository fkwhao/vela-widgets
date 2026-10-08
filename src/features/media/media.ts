export interface MediaControls { play: boolean; pause: boolean; previous: boolean; next: boolean; seek: boolean }
export interface MediaVolume { level: number; muted: boolean }
export interface MediaSession {
  id: string;
  source: string;
  title: string;
  artist: string;
  album: string;
  artwork: string | null;
  playbackStatus: "playing" | "paused" | "stopped" | "closed";
  positionMs: number;
  durationMs: number;
  updatedAt: number;
  playbackRate: number;
  controls: MediaControls;
  volume: MediaVolume | null;
}
export interface MediaSnapshot {
  revision: number;
  status: "disabled" | "ready" | "unavailable";
  session: MediaSession | null;
  error: string | null;
}
export interface MediaSpectrum {
  revision: number;
  sessionId: string | null;
  status: 'disabled' | 'ready' | 'unavailable';
  bands: number[];
}
export function mediaSpectrumBands(spectrum: MediaSpectrum | undefined, session: MediaSession): number[] {
  if (!spectrum || spectrum.status !== 'ready' || spectrum.sessionId !== session.id || session.playbackStatus !== 'playing') return [0,0,0,0];
  return Array.from({length:4}, (_,i) => Number.isFinite(spectrum.bands[i]) ? Math.max(0,Math.min(1,spectrum.bands[i])) : 0);
}
export type MediaActionName = "play" | "pause" | "previous" | "next" | "seek" | "volume" | "mute";
export interface MediaAction { sessionId: string; action: MediaActionName; positionMs?: number; volumeLevel?: number; muted?: boolean }

export function mediaPosition(session: MediaSession | null, now: number): number {
  if (!session || !Number.isFinite(session.durationMs) || session.durationMs <= 0) return 0;
  const elapsed = session.playbackStatus === "playing" && session.updatedAt > 0
    ? Math.max(0, now - session.updatedAt) * (Number.isFinite(session.playbackRate) && session.playbackRate > 0 ? session.playbackRate : 1) : 0;
  return Math.max(0, Math.min(session.durationMs, (Number.isFinite(session.positionMs) ? session.positionMs : 0) + elapsed));
}

export function mediaTime(milliseconds: number): string {
  const total = Math.floor(Math.max(0, Number.isFinite(milliseconds) ? milliseconds : 0) / 1000);
  const seconds = String(total % 60).padStart(2, "0");
  const minutes = Math.floor(total / 60);
  return minutes >= 60 ? `${Math.floor(minutes / 60)}:${String(minutes % 60).padStart(2, "0")}:${seconds}` : `${minutes}:${seconds}`;
}

export function mediaSource(source: string): string {
  const known: [RegExp, string][] = [
    [/spotify/i, "Spotify"], [/music\.163|cloudmusic/i, "网易云音乐"], [/qqmusic/i, "QQ 音乐"],
    [/msedge/i, "Microsoft Edge"], [/chrome/i, "Google Chrome"], [/firefox/i, "Firefox"],
    [/zunemusic|mediaplayer/i, "媒体播放器"], [/vlc/i, "VLC"], [/applemusic/i, "Apple Music"],
  ];
  const name = known.find(([pattern]) => pattern.test(source))?.[1];
  return name ?? source.split(/[\\/]/).pop()?.replace(/\.exe$/i, "").split("!").pop() ?? "媒体";
}

export function safeArtwork(artwork: string | null): string | null {
  // Only artwork supplied by the native raster reader; never fetch a media URL.
  return artwork && /^data:image\/(png|jpeg|gif|webp);base64,[A-Za-z0-9+/]+=*$/.test(artwork) ? artwork : null;
}
