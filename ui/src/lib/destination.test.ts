import { describe, expect, it } from 'vitest';
import { reachOf, shortDest } from './destination';

// The panel's honesty rests on this: it must never say "off this machine" about
// a folder on the same disk. Both tiers are covered, because a restic repo and a
// git remote spell "local" in opposite ways.
describe('reachOf', () => {
  it('reads git remotes', () => {
    expect(reachOf('git@github.com:you/notes.git')).toBe('remote');
    expect(reachOf('https://github.com/you/notes.git')).toBe('remote');
    expect(reachOf('ssh://git@host/srv/notes.git')).toBe('remote');
    // A bare path or file:// is a git remote that never leaves the machine.
    expect(reachOf('/srv/notes.git')).toBe('local');
    expect(reachOf('file:///srv/notes.git')).toBe('local');
    expect(reachOf('../notes.git')).toBe('local');
  });

  it('reads restic repos, where a bare path is the local case', () => {
    expect(reachOf('/mnt/backup')).toBe('local');
    expect(reachOf('~/backup')).toBe('local');
    expect(reachOf('s3:s3.amazonaws.com/bucket')).toBe('remote');
    expect(reachOf('b2:bucket:path')).toBe('remote');
    expect(reachOf('sftp:user@host:/srv/restic')).toBe('remote');
    expect(reachOf('rclone:remote:path')).toBe('remote');
    expect(reachOf('rest:https://host/repo')).toBe('remote');
  });

  it('treats absence as unset, not as a destination', () => {
    expect(reachOf(null)).toBe('unset');
    expect(reachOf(undefined)).toBe('unset');
    expect(reachOf('   ')).toBe('unset');
  });

  it('does not mistake a windows drive letter for a scheme', () => {
    expect(reachOf('C:\\backup')).toBe('local');
  });
});

describe('shortDest', () => {
  it('strips scheme, credentials and .git noise', () => {
    expect(shortDest('https://github.com/you/notes.git')).toBe('github.com/you/notes');
    expect(shortDest('git@github.com:you/notes.git')).toBe('github.com:you/notes');
    // A token pasted into the URL must never be echoed back to the screen.
    expect(shortDest('https://user:ghp_secret@github.com/you/notes.git')).toBe(
      'github.com/you/notes',
    );
  });

  it('leaves a plain path alone', () => {
    expect(shortDest('/mnt/backup')).toBe('/mnt/backup');
  });
});
