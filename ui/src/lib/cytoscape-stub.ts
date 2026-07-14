// Empty stand-in for cytoscape and its layout extensions (cose-bilkent, fcose).
// Mermaid imports these ONLY for the architecture and mindmap diagrams — the two
// diagram types we deliberately drop from the bundle to save ~0.5 MB of graph
// libraries (see ui/vite.config.ts, which aliases those packages here). The stub
// only needs to be import-safe: a callable with a no-op `.use`, so Mermaid's
// `cytoscape.use(extension)` registration doesn't throw at module-evaluation
// time. Anything that actually renders one of those diagrams throws later and is
// caught per-block in render.ts, leaving the diagram's code fence as written.
type CytoscapeStub = {
  (...args: unknown[]): CytoscapeStub;
  use: (...args: unknown[]) => void;
};

const stub = ((..._args: unknown[]) => stub) as CytoscapeStub;
stub.use = () => {};

export default stub;
