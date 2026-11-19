// **Fingerprinting, answered with made-up detail, on the phone's web pages**
// (`privacy-plan-2026-11-19.md`, the comparison's last gap): a site that reads a canvas, a WebGL
// drawing or an audio buffer to tell this phone apart gets values changed by a tiny, invisible amount,
// the same on that site for this session and different on every other site, so its readings no
// longer follow the person from site to site (Brave's "farbling"). The graphics card's own name is not
// given. Built on its own (`addon/build.mjs`) into `dist/page/fingerprint.js`; formicaria's page view
// runs it at each page's start while *Block trackers* is on, with this session's secret
// (`__fmSeed`, never sent anywhere).

/** A small, fast generator (mulberry32): the same seed gives the same sequence. */
export function prng(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** This session's seed for a site: the secret and the site's name, hashed (FNV-1a). */
export function seedFor(secret: string, host: string): number {
  let h = 0x811c9dc5;
  for (const c of `${secret}|${host.replace(/^www\./, '')}`) {
    h ^= c.charCodeAt(0);
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  return h >>> 0;
}

/** Pixels (RGBA) with the lowest bit of a few colour values flipped, as the seed says; alpha kept. */
export function perturbPixels(data: Uint8ClampedArray, seed: number): Uint8ClampedArray {
  const r = prng(seed);
  const n = Math.max(1, Math.floor(data.length / 4 / 50));
  for (let i = 0; i < n; i++) {
    const px = Math.floor(r() * (data.length / 4));
    const ch = Math.floor(r() * 3);
    data[px * 4 + ch] ^= 1;
  }
  return data;
}

/** Audio samples moved by at most one part in ten million, as the seed says. */
export function perturbSamples(data: Float32Array, seed: number): Float32Array {
  const r = prng(seed);
  for (let i = 0; i < data.length; i += 1 + Math.floor(r() * 16)) data[i] += (r() - 0.5) * 1e-7;
  return data;
}

type W = typeof globalThis & { __fmSeed?: string; __fmFingerprint?: boolean };
const w = globalThis as W;

if (typeof window !== 'undefined' && w.__fmSeed && !w.__fmFingerprint) {
  w.__fmFingerprint = true;
  const seed = seedFor(w.__fmSeed, location.hostname);
  const define = (o: object, k: string, v: unknown) => {
    try {
      Object.defineProperty(o, k, { value: v, configurable: true, writable: true });
    } catch {
      /* a property the page view does not let go of */
    }
  };

  // Canvas: what is read back is changed, never what is shown.
  const C2D = window.CanvasRenderingContext2D?.prototype;
  if (C2D) {
    const getImageData = C2D.getImageData;
    define(
      C2D,
      'getImageData',
      function (this: CanvasRenderingContext2D, ...a: Parameters<typeof getImageData>) {
        const img = getImageData.apply(this, a);
        perturbPixels(img.data, seed);
        return img;
      },
    );
    const copy = (c: HTMLCanvasElement): HTMLCanvasElement | null => {
      try {
        if (!c.width || !c.height) return null;
        const d = document.createElement('canvas');
        d.width = c.width;
        d.height = c.height;
        const x = d.getContext('2d');
        if (!x) return null;
        x.drawImage(c, 0, 0);
        const img = getImageData.call(x, 0, 0, d.width, d.height);
        perturbPixels(img.data, seed);
        x.putImageData(img, 0, 0);
        return d;
      } catch {
        return null;
      }
    };
    const HC = window.HTMLCanvasElement.prototype;
    const toDataURL = HC.toDataURL;
    define(HC, 'toDataURL', function (this: HTMLCanvasElement, ...a: Parameters<typeof toDataURL>) {
      return toDataURL.apply(copy(this) ?? this, a);
    });
    const toBlob = HC.toBlob;
    define(HC, 'toBlob', function (this: HTMLCanvasElement, ...a: Parameters<typeof toBlob>) {
      return toBlob.apply(copy(this) ?? this, a);
    });
  }

  // WebGL: pixels read back changed; the graphics card's own name not given.
  for (const G of [window.WebGLRenderingContext, window.WebGL2RenderingContext]) {
    const P = G?.prototype as (WebGLRenderingContext & Record<string, unknown>) | undefined;
    if (!P) continue;
    const getParameter = P.getParameter;
    define(P, 'getParameter', function (this: WebGLRenderingContext, p: number) {
      if (p === 0x9245) return 'WebKit'; // UNMASKED_VENDOR_WEBGL
      if (p === 0x9246) return 'WebKit WebGL'; // UNMASKED_RENDERER_WEBGL
      return getParameter.call(this, p);
    });
    const readPixels = P.readPixels as (...a: unknown[]) => void;
    define(P, 'readPixels', function (this: WebGLRenderingContext, ...a: unknown[]) {
      readPixels.apply(this, a);
      const out = a[6];
      if (out instanceof Uint8Array)
        perturbPixels(new Uint8ClampedArray(out.buffer, out.byteOffset, out.byteLength), seed);
    });
  }

  // Audio: a buffer's samples, and an analyser's readings, moved by an inaudible amount.
  const AB = window.AudioBuffer?.prototype;
  if (AB) {
    const getChannelData = AB.getChannelData;
    const done = new WeakSet<Float32Array>();
    define(AB, 'getChannelData', function (this: AudioBuffer, ch: number) {
      const d = getChannelData.call(this, ch);
      if (!done.has(d)) {
        done.add(d);
        perturbSamples(d, seed + ch);
      }
      return d;
    });
  }
  const AN = window.AnalyserNode?.prototype;
  if (AN) {
    const getFloat = AN.getFloatFrequencyData;
    define(
      AN,
      'getFloatFrequencyData',
      function (this: AnalyserNode, a: Float32Array<ArrayBuffer>) {
        getFloat.call(this, a);
        perturbSamples(a, seed ^ 0x5bd1e995);
      },
    );
  }
}
