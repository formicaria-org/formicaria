// **In the app's reader, following a link replaces the page** (`decisions.md#ui`, 2026-10-20,
// *Back is one step, everywhere*). The app shows this book in a frame named `manual-frame`; every
// page followed in a frame adds an entry to the history the app's Back walks, and the app cannot
// see those entries (the frame is sandboxed, from another origin). Left alone, Done stepped back one
// manual page instead of closing the reader, and Backs that did nothing followed. So inside the
// reader a link replaces the page: the reader is one step, and Back or Done closes it.
//
// Read on the web, the book is not in that frame and links behave as they always do.
(function () {
  if (window.name !== 'manual-frame') return;
  document.addEventListener(
    'click',
    function (e) {
      if (e.defaultPrevented || e.button !== 0) return;
      if (e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
      var a = e.target && e.target.closest ? e.target.closest('a[href]') : null;
      if (!a || (a.target && a.target !== '_self')) return;
      var url = new URL(a.getAttribute('href'), location.href);
      // The book's own pages only; anything else is left to the frame's rules.
      if (url.protocol !== location.protocol || url.host !== location.host) return;
      e.preventDefault();
      location.replace(url.href);
    },
    true,
  );
})();
