// Point Excalidraw at fonts we serve ourselves (see ui/scripts/copy-excalidraw-fonts.mjs).
//
// This lives in its own file rather than inline in index.html for one reason: the inline
// form would force `script-src 'unsafe-inline'` in fm-serve's Content-Security-Policy, and
// that is the single clause that turns any HTML-sanitiser bypass into script execution.
// One extra request on load buys a policy with no inline scripts at all.
//
// Loaded as a classic (non-module) script, so it runs *before* the deferred module bundle
// that reads it.
window.EXCALIDRAW_ASSET_PATH = "/";
