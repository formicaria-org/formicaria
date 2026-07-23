import { describe, it, expect } from 'vitest';
import { encodeWav, resampleTo } from './record';

// `encodeWav` is the pure core of in-app recording (the getUserMedia/AudioContext side needs a real
// mic, so it isn't unit-tested). A wrong header is the classic bug — it makes whisper hear the wrong
// pitch/speed — so pin the header bytes exactly, plus the 16-bit LE sample conversion.

const str = (view: DataView, off: number, len: number) =>
  Array.from({ length: len }, (_, i) => String.fromCharCode(view.getUint8(off + i))).join('');

describe('encodeWav', () => {
  it('writes a canonical 16-bit mono PCM WAV header at the given sample rate', () => {
    const rate = 16000;
    const wav = encodeWav([new Float32Array([0, 0.5, -0.5, 1, -1])], rate);
    const v = new DataView(wav);
    expect(str(v, 0, 4)).toBe('RIFF');
    expect(str(v, 8, 4)).toBe('WAVE');
    expect(str(v, 12, 4)).toBe('fmt ');
    expect(v.getUint16(20, true)).toBe(1); // PCM
    expect(v.getUint16(22, true)).toBe(1); // mono
    expect(v.getUint32(24, true)).toBe(rate); // sample rate lands in the header
    expect(v.getUint32(28, true)).toBe(rate * 2); // byte rate (mono, 16-bit)
    expect(v.getUint16(32, true)).toBe(2); // block align
    expect(v.getUint16(34, true)).toBe(16); // bits/sample
    expect(str(v, 36, 4)).toBe('data');
    // 5 samples × 2 bytes.
    expect(v.getUint32(40, true)).toBe(10);
    expect(wav.byteLength).toBe(44 + 10);
  });

  it('converts float samples in [-1,1] to 16-bit little-endian and clamps out-of-range', () => {
    const wav = encodeWav([new Float32Array([0, 1, -1, 2, -2])], 16000);
    const v = new DataView(wav);
    expect(v.getInt16(44, true)).toBe(0);
    expect(v.getInt16(46, true)).toBe(32767); // +1 → max
    expect(v.getInt16(48, true)).toBe(-32768); // -1 → min
    expect(v.getInt16(50, true)).toBe(32767); // +2 clamped to +1
    expect(v.getInt16(52, true)).toBe(-32768); // -2 clamped to -1
  });

  it('concatenates multiple chunks in order', () => {
    const wav = encodeWav([new Float32Array([1]), new Float32Array([-1])], 8000);
    const v = new DataView(wav);
    expect(v.getUint32(40, true)).toBe(4); // 2 samples
    expect(v.getInt16(44, true)).toBe(32767);
    expect(v.getInt16(46, true)).toBe(-32768);
  });
});

describe('resampleTo', () => {
  it('is a no-op when the rates match', () => {
    const input = new Float32Array([0.1, 0.2, 0.3]);
    expect(resampleTo(input, 16000, 16000)).toBe(input);
  });

  it('halves the sample count downsampling 32k → 16k', () => {
    const input = new Float32Array([0, 1, 0, 1, 0, 1, 0, 1]); // 8 samples at 32k
    const out = resampleTo(input, 32000, 16000);
    expect(out.length).toBe(4); // → 4 samples at 16k
  });

  it('doubles the sample count upsampling 8k → 16k and interpolates', () => {
    const out = resampleTo(new Float32Array([0, 1]), 8000, 16000);
    expect(out.length).toBe(4);
    expect(out[0]).toBeCloseTo(0);
    expect(out[2]).toBeCloseTo(1); // the original second sample lands mid-way
  });

  it('handles an empty buffer', () => {
    expect(resampleTo(new Float32Array([]), 48000, 16000).length).toBe(0);
  });
});
