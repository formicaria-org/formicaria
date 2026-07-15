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
    clearMocks: true,
  },
  resolve: {
    alias: {
      cytoscape: cytoscapeStub,
      'cytoscape-cose-bilkent': cytoscapeStub,
      'cytoscape-fcose': cytoscapeStub,
    },
  },
});
