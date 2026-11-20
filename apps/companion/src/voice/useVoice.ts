import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react';
import { voiceAPI, voiceError, type Take, type VoicePreferences } from './api';
import { capture, type Capture } from './pcm';

type Phase = 'idle' | 'loading' | 'playing' | 'paused' | 'starting' | 'recording' | 'transcribing';
interface Scope {
  id: string;
  persona: string;
  platform: string;
  abort: AbortController;
  timer?: number;
}
interface Input extends Scope {
  type: 'input';
  capture?: Capture;
  finishing: boolean;
}
interface Output extends Scope {
  type: 'output';
  context: AudioContext;
  gain: GainNode;
  analyser: AnalyserNode;
  volume: number;
  source?: AudioBufferSourceNode;
}
type Operation = Input | Output;

export function chunksForSpeech(text: string): string[] {
  const chunks: string[] = [];
  let chunk: string[] = [];
  for (const char of Array.from(text.trim()).slice(0, 1680)) {
    chunk.push(char);
    if (chunk.length === 280 || (chunk.length >= 60 && /[。！？.!?\n]/u.test(char))) {
      const spoken = chunk.join('').trim();
      if (spoken) chunks.push(spoken);
      chunk = [];
    }
  }
  const last = chunk.join('').trim();
  if (last) chunks.push(last);
  return chunks;
}

async function playTake(job: Output, take: Take, onStart: () => void): Promise<void> {
  if (job.abort.signal.aborted) return;
  if (!take.wav || take.wav.length > 8 * 1024 * 1024 * 4 / 3 + 4) throw 'VOICE_INVALID_AUDIO';
  const bytes = Uint8Array.from(atob(take.wav), char => char.charCodeAt(0));
  const decoded = await job.context.decodeAudioData(bytes.buffer);
  if (job.abort.signal.aborted) return;
  const source = job.context.createBufferSource();
  job.source = source;
  source.buffer = decoded;
  source.connect(job.gain);
  await new Promise<void>((resolve, reject) => {
    const complete = () => {
      job.abort.signal.removeEventListener('abort', complete);
      job.source = undefined;
      source.disconnect();
      resolve();
    };
    source.onended = complete;
    job.abort.signal.addEventListener('abort', complete, { once: true });
    try { source.start(); onStart(); }
    catch (error) {
      job.abort.signal.removeEventListener('abort', complete);
      source.disconnect();
      job.source = undefined;
      reject(error);
    }
  });
}

export function useVoice(
  persona: string,
  platform: string,
  page: string,
  onTranscript: (text: string) => void,
) {
  const enabled = !!persona && ['windows', 'android'].includes(platform);
  const [stored, setStored] = useState<{ persona: string; prefs: VoicePreferences }>();
  const prefs = stored?.persona === persona ? stored.prefs : undefined;
  const [phase, setPhase] = useState<Phase>('idle');
  const [muted, setMuted] = useState(false);
  const [message, setMessage] = useState('');
  const [problem, setProblem] = useState('');
  const [model, setModel] = useState('');
  const current = useRef<Operation | null>(null);

  const stop = useCallback(() => {
    const job = current.current;
    if (!job) return;
    current.current = null;
    clearTimeout(job.timer);
    if (job.type === 'input') {
      job.abort.abort();
      job.capture?.cancel();
      void voiceAPI.record(job.persona, job.id, { kind: 'cancel' }).catch(() => {});
    } else {
      try { job.source?.stop(); } catch { /* It may already have ended. */ }
      job.abort.abort();
      void job.context.close().catch(() => {});
      if (job.platform === 'windows') void voiceAPI.meter(job.persona, 0).catch(() => {});
      if (job.platform === 'android') void voiceAPI.focus(job.id, 'release').catch(() => {});
    }
    setPhase('idle');
  }, []);

  useLayoutEffect(() => {
    setMessage('');
    setProblem('');
    return stop;
  }, [persona, page, stop]);
  useEffect(() => {
    if (!enabled) return;
    let alive = true;
    void voiceAPI.prefs(persona).then(value => {
      if (alive) setStored({ persona, prefs: value });
    }).catch(error => { if (alive) setProblem(voiceError(error)); });
    return () => { alive = false; };
  }, [enabled, persona]);
  useEffect(() => {
    if (!enabled) return;
    const onHide = () => { if (document.hidden) stop(); };
    const onBlur = () => {
      // Windows retains playback when the user clicks the desktop pet.
      const job = current.current;
      if (platform === 'windows' && job?.type === 'input' && job.capture) stop();
    };
    window.addEventListener('blur', onBlur);
    window.addEventListener('pagehide', stop);
    document.addEventListener('visibilitychange', onHide);
    return () => {
      window.removeEventListener('blur', onBlur);
      window.removeEventListener('pagehide', stop);
      document.removeEventListener('visibilitychange', onHide);
    };
  }, [enabled, platform, stop]);

  async function save(update: VoicePreferences) {
    const saved = await voiceAPI.prefs(persona, update);
    setStored({ persona, prefs: saved });
  }

  async function chooseModel() {
    const selected = await voiceAPI.selectModel();
    if (selected) setModel(selected);
  }

  function report(job: Operation, error: unknown) {
    if (current.current === job) {
      stop();
      if (error !== 'VOICE_CANCELLED') setProblem(voiceError(error));
    }
  }

  async function play(text?: string) {
    if (!enabled || !prefs) return;
    if (current.current?.type === 'output') stop();
    if (current.current) return;
    const sections = text === undefined ? [] : chunksForSpeech(text);
    if (text !== undefined && !sections.length) return;
    setMessage('');
    setProblem('');
    let context: AudioContext;
    try { context = new AudioContext(); }
    catch (error) { setProblem(voiceError(error)); return; }
    const gain = context.createGain();
    const analyser = context.createAnalyser();
    const volume = prefs.volume_percent / 100;
    gain.gain.value = prefs.muted ? 0 : volume;
    gain.connect(analyser).connect(context.destination);
    const job: Output = {
      type: 'output', id: crypto.randomUUID(), persona, platform,
      abort: new AbortController(), context, analyser, gain, volume,
    };
    current.current = job;
    setMuted(prefs.muted);
    setPhase('loading');
    try {
      await context.resume();
      if (current.current !== job) return;
      if (platform === 'android') {
        const acquired = await voiceAPI.focus(job.id, 'acquire');
        if (current.current !== job) {
          if (acquired) void voiceAPI.focus(job.id, 'release').catch(() => {});
          return;
        }
        if (!acquired) throw 'VOICE_FOCUS_LOST';
      }
      const samples = new Float32Array(analyser.fftSize);
      async function watchOutput() {
        if (current.current !== job) return;
        try {
          if (platform === 'android' && !await voiceAPI.focus(job.id, 'status')) throw 'VOICE_FOCUS_LOST';
          if (platform === 'windows') {
            let level = 0;
            if (job.source && context.state === 'running' && gain.gain.value > 0) {
              analyser.getFloatTimeDomainData(samples);
              let energy = 0;
              for (const sample of samples) energy += sample * sample;
              level = Math.min(100, Math.round(Math.sqrt(energy / samples.length) * 320));
            }
            await voiceAPI.meter(persona, level);
          }
        } catch (error) { report(job, error); return; }
        if (current.current === job) job.timer = window.setTimeout(watchOutput, platform === 'windows' ? 85 : 200);
      }
      void watchOutput();
      if (text === undefined) {
        const guide = await voiceAPI.guide(persona);
        if (current.current !== job) return;
        await playTake(job, guide.audio, () => setPhase('playing'));
      } else {
        // Retain at most one prefetched section and observe every rejection immediately.
        const load = (section: string) => voiceAPI.speak(persona, job.id, section)
          .then(value => ({ value }), error => ({ error }));
        let next = load(sections[0]);
        for (let index = 0; index < sections.length; index++) {
          const result = await next;
          if (current.current !== job) return;
          if ('error' in result) throw result.error;
          if (index + 1 < sections.length) next = load(sections[index + 1]);
          await playTake(job, result.value, () => setPhase('playing'));
          if (current.current !== job) return;
          if (index + 1 < sections.length) setPhase('loading');
        }
      }
      if (current.current === job) {
        stop();
        if (text && Array.from(text.trim()).length > 1680) setMessage('已朗读此消息的前 1680 字。');
      }
    } catch (error) { report(job, error); }
  }

  async function pause() {
    const job = current.current;
    if (job?.type !== 'output' || !job.source) return;
    try {
      if (job.context.state === 'suspended') {
        await job.context.resume();
        if (current.current === job) setPhase('playing');
      } else {
        await job.context.suspend();
        if (current.current === job) setPhase('paused');
      }
    } catch (error) { report(job, error); }
  }

  function toggleMute() {
    const job = current.current;
    if (job?.type !== 'output') return;
    const next = !muted;
    job.gain.gain.value = next ? 0 : job.volume;
    setMuted(next);
  }

  function accept(job: Input, transcript: string | null) {
    if (current.current !== job) return;
    stop();
    if (transcript?.trim()) {
      onTranscript(transcript.trim());
      setMessage('已把识别结果填入输入框。');
    }
  }

  async function finish(job = current.current) {
    if (job?.type !== 'input' || current.current !== job || job.finishing) return;
    job.finishing = true;
    clearTimeout(job.timer);
    setPhase('transcribing');
    try {
      if (job.platform === 'android') {
        await voiceAPI.record(job.persona, job.id, { kind: 'finish' });
      } else {
        const pcm = job.capture?.stop();
        if (!pcm) throw 'VOICE_RECORDING_LENGTH';
        job.capture = undefined;
        accept(job, await voiceAPI.record(job.persona, job.id, { kind: 'transcribe', pcm }));
      }
    } catch (error) { report(job, error); }
  }

  async function start() {
    if (!enabled || current.current) return;
    setProblem('');
    setMessage('');
    const job: Input = {
      type: 'input', id: crypto.randomUUID(), persona, platform,
      abort: new AbortController(), finishing: false,
    };
    current.current = job;
    setPhase('starting');
    try {
      await voiceAPI.record(persona, job.id, { kind: 'check' });
      if (current.current !== job) return;
      if (platform === 'android') {
        const result = voiceAPI.record(persona, job.id, { kind: 'begin' });
        const watchInput = async () => {
          if (current.current !== job || job.finishing) return;
          try {
            const state = await voiceAPI.record(persona, job.id, { kind: 'status' });
            if (current.current !== job || job.finishing) return;
            if (state === 'recording') setPhase('recording');
            if (state === 'transcribing') setPhase('transcribing');
            job.timer = window.setTimeout(watchInput, 400);
          } catch (error) { report(job, error); }
        };
        job.timer = window.setTimeout(watchInput, 240);
        accept(job, await result);
      } else {
        job.capture = await capture(job.abort.signal, () => void finish(job));
        if (current.current !== job) { job.capture.cancel(); return; }
        setPhase('recording');
        job.timer = window.setTimeout(() => void finish(job), 30_000);
      }
    } catch (error) { report(job, error); }
  }

  return { enabled, prefs, save, model, chooseModel, phase, muted, message, problem, play, pause, toggleMute, start, finish, stop };
}

export type VoiceController = ReturnType<typeof useVoice>;
