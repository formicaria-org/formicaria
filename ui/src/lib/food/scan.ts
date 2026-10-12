/// Read a barcode out of a picture (`decisions.md#ui`, 2026-10-12, *a barcode is read from a
/// photograph, not from a live camera*).
///
/// The reader (`zxing-wasm`, about 1 MB) is asked for only when a person first uses it, and its
/// WebAssembly file is served by the app: by default the library would fetch it from a CDN, which
/// the phone shell's content policy forbids and an offline app must not need. The picture is
/// handed to the reader in memory and goes nowhere else.
import { digitsOf, isBarcode } from './barcode';

/** The number under the barcode in `image`, when there is one and its check digit holds. */
export async function barcodeIn(image: Blob): Promise<string | null> {
  const [{ readBarcodes, prepareZXingModule }, { default: wasm }] = await Promise.all([
    import('zxing-wasm/reader'),
    import('zxing-wasm/reader/zxing_reader.wasm?url'),
  ]);
  prepareZXingModule({
    overrides: {
      locateFile: (path: string, prefix: string) => (path.endsWith('.wasm') ? wasm : prefix + path),
    },
  });
  const found = await readBarcodes(image, {
    // Retail codes only: EAN-13, EAN-8, UPC-A, UPC-E.
    formats: ['EANUPC'],
    tryHarder: true,
    maxNumberOfSymbols: 4,
  });
  for (const f of found) {
    const digits = digitsOf(f.text);
    if (isBarcode(digits)) return digits;
  }
  return null;
}
