// Build the static site into ./dist:
//
//   index.html     the landing page
//   docs.html      the documentation home
//   <slug>.html    one page per entry in render/nav.ts
//   searchindex.json, theme/style.css, assets/*, .nojekyll
//
// `bun run build.ts`, and that is the whole toolchain — the markdown under
// ../docs is the source, and it stays readable on GitHub either way.

import { renderLanding, renderDocsIndex } from "./render/landing";
import { renderDoc, headingsFor, plain, sidebar } from "./render/doc";
import { pages, groupOf } from "./render/nav";
import { favicon, ogImage } from "./assets/svg";

const root = import.meta.dir;
const docs = `${root}/../docs`;
const out = `${root}/dist`;

const write = (rel: string, body: string) => Bun.write(`${out}/${rel}`, body);

console.log("building the Tailor site …");

await write("index.html", renderLanding());
await write("docs.html", renderDocsIndex(sidebar("docs.html")));
console.log("  page index.html");
console.log("  page docs.html");

type Entry = {
  title: string;
  out: string;
  group: string;
  blurb: string;
  headings: { text: string; id: string }[];
  text: string;
};
const index: Entry[] = [];

for (const page of pages()) {
  const md = await Bun.file(`${docs}/${page.src}`).text();

  // `readme.md` is the docs home, and that page is written here rather than
  // rendered — it is an index, and an index of itself reads badly.
  if (page.out !== "docs.html") {
    await write(page.out, renderDoc(page, md));
    console.log(`  page ${page.src} -> ${page.out}`);
  }

  index.push({
    title: page.title,
    out: page.out,
    group: groupOf(page.out),
    blurb: page.blurb,
    headings: headingsFor(md).map(({ text, id }) => ({ text, id })),
    text: plain(md).slice(0, 400),
  });
}

await write("searchindex.json", JSON.stringify(index));
await write("theme/style.css", await Bun.file(`${root}/theme/style.css`).text());
await write("assets/favicon.svg", favicon());
await write("assets/og.svg", ogImage());
// GitHub Pages runs Jekyll unless told not to, and Jekyll eats directories
// beginning with an underscore.
await write(".nojekyll", "");

console.log(`done -> ${out}`);
