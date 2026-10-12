import { expect, test } from 'vitest';
import { isBarcode, sameCode } from './barcode';

test('a barcode is checked by its own last digit, as GS1 works it out', () => {
  // GS1's worked example: 629104150021 → check digit 3.
  expect(isBarcode('6291041500213')).toBe(true);
  expect(isBarcode('6291041500214')).toBe(false);
  expect(isBarcode('3017624010701')).toBe(true);
  expect(isBarcode('96385074')).toBe(true); // eight digits
  expect(isBarcode('036000291452')).toBe(true); // twelve
  expect(isBarcode('12345')).toBe(false);
  expect(isBarcode('30176240107ab')).toBe(false);
  expect(isBarcode(' 3017 6240 10701 ')).toBe(true);
});

test('twelve digits and thirteen with a zero in front are one product', () => {
  expect(sameCode('036000291452')).toBe('0036000291452');
  expect(sameCode('3017624010701')).toBe('3017624010701');
});
