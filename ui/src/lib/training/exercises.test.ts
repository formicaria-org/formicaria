import { expect, test } from 'vitest';
import { sameName } from './exercises';

test('an exercise typed in a session finds its card by name, a plural or a hyphen aside', () => {
  expect(sameName('Squat', 'Squats')).toBe(true);
  expect(sameName('push up', 'Push-Up')).toBe(true);
  expect(sameName('Biceps curl', 'Biceps curls')).toBe(true);
  expect(sameName('wall press-up', 'Wall press-up')).toBe(true);
  expect(sameName('Front squat', 'Back Squat')).toBe(false);
  expect(sameName('Press', 'Leg Press')).toBe(false);
});
