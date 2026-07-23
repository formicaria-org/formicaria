// In-app microphone recording → a **16 kHz mono 16-bit PCM WAV** `File` — the exact format
// whisper.cpp's `whisper-server` accepts directly, so the transcribe path needs no server-side ffmpeg
// conversion. Browser `MediaRecorder` would give webm/opus instead, which whisper can't read without
// `--convert`; so we capture raw PCM via an `AudioContext` pinned to 16 kHz (the browser resamples the
// mic into it) and encode the WAV ourselves. The resulting `File` goes through the *same* ingest→embed
// path as a dropped or attached file, so a recorded clip and an attached one behave identically —
// including that `/transcribe` then turns either into a proposal.
//
// getUserMedia needs a secure context; `127.0.0.1`/`localhost` qualify, so this works in the browser
// build. (The phone/wry build additionally needs the RECORD_AUDIO permission + wry permission plumbing;
// transcription is off on the phone for now anyway.)

/** A live recording. `stop()` finalizes it into a WAV `File`; `cancel()` throws it away. */
export interface Recording {
  stop(): Promise<File>;
  cancel(): void;
}

type AudioCtor = typeof AudioContext;

/** Begin recording from the default microphone. Rejects if permission is denied or there is no mic. */
export async function startRecording(): Promise<Recording> {
  const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
  const Ctor: AudioCtor =
    window.AudioContext ?? (window as unknown as { webkitAudioContext: AudioCtor }).webkitAudioContext;
  // Ask for 16 kHz so the mic is resampled into it; some browsers ignore the hint, so we read back the
  // rate actually used and write that into the WAV header (a wrong header = wrong pitch/speed).
  const ctx = new Ctor({ sampleRate: 16000 });
  const rate = ctx.sampleRate;
  const source = ctx.createMediaStreamSource(stream);
  const node = ctx.createScriptProcessor(4096, 1, 1);
  const chunks: Float32Array[] = [];
  node.onaudioprocess = (e) => {
    // Copy — the event's buffer is reused after the callback returns.
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
      const wav = encodeWav(chunks, rate);
      return new File([wav], `recording-${Date.now()}.wav`, { type: 'audio/wav' });
    },
    cancel() {
      teardown();
    },
  };
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
