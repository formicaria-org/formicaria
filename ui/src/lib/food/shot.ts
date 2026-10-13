/// A photograph made ready for the assistant to read (`decisions.md#agent`, 2026-10-13, *a
/// label is read from a photograph, and the person checks it*).
///
/// A phone's picture is twelve megapixels; a reader spends time on every one, and a label that
/// fills the frame is as legible at a thousand. So the picture is drawn small here, upright as
/// the camera meant it, turned by as many quarter turns as the person asked, and handed over as
/// a JPEG. The original is not changed and nothing is kept.

/** The longest side of what is sent. Measured on the laptop's reader, 2026-10-13: the same
 *  label came back exact at 768 and at 1024, in 17 and 33 seconds; at 1700 it took 74. */
export const SIDE = 1024;

/** `image`, no larger than `SIDE` on its longest side and turned `turns` quarter turns
 *  clockwise, as a JPEG `data:` address. */
export async function shrink(image: Blob, turns = 0): Promise<string> {
  const bitmap = await createImageBitmap(image, { imageOrientation: 'from-image' });
  try {
    const scale = Math.min(1, SIDE / Math.max(bitmap.width, bitmap.height));
    const w = Math.max(1, Math.round(bitmap.width * scale));
    const h = Math.max(1, Math.round(bitmap.height * scale));
    const quarter = ((turns % 4) + 4) % 4;
    const canvas = document.createElement('canvas');
    canvas.width = quarter % 2 ? h : w;
    canvas.height = quarter % 2 ? w : h;
    const ctx = canvas.getContext('2d');
    if (!ctx) throw new Error('This device could not prepare the picture.');
    ctx.translate(canvas.width / 2, canvas.height / 2);
    ctx.rotate((quarter * Math.PI) / 2);
    ctx.drawImage(bitmap, -w / 2, -h / 2, w, h);
    return canvas.toDataURL('image/jpeg', 0.85);
  } finally {
    bitmap.close();
  }
}
