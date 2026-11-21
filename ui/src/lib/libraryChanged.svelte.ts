// **The library changed** (2026-11-21, found in the emulator and on a phone: a playlist made by Save
// media while Music was open did not show — Music reloaded only when its own feed changed, and a new
// playlist does not change the Songs feed). Raised wherever songs or playlists are written outside
// Music's own sheets: a saved file filed, a note's tracks made songs, the computer's change stream.
// Music and Videos read it as a signal to load again.
export const library = $state({ n: 0 });
export const touched = () => void (library.n += 1);
