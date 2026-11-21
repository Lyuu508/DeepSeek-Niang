export const clipLimit = 12 * 1024 * 1024;
const readLimit = 128 * 1024 * 1024;
export type Plan = { focus: string; start_ms: number; end_ms: number };
export type VideoMode = 'clip' | 'frames';
type Frame = { at_ms: number; jpeg: string };
export type VideoInput = { kind: 'clip'; mp4: string } | { kind: 'frames'; frames: Frame[] };

async function encode(blob: Blob) {
  const bytes = new Uint8Array(await blob.arrayBuffer());
  const chunks: string[] = [];
  for (let start = 0; start < bytes.length; start += 8192) {
    chunks.push(String.fromCharCode(...bytes.subarray(start, start + 8192)));
  }
  return btoa(chunks.join(''));
}

function mediaEvent(video: HTMLVideoElement, name: string, start: () => void) {
  return new Promise<void>((resolve, reject) => {
    function cleanup() {
      window.clearTimeout(timeout);
      video.removeEventListener(name, success);
      video.removeEventListener('error', failure);
    }
    function success() { cleanup(); resolve(); }
    function failure() { cleanup(); reject('VIDEO_WEBVIEW_DECODE'); }
    const timeout = window.setTimeout(failure, 8000);
    video.addEventListener(name, success, { once: true });
    video.addEventListener('error', failure, { once: true });
    try { start(); } catch { failure(); }
  });
}

async function withVideo<T>(file: File, use: (video: HTMLVideoElement) => Promise<T>) {
  if (file.size === 0 || file.size > readLimit) throw 'VIDEO_FILE_LIMIT';
  const url = URL.createObjectURL(file);
  const video = document.createElement('video');
  video.muted = true;
  video.playsInline = true;
  video.preload = 'metadata';
  try {
    await mediaEvent(video, 'loadedmetadata', () => { video.src = url; });
    const length = Math.floor(video.duration * 1000);
    if (!Number.isFinite(length) || length < 100 || length > 300_000 || !video.videoWidth || !video.videoHeight) {
      throw 'VIDEO_FILE_DURATION';
    }
    return await use(video);
  } finally {
    video.removeAttribute('src');
    video.load();
    URL.revokeObjectURL(url);
  }
}

export function canSendClip(file: File) {
  return file.size <= clipLimit && /\.mp4$/i.test(file.name);
}

export function videoDuration(file: File) {
  return withVideo(file, async video => Math.floor(video.duration * 1000));
}

export function sampleTimes(plan: Plan) {
  const span = plan.end_ms - plan.start_ms;
  const count = Math.min(12, Math.max(4, Math.ceil(span / 2000)));
  return Array.from({ length: count }, (_, index) => plan.start_ms + Math.floor(span * (index + .5) / count));
}

function jpeg(canvas: HTMLCanvasElement, quality: number) {
  return new Promise<Blob>((resolve, reject) => canvas.toBlob(
    blob => blob?.type === 'image/jpeg' ? resolve(blob) : reject('VIDEO_WEBVIEW_DECODE'), 'image/jpeg', quality));
}

export async function prepareVideo(file: File, mode: VideoMode, plan: Plan): Promise<VideoInput> {
  if (mode === 'clip') {
    if (!canSendClip(file)) throw 'VIDEO_FILE_LIMIT';
    return { kind: 'clip', mp4: await encode(file) };
  }
  return withVideo(file, async video => {
    const scale = Math.min(1, 720 / Math.max(video.videoWidth, video.videoHeight));
    const canvas = document.createElement('canvas');
    canvas.width = Math.max(1, Math.floor(video.videoWidth * scale));
    canvas.height = Math.max(1, Math.floor(video.videoHeight * scale));
    const context = canvas.getContext('2d');
    if (!context) throw 'VIDEO_WEBVIEW_DECODE';
    const frames: Frame[] = [];
    for (const at of sampleTimes(plan)) {
      if (video.readyState < 2 || Math.abs(video.currentTime - at / 1000) > .0005) {
        await mediaEvent(video, 'seeked', () => { video.currentTime = at / 1000; });
      }
      context.drawImage(video, 0, 0, canvas.width, canvas.height);
      let blob = await jpeg(canvas, .74);
      if (blob.size > 180 * 1024) blob = await jpeg(canvas, .45);
      if (blob.size > 180 * 1024) throw 'VIDEO_FILE_LIMIT';
      frames.push({ at_ms: Math.round(video.currentTime * 1000), jpeg: await encode(blob) });
    }
    return { kind: 'frames', frames };
  });
}
