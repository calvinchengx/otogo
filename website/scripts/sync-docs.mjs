// Generates Starlight content from the canonical Markdown in /docs, keeping
// /docs as the single source of truth: its files stay pristine, so they render
// correctly on GitHub *and* on the site, and nobody has to decide which copy is
// authoritative. Run automatically before dev/build.
//
// For each docs/NN-name.md it derives the title from the leading H1, injects
// Starlight frontmatter, drops the now-duplicate H1, and rewrites links.
import { readdirSync, readFileSync, writeFileSync, rmSync, mkdirSync, existsSync, copyFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const REPO = join(here, '..', '..');
const DOCS_SRC = join(REPO, 'docs');
const OUT = join(here, '..', 'src', 'content', 'docs');
// Must match `base` in astro.config.mjs.
const BASE = '/otogo/';
const REPO_URL = 'https://github.com/calvinchengx/otogo';

const DOC_RE = /^\d{2}-.*\.md$/;
// `](NN-slug.md#anchor)` → `](/otogo/NN-slug/#anchor)`
const LINK_RE = /\]\((?:\.\/)?(\d{2}-[a-z0-9-]+)\.md(#[^)]*)?\)/g;
// Repo-relative links (`../examples/round.sh`) are correct on GitHub, where
// /docs sits one level under the root — but dead on the site, whose pages are
// served from flat /<base>/<slug>/ routes with nothing above them. Rewriting to
// an absolute GitHub URL is what keeps ONE source of truth working in both
// renderings. A path that resolves to nothing is reported rather than silently
// linked into a 404.
const REPO_LINK_RE = /\]\(\.\.\/([^)#]+)(#[^)]*)?\)/g;

function rewriteRepoLinks(md, where) {
  return md.replace(REPO_LINK_RE, (_m, path, anchor = '') => {
    const clean = path.replace(/\/+$/, '');
    if (!existsSync(join(REPO, clean))) {
      console.warn(`  ! ${where}: ../${clean} does not exist`);
    }
    const kind = path.endsWith('/') ? 'tree' : 'blob';
    return `](${REPO_URL}/${kind}/main/${clean}${anchor})`;
  });
}

function quote(s) {
  return `"${s.replace(/"/g, '\\"')}"`;
}

rmSync(OUT, { recursive: true, force: true });
mkdirSync(OUT, { recursive: true });

const files = readdirSync(DOCS_SRC).filter((f) => DOC_RE.test(f)).sort();
if (files.length === 0) throw new Error('no docs/NN-*.md found');

for (const file of files) {
  const raw = readFileSync(join(DOCS_SRC, file), 'utf8');
  const m = raw.match(/^#\s+(.+)$/m);
  if (!m) throw new Error(`${file} has no H1 to take a title from`);
  const title = m[1].trim();

  let body = raw.replace(/^#\s+.+$/m, '').replace(/^\s+/, '');
  body = body.replace(LINK_RE, (_x, slug, anchor = '') => `](${BASE}${slug}/${anchor})`);
  body = rewriteRepoLinks(body, file);

  const slug = file.replace(/\.md$/, '');
  writeFileSync(
    join(OUT, file),
    `---\ntitle: ${quote(title)}\n---\n\n${body}`,
  );
}
// The landing page is hand-written and lives outside the generated directory,
// which this script wipes on every run.
copyFileSync(join(here, '..', 'landing', 'index.mdx'), join(OUT, 'index.mdx'));

console.log(`  synced ${files.length} docs + landing → website/src/content/docs/`);
