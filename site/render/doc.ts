// One `docs/*.md` file as a page.
//
// The markdown is the source and stays readable on GitHub, so nothing here
// invents syntax. What it does do is fix up the two things that only make
// sense once both ends are HTML: a link to `codegen.md` has to become
// `codegen.html`, and every heading needs an id so the sidebar and the
// keyboard-shortcut link from the app's Help menu can point at one.

import { Marked } from "marked";
import { markedHighlight } from "marked-highlight";
import hljs from "highlight.js";

import { shell, escape } from "./shell";
import { GROUPS, pages, rewriteLink, type Page } from "./nav";

export function highlight(code: string, language: string): string {
  const lang = hljs.getLanguage(language) ? language : "plaintext";
  return hljs.highlight(code, { language: lang }).value;
}

const marked = new Marked(
  markedHighlight({
    emptyLangClass: "hljs",
    langPrefix: "hljs language-",
    highlight,
  }),
);

/** A heading's anchor: lowercase, words joined by hyphens. */
export function slug(text: string): string {
  return text
    .toLowerCase()
    .replace(/`/g, "")
    .replace(/[^\w\s-]/g, "")
    .trim()
    .replace(/\s+/g, "-");
}

export type Heading = { text: string; id: string; depth: number };

export function headingsFor(md: string): Heading[] {
  const out: Heading[] = [];
  for (const line of md.split("\n")) {
    const match = /^(#{2,3})\s+(.+?)\s*$/.exec(line);
    if (!match) continue;
    const text = match[2].replace(/`/g, "");
    out.push({ text, id: slug(text), depth: match[1].length });
  }
  return out;
}

/** The sidebar, with `current` marked. */
export function sidebar(current: string): string {
  const groups = GROUPS.map((group) => {
    const links = group.pages
      .map(
        (page) =>
          `<a href="${page.out}"${
            page.out === current ? ' class="here"' : ""
          }>${page.title}</a>`,
      )
      .join("\n    ");
    return `  <h4>${group.name}</h4>\n    ${links}`;
  }).join("\n");
  return `<nav class="side" aria-label="Documentation">\n${groups}\n</nav>`;
}

/** Previous and next, in reading order, so a page is never a dead end. */
function pager(current: string): string {
  const all = pages();
  const at = all.findIndex((page) => page.out === current);
  if (at < 0) return "";
  const link = (page: Page | undefined, label: string, align: string) =>
    page
      ? `<a href="${page.out}" style="text-align:${align}"><span>${label}</span>${page.title}</a>`
      : "<span></span>";
  return `<div class="pager">
  ${link(all[at - 1], "Previous", "left")}
  ${link(all[at + 1], "Next", "right")}
</div>`;
}

export function renderDoc(page: Page, md: string): string {
  // The first `# Heading` becomes the page title; the rest is the body. Left
  // in, it would be a second H1 under the one the shell already implies.
  const withoutTitle = md.replace(/^#\s+.*\n/, "");
  let html = marked.parse(withoutTitle) as string;

  // `codegen.md` -> `codegen.html`, keeping any fragment.
  html = html.replace(
    /href="([^":]+\.md)(#[^"]*)?"/g,
    (_all, file: string, hash = "") => `href="${rewriteLink(file + hash)}"`,
  );

  // An id and a permalink on every heading the sidebar might point at.
  html = html.replace(
    /<h([23])>(.*?)<\/h\1>/g,
    (_all, depth: string, inner: string) => {
      const id = slug(inner.replace(/<[^>]+>/g, ""));
      return `<h${depth} id="${id}">${inner}<a class="anchor" href="#${id}" aria-label="Link to this section">#</a></h${depth}>`;
    },
  );

  const body = `<div class="docs">
${sidebar(page.out)}
<article class="prose">
  <h1>${escape(page.title)}</h1>
${html}
${pager(page.out)}
</article>
</div>`;

  return shell(
    {
      title: page.title,
      description: page.blurb,
      current: "docs",
      bodyClass: "doc",
    },
    body,
  );
}

/** The plain text of a page, for the search index. */
export function plain(md: string): string {
  return md
    .replace(/```[\s\S]*?```/g, " ")
    .replace(/^#{1,6}\s+.*$/gm, " ")
    .replace(/\[([^\]]+)\]\([^)]*\)/g, "$1")
    .replace(/[`*_>#|]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}
