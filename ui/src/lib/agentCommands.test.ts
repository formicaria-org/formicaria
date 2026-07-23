import { describe, it, expect } from 'vitest';
import { withCommand, AGENT_COMMANDS } from './agentCommands';

describe('withCommand — tapping a command chip', () => {
  it('addresses an online assistant when the draft mentions none', () => {
    // Empty draft + a known agent → the message is addressed and the command inserted.
    expect(withCommand('', '/research', 'qwen3-4b-2507')).toBe('@qwen3-4b-2507 /research ');
  });

  it('keeps the user question and inserts the command after it', () => {
    expect(withCommand('how do mRNA vaccines work', '/research', 'lfm2.5-1.2b')).toBe(
      '@lfm2.5-1.2b how do mRNA vaccines work /research ',
    );
  });

  it('does not add a second @mention when one is already present', () => {
    expect(withCommand('@math-helper what is 2+2', '/search', 'qwen')).toBe(
      '@math-helper what is 2+2 /search ',
    );
  });

  it('is idempotent — tapping the same command twice does not duplicate it', () => {
    const once = withCommand('@a explain RAG', '/research', 'a');
    expect(withCommand(once.trim(), '/research', 'a')).toBe('@a explain RAG /research ');
  });

  it('still inserts the command unaddressed when no assistant is known', () => {
    // No agent to address → the command is inserted anyway; the send path shows the "no assistant" notice.
    expect(withCommand('some question', '/research', undefined)).toBe('some question /research ');
  });

  it('offers /research first, and /transcribe is a normal command chip (no per-note button)', () => {
    expect(AGENT_COMMANDS[0].cmd).toBe('/research');
    expect(AGENT_COMMANDS.map((c) => c.cmd)).toEqual(['/research', '/search', '/propose', '/transcribe']);
  });
});
