import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// Drop Mermaid's cytoscape-backed diagrams (architecture, mindmap) from the
// bundle. Those two diagram types are the only Mermaid features that pull in
// cytoscape + its layout extensions — ~0.5 MB of graph libraries this notebook
// never uses. Aliasing the packages to an empty stub keeps the real libs out of
// the build entirely; if a note ever contains one of those diagrams it fails to
// render and render.ts's per-block try/catch leaves the code fence as-is
// (graceful). Every everyday diagram (flowchart, sequence, gantt, class, state,
// ER, pie, …) uses dagre, not cytoscape, and is unaffected.
const cytoscapeStub = new URL('./src/lib/cytoscape-stub.ts', import.meta.url).pathname;

// Tauri drives this dev server on a fixed port (see tauri.conf.json devUrl) and
// wants its own console output left intact.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  resolve: {
    alias: {
      cytoscape: cytoscapeStub,
      'cytoscape-cose-bilkent': cytoscapeStub,
      'cytoscape-fcose': cytoscapeStub,
    },
  },
  build: { target: 'es2022', outDir: 'dist', emptyOutDir: true },
});
