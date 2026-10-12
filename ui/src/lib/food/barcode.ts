/// The number under a barcode: is it one, by its own check digit?
///
/// GS1's rule (`sources-2026-10-11.md` §6c; gs1.org, *How to calculate a check digit manually*):
/// from the digit next to the check digit leftwards, multiply alternately by 3 and 1, add up, and
/// "subtract the sum from nearest equal or higher multiple of ten". The same for 8, 12, 13 and 14
/// digits, since the weights are counted from the right.
export function digitsOf(text: string): string {
  return text.replace(/[\s-]/g, '');
}

export function isBarcode(text: string): boolean {
  const d = digitsOf(text);
  if (!/^\d+$/.test(d) || ![8, 12, 13, 14].includes(d.length)) return false;
  let sum = 0;
  for (let i = d.length - 2, w = 3; i >= 0; i--, w = w === 3 ? 1 : 3) sum += Number(d[i]) * w;
  return (10 - (sum % 10)) % 10 === Number(d[d.length - 1]);
}

/** One spelling for the same product: a 12-digit code is the 13-digit one with a zero in front. */
export const sameCode = (text: string): string => {
  const d = digitsOf(text);
  return d.length === 12 ? `0${d}` : d;
};
