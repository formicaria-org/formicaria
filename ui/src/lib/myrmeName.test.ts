// A device's name for its Myrme (2026-10-18): set in Settings, and what this device's messages are
// stamped with from then on — so `@Myrme` written here is answered by this device's copy.
import { expect, test } from 'vitest';
import { agentStatus, setAssistantName, thisDevice } from './ipc';

test('a device named in Settings stamps its messages with that name', async () => {
  expect(await setAssistantName('Office desktop!')).toBe('office-desktop');
  expect(thisDevice()).toBe('office-desktop');
  expect((await agentStatus()).device).toBe('office-desktop');
  // Nothing usable keeps the default.
  expect(await setAssistantName('  —  ')).toBe('laptop');
});
