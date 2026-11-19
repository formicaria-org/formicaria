// The address bar's formicaria icon on a video or music page (the owner, 2026-11-19, on a YouTube
// page: *"it says only 12 images and no video"*): Save media first, and not the page's thumbnails.
import { afterEach, describe, expect, it } from 'vitest';
import { mediaPage, suggestions } from '../../addon/src/snip';

const imgs = (n: number) =>
  Array.from({ length: n }, () => {
    const i = document.createElement('img');
    Object.defineProperty(i, 'naturalWidth', { value: 320 });
    Object.defineProperty(i, 'naturalHeight', { value: 180 });
    return i;
  });

afterEach(() => {
  document.body.replaceChildren();
});

describe('what the address bar icon offers', () => {
  it('a page with a video offers Save media, and not its thumbnails', () => {
    document.body.append(document.createElement('video'), ...imgs(12));
    expect(mediaPage()).toBe(true);
    const labels = suggestions().map((s) => s.label);
    expect(labels[0]).toBe('Save media…');
    expect(labels.some((l) => /images/.test(l))).toBe(false);
  });

  it('a page of pictures still offers its images, and no Save media', () => {
    document.body.append(...imgs(3));
    expect(mediaPage()).toBe(false);
    const labels = suggestions().map((s) => s.label);
    expect(labels).toContain('The 3 images');
    expect(labels).not.toContain('Save media…');
  });
});
