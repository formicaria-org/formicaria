// In-app microphone recording → a **16 kHz mono 16-bit PCM WAV** `File` — the format whisper.cpp reads
// directly (no server-side ffmpeg). Browser `MediaRecorder` would give webm/opus instead. We capture
// raw PCM via an `AudioContext` and encode the WAV ourselves.
//
// Device-general and defensive, because this runs in the browser build AND the phone's WebView:
//   - the AudioContext uses the engine's NATIVE rate (forcing 16 kHz throws in some engines); we
//     resample to 16 kHz in JS on stop, so the output is always what whisper wants regardless of device;
//   - the context is `resume()`d — created after the getUserMedia await it can start *suspended* under
//     autoplay policy, and a suspended context never fires `onaudioprocess` (a silent recording);
//   - a missing `navigator.mediaDevices` (an insecure context — e.g. the UI opened over a LAN IP rather
//     than localhost, where getUserMedia is disabled) fails with a plain, actionable message.
//
// getUserMedia needs a secure context: `localhost`/`127.0.0.1` and the phone's app scheme qualify; a
// `http://<lan-ip>` URL does not. The phone also needs RECORD_AUDIO (declared via the manifest inject);
// wry's onPermissionRequest then prompts and grants the WebView's AUDIO_CAPTURE.

const TARGET_RATE = 16000;

/** A live recording. `stop()` finalizes it into a WAV `File`; `cancel()` throws it away. */
export interface Recording {
  stop(): Promise<File>;
  cancel(): void;
}

type AudioCtor = typeof AudioContext;

/** Begin recording from the default microphone. Rejects with an actionable message if the mic is
 *  unavailable, permission is denied, or the context is an insecure origin. */
export async function startRecording(): Promise<Recording> {
  const md = navigator.mediaDevices;
  if (!md?.getUserMedia) {
    throw new Error(
      'microphone unavailable — recording needs a secure context; open the app on localhost (not a LAN address)',
    );
  }
  const stream = await md.getUserMedia({ audio: true });
  const Ctor: AudioCtor =
    window.AudioContext ?? (window as unknown as { webkitAudioContext: AudioCtor }).webkitAudioContext;
  // Native rate — forcing 16 kHz throws in some engines. We resample on stop instead.
  const ctx = new Ctor();
  // A context created after an await can be suspended; without resuming, no audio is ever processed.
  try {
    await ctx.resume();
  } catch {
    /* best effort — most engines are already running */
  }
  const rate = ctx.sampleRate;
  const source = ctx.createMediaStreamSource(stream);
  const node = ctx.createScriptProcessor(4096, 1, 1);
  const chunks: Float32Array[] = [];
  node.onaudioprocess = (e) => {
    // Copy — the event buffer is reused after the callback returns. (The node writes no output, so
    // connecting it to `destination` below plays silence, not a mic-to-speaker feedback loop.)
    chunks.push(new Float32Array(e.inputBuffer.getChannelData(0)));
  };
  source.connect(node);
  node.connect(ctx.destination); // some engines only pull audio when the node reaches a sink

  const teardown = () => {
    node.onaudioprocess = null;
    node.disconnect();
    source.disconnect();
    stream.getTracks().forEach((t) => t.stop());
    void ctx.close();
  };

  return {
    async stop() {
      teardown();
      const pcm = resampleTo(concat(chunks), rate, TARGET_RATE);
      const wav = encodeWav([pcm], TARGET_RATE);
      return new File([wav], `recording-${Date.now()}.wav`, { type: 'audio/wav' });
    },
    cancel() {
      teardown();
    },
  };
}

/** Flatten recorded chunks into one buffer. */
function concat(chunks: Float32Array[]): Float32Array {
  const length = chunks.reduce((n, c) => n + c.length, 0);
  const out = new Float32Array(length);
  let off = 0;
  for (const c of chunks) {
    out.set(c, off);
    off += c.length;
  }
  return out;
}

/** Linear-resample mono Float32 PCM from `srcRate` to `dstRate`. Pure, so it is unit-tested. Enough
 *  for speech → whisper; no anti-alias filter, which for downsampling a voice mic is inaudible. */
export function resampleTo(input: Float32Array, srcRate: number, dstRate: number): Float32Array {
  if (srcRate === dstRate || input.length === 0) return input;
  const ratio = srcRate / dstRate;
  const outLen = Math.max(1, Math.round(input.length / ratio));
  const out = new Float32Array(outLen);
  for (let i = 0; i < outLen; i++) {
    const pos = i * ratio;
    const i0 = Math.floor(pos);
    const i1 = Math.min(i0 + 1, input.length - 1);
    out[i] = input[i0] + (input[i1] - input[i0]) * (pos - i0);
  }
  return out;
}

/**
 * Encode mono Float32 PCM chunks (samples in [-1, 1]) as a 16-bit PCM WAV. Pure, so it is unit-tested
 * without a microphone. Produces the canonical 44-byte header + little-endian samples.
 */
export function encodeWav(chunks: Float32Array[], sampleRate: number): ArrayBuffer {
  const length = chunks.reduce((n, c) => n + c.length, 0);
  const buffer = new ArrayBuffer(44 + length * 2);
  const view = new DataView(buffer);
  const writeStr = (off: number, s: string) => {
    for (let i = 0; i < s.length; i++) view.setUint8(off + i, s.charCodeAt(i));
  };
  writeStr(0, 'RIFF');
  view.setUint32(4, 36 + length * 2, true); // file size - 8
  writeStr(8, 'WAVE');
  writeStr(12, 'fmt ');
  view.setUint32(16, 16, true); // PCM fmt chunk size
  view.setUint16(20, 1, true); // audio format = PCM
  view.setUint16(22, 1, true); // channels = mono
  view.setUint32(24, sampleRate, true);
  view.setUint32(28, sampleRate * 2, true); // byte rate = rate * channels * bytesPerSample
  view.setUint16(32, 2, true); // block align = channels * bytesPerSample
  view.setUint16(34, 16, true); // bits per sample
  writeStr(36, 'data');
  view.setUint32(40, length * 2, true);

  let off = 44;
  for (const chunk of chunks) {
    for (let i = 0; i < chunk.length; i++) {
      const s = Math.max(-1, Math.min(1, chunk[i]));
      view.setInt16(off, s < 0 ? s * 0x8000 : s * 0x7fff, true);
      off += 2;
    }
  }
  return buffer;
}
