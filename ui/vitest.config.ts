import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { svelteTesting } from '@testing-library/svelte/vite';

// The test suite runs in jsdom because the render/component tests mutate a real
// DOM — that DOM is the whole point, and it needs no Tauri window, no WebKitGTK,
// just a document. Two kinds of tests share this config:
//   • render.test.ts / urgency.test.ts — plain .ts modules.
//   • App.flow.test.ts — mounts real Svelte components, so we load the Svelte
//     plugin (compiles .svelte) and `svelteTesting()` (resolves the browser build
//     and auto-unmounts between tests). The heavy lazy imports (KaTeX, Mermaid)
//     are mocked per-test so the run stays fast and hermetic.
//
// The cytoscape alias mirrors vite.config.ts so module resolution matches the
// real build; Mermaid is mocked in tests, so it is belt-and-braces.
const cytoscapeStub = new URL('./src/lib/cytoscape-stub.ts', import.meta.url).pathname;

export default defineConfig({
  plugins: [svelte(), svelteTesting()],
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.ts'],
    // See `src/test-setup.ts`: Testing Library's 1 s async default is shorter than a loaded machine
    // needs to mount the whole `App`, which showed up as a flake in an unrelated file.
    setupFiles: ['src/test-setup.ts'],
    clearMocks: true,
    // **`testTimeout` must exceed `asyncUtilTimeout`, or raising the latter does nothing.**
    // Both sat at 5000: a `findBy*` that needed to retry consumed the whole test budget, so the
    // *test* timed out first and the failure read `Test timed out in 5000ms` at the `it(...)` line
    // — no element name, no DOM, nothing to act on. Two `App.flow.test.ts` tests were failing this
    // way, at exactly 5004 ms.
    //
    // With headroom, a query that will never succeed now fails at its own 5 s mark with Testing
    // Library's real message (which element, and the DOM it searched), and a genuine hang still
    // fails — just later, and still visibly. The point is that the two budgets have to be ordered
    // to mean anything.
    testTimeout: 20000,
  },
  resolve: {
    alias: {
      cytoscape: cytoscapeStub,
      'cytoscape-cose-bilkent': cytoscapeStub,
      'cytoscape-fcose': cytoscapeStub,
    },
  },
});
