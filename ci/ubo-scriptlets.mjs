// **uBlock Origin's scriptlets as `adblock-rust` resources** (`privacy-plan-2026-11-19.md` 3.3), the
// way Brave's packager makes them (`brave-core-crx-packager/lib/adBlockRustUtils.js`): uBO's
// `src/js/resources/scriptlets.js` imported as the ES module it is, each scriptlet's function as its
// source, base64, with its aliases and dependencies. Usage: node ci/ubo-scriptlets.mjs <uBlock dir> <out.json>
import { writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const [dir, out] = process.argv.slice(2);
const { builtinScriptlets } = await import(pathToFileURL(resolve(join(dir, 'src/js/resources/scriptlets.js'))).href);
const names = new Set(builtinScriptlets.map((s) => s.name));
for (const s of builtinScriptlets)
  for (const d of s.dependencies ?? []) if (!names.has(d)) throw new Error(`${s.name} needs ${d}, which is missing`);
const resources = builtinScriptlets.map((s) => ({
  name: s.name,
  aliases: s.aliases ?? [],
  kind: { mime: 'application/javascript' },
  content: Buffer.from(s.fn.toString()).toString('base64'),
  dependencies: s.dependencies ?? [],
}));
writeFileSync(out, JSON.stringify(resources));
console.log(`ubo-scriptlets: ${resources.length} scriptlets`);
