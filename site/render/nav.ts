// Which documentation pages the site publishes, in the order they are meant
// to be read, and the groups the sidebar shows them under.
//
// A list rather than a directory walk. `docs/` holds pages for two audiences —
// people using Tailor, and people working on it — and only the first belong on
// the site. A walk would publish `release.md` to anyone who found it.

export type Page = {
  /** File under `../docs`. */
  src: string;
  /** File written into `dist`. */
  out: string;
  /** What the sidebar and the tab title call it. */
  title: string;
  /** One line, for the docs index and the search entry. */
  blurb: string;
};

export type Group = { name: string; pages: Page[] };

export const GROUPS: Group[] = [
  {
    name: "Start here",
    pages: [
      {
        src: "readme.md",
        out: "docs.html",
        title: "Overview",
        blurb: "What Tailor is, the window, and what it is not.",
      },
      {
        src: "tutorial.md",
        out: "tutorial.html",
        title: "Tutorial",
        blurb: "Build an app end to end, and run what comes out.",
      },
      {
        src: "running.md",
        out: "running.html",
        title: "Running your design",
        blurb: "Run, Build, Stop, the console, and what the compiler says.",
      },
    ],
  },
  {
    name: "Designing",
    pages: [
      {
        src: "canvas.md",
        out: "canvas.html",
        title: "The canvas",
        blurb: "Modes, selection, resizing, layout, snapping, the live window.",
      },
      {
        src: "components.md",
        out: "components.html",
        title: "Components & slots",
        blurb: "The catalog, slots, drawn containers, your own components.",
      },
      {
        src: "state.md",
        out: "state.html",
        title: "State & actions",
        blurb: "Signals, two-way binding, events, and writing an action.",
      },
    ],
  },
  {
    name: "The code",
    pages: [
      {
        src: "codegen.md",
        out: "codegen.html",
        title: "What gets generated",
        blurb: "The output, the flavours, export, the file format, modules.",
      },
      {
        src: "zed.md",
        out: "zed.html",
        title: "Zed & other editors",
        blurb: "Jumping between a component and its code, both directions.",
      },
      {
        src: "mcp.md",
        out: "mcp.html",
        title: "The MCP server",
        blurb: "Driving the same document from an agent.",
      },
    ],
  },
  {
    name: "Extending",
    pages: [
      {
        src: "libraries.md",
        out: "libraries.html",
        title: "Component libraries",
        blurb: "What a target library is, and how to add one.",
      },
    ],
  },
];

export function pages(): Page[] {
  return GROUPS.flatMap((group) => group.pages);
}

export function groupOf(out: string): string {
  return GROUPS.find((g) => g.pages.some((p) => p.out === out))?.name ?? "";
}

/** Where a `docs/*.md` link points once both ends are HTML. */
export function rewriteLink(href: string): string {
  const [file, hash] = href.split("#");
  const page = pages().find((p) => p.src === file);
  if (!page) return href;
  return hash ? `${page.out}#${hash}` : page.out;
}
