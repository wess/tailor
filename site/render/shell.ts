// The page around the page: `<head>`, the header, the footer.
//
// One skeleton for the landing page and every doc page, so the header cannot
// drift between them and there is one place that knows the description, the
// social card, and where the stylesheet lives.

const SITE = "https://wess.io/tailor";
const DESCRIPTION =
  "A visual interface builder for gpui. Lay out a screen from real components, " +
  "write the code behind it, press Run — and take the Rust with you.";

export type ShellOptions = {
  title: string;
  /** Overrides the site description in the meta tags. */
  description?: string;
  /** The nav item to mark current. */
  current?: "docs" | "github";
  /** Extra markup before `</head>` — a page's own JSON-LD, a preload. */
  head?: string;
  /** Extra markup before `</body>`. */
  scripts?: string;
  /** Applied to `<body>`, so a page can opt into a layout. */
  bodyClass?: string;
};

export function shell(options: ShellOptions, body: string): string {
  const description = options.description ?? DESCRIPTION;
  const title =
    options.title === "Tailor" ? "Tailor" : `${options.title} · Tailor`;
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${escape(title)}</title>
<meta name="description" content="${escape(description)}">
<meta name="color-scheme" content="dark">
<meta property="og:type" content="website">
<meta property="og:title" content="${escape(title)}">
<meta property="og:description" content="${escape(description)}">
<meta property="og:image" content="${SITE}/assets/og.svg">
<meta name="twitter:card" content="summary_large_image">
<link rel="icon" href="assets/favicon.svg" type="image/svg+xml">
<link rel="stylesheet" href="theme/style.css">
${options.head ?? ""}
</head>
<body${options.bodyClass ? ` class="${options.bodyClass}"` : ""}>
<a class="skip" href="#content">Skip to content</a>
${header(options.current)}
<main id="content">
${body}
</main>
${footer()}
${options.scripts ?? ""}
</body>
</html>
`;
}

function header(current?: string): string {
  const link = (href: string, label: string, key: string) =>
    `<a href="${href}"${current === key ? ' class="here"' : ""}>${label}</a>`;
  return `<header class="bar">
  <a class="wordmark" href="index.html">
    ${mark()}
    <span>Tailor</span>
  </a>
  <nav>
    ${link("docs.html", "Docs", "docs")}
    ${link("tutorial.html", "Tutorial", "tutorial")}
    <a href="https://github.com/wess/tailor/releases">Download</a>
    <a class="ghost" href="https://github.com/wess/tailor">GitHub</a>
  </nav>
</header>`;
}

function footer(): string {
  return `<footer class="foot">
  <div>
    <strong>Tailor</strong>
    <p>A visual interface builder for <a href="https://github.com/zed-industries/zed">gpui</a>,
       built with <a href="https://github.com/wess/guise">guise</a>. MIT licensed.</p>
  </div>
  <div class="cols">
    <div>
      <h4>Docs</h4>
      <a href="docs.html">Overview</a>
      <a href="tutorial.html">Tutorial</a>
      <a href="running.html">Running</a>
      <a href="codegen.html">Generated code</a>
    </div>
    <div>
      <h4>Project</h4>
      <a href="https://github.com/wess/tailor">Source</a>
      <a href="https://github.com/wess/tailor/releases">Releases</a>
      <a href="https://github.com/wess/tailor/issues">Issues</a>
      <a href="https://github.com/wess/tailor/blob/main/CHANGELOG.md">Changelog</a>
    </div>
  </div>
</footer>`;
}

/** The wordmark: a needle's eye over a seam. Small enough to inline. */
export function mark(size = 20): string {
  return `<svg width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" aria-hidden="true">
  <rect x="2.5" y="2.5" width="19" height="19" rx="5" stroke="currentColor" stroke-width="1.6" opacity=".55"/>
  <path d="M8 16.5 16 7.5" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/>
  <circle cx="8" cy="16.5" r="2.1" fill="currentColor"/>
  <circle cx="16" cy="7.5" r="2.1" stroke="currentColor" stroke-width="1.6"/>
</svg>`;
}

export function escape(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}
