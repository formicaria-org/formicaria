// Side-effect CSS imports (KaTeX styles, loaded lazily with the math renderer).
declare module '*.css';

// KaTeX's auto-render helper — dist path has no bundled types.
declare module 'katex/dist/contrib/auto-render.js' {
  const renderMathInElement: (
    el: HTMLElement,
    options?: {
      delimiters?: { left: string; right: string; display: boolean }[];
      throwOnError?: boolean;
      ignoredTags?: string[];
    },
  ) => void;
  export default renderMathInElement;
}
