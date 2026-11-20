import captureUrl from './capture.worklet.js?url&no-inline';

export interface Capture {
  stop(): string;
  cancel(): void;
}

const RATE = 16000;
const MAX_FRAMES = 30 * RATE;

export function encode(bytes: Uint8Array): string {
  const chunks: string[] = [];
  for (let start = 0; start < bytes.length; start += 8192) {
    chunks.push(String.fromCharCode(...bytes.subarray(start, start + 8192)));
  }
  return btoa(chunks.join(''));
}

export async function capture(signal: AbortSignal, atLimit: () => void): Promise<Capture> {
  if (!navigator.mediaDevices?.getUserMedia) throw 'VOICE_UNAVAILABLE';
  const stream = await navigator.mediaDevices.getUserMedia({ audio: { channelCount: 1 }, video: false });
  const releaseStream = () => stream.getTracks().forEach(track => track.stop());
  if (signal.aborted) {
    releaseStream();
    throw 'VOICE_CANCELLED';
  }
  let context: AudioContext;
  try { context = new AudioContext({ sampleRate: RATE }); }
  catch (error) { releaseStream(); throw error; }
  const pcm = new DataView(new ArrayBuffer(MAX_FRAMES * 2));
  let used = 0;
  let ended = false;
  let worklet: AudioWorkletNode | undefined;
  function close() {
    if (ended) return;
    ended = true;
    signal.removeEventListener('abort', close);
    releaseStream();
    worklet?.port.close();
    void context.close().catch(() => {});
  }
  signal.addEventListener('abort', close, { once: true });
  try {
    if (context.sampleRate !== RATE || !context.audioWorklet) throw 'VOICE_UNAVAILABLE';
    await context.audioWorklet.addModule(captureUrl);
    if (signal.aborted) throw 'VOICE_CANCELLED';
    worklet = new AudioWorkletNode(context, 'companion-microphone', { channelCount: 1 });
    worklet.port.onmessage = ({ data }: MessageEvent<Float32Array>) => {
      if (ended) return;
      for (const frame of data) {
        if (used === MAX_FRAMES) break;
        const clamped = Number.isFinite(frame) ? Math.max(-1, Math.min(1, frame)) : 0;
        pcm.setInt16(used++ * 2, Math.round(clamped * 32767), true);
      }
      if (used === MAX_FRAMES) atLimit();
    };
    const silent = context.createGain();
    silent.gain.value = 0;
    context.createMediaStreamSource(stream).connect(worklet).connect(silent).connect(context.destination);
    await context.resume();
    if (signal.aborted) throw 'VOICE_CANCELLED';
    return {
      stop() {
        close();
        if (used < RATE * 0.5) throw 'VOICE_RECORDING_LENGTH';
        return encode(new Uint8Array(pcm.buffer, 0, used * 2));
      },
      cancel: close,
    };
  } catch (error) {
    close();
    throw error;
  }
}
