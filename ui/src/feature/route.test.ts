import { describe, expect, it } from 'vitest';
import { addressUrl, itemUrl, parseRoute } from './route';

describe('a feature page knows what it shows from its address', () => {
  it('reads notes, features, views and built-in views', () => {
    const id = '01JABCDEFGHJKMNPQRSTVWXYZ0';
    expect(parseRoute(`/f/note/${id}`)).toEqual({ kind: 'note', id });
    expect(parseRoute('/f/group/Food')).toMatchObject({ kind: 'group', name: 'Food' });
    expect(parseRoute('/f/view/Active')).toEqual({ kind: 'view', name: 'Active' });
    expect(parseRoute('/f/agenda')).toEqual({ kind: 'builtin', pane: 'agenda' });
    // Anything else: the notes.
    expect(parseRoute('/f/nowhere')).toMatchObject({ kind: 'group', name: 'Library' });
  });
  it('turns formicaria’s item addresses into page addresses, and back', () => {
    expect(addressUrl('note:01JABCDEFGHJKMNPQRSTVWXYZ0')).toBe(
      '/f/note/01JABCDEFGHJKMNPQRSTVWXYZ0',
    );
    expect(addressUrl('view:group:Food')).toBe('/f/group/Food');
    expect(addressUrl('view:saved:Weekly review')).toBe('/f/view/Weekly%20review');
    expect(addressUrl('view:agenda')).toBe('/f/agenda');
    expect(itemUrl(parseRoute('/f/group/Tasks'))).toBe('/f/group/Tasks');
  });
});
