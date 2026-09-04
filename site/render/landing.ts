// The landing page.
//
// The hero is not a screenshot. Tailor's whole claim is the seam between a
// design and the Rust it produces, so the picture is that seam: a small mock
// canvas on the left and the code it generates on the right, both drawn as
// markup. It stays sharp at any density, weighs nothing, and it is the one
// image that cannot be out of date — it is written from the same file the
// generator would print.

import { shell, mark } from "./shell";
import { GROUPS } from "./nav";
import { highlight } from "./doc";

const GENERATED = `impl Render for SignInScreen {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(px(16.))
            .child(Title::new("Welcome back").order(2))
            .child(TextInput::bind(&self.email, cx))
            .child(
                Button::new("submit", "Sign in")
                    .variant(Variant::Filled)
                    .full_width(true)
                    .on_click(cx.listener(|this, _window, cx| this.submit(cx))),
            )
    }
}

impl SignInScreen {
    /// Sign the user in.
    pub fn submit(&mut self, cx: &mut Context<Self>) {
        let who = crate::accounts::current();
        self.email.set(cx, who);
        cx.notify();
    }
}`;

function icon(path: string): string {
  return `<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor"
  stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${path}</svg>`;
}

const ICONS = {
  play: `<polygon points="6 3 20 12 6 21 6 3"/>`,
  code: `<polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/>`,
  bug: `<path d="m8 2 1.88 1.88M14.12 3.88 16 2"/><path d="M9 7.13v-1a3.003 3.003 0 1 1 6 0v1"/><path d="M12 20c-3.3 0-6-2.7-6-6v-3a4 4 0 0 1 4-4h4a4 4 0 0 1 4 4v3c0 3.3-2.7 6-6 6"/><path d="M12 20v-9"/>`,
  layers: `<path d="m12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83Z"/><path d="m22 17.65-9.17 4.16a2 2 0 0 1-1.66 0L2 17.65"/><path d="m22 12.65-9.17 4.16a2 2 0 0 1-1.66 0L2 12.65"/>`,
  bot: `<path d="M12 8V4H8"/><rect width="16" height="12" x="4" y="8" rx="2"/><path d="M2 14h2M20 14h2M15 13v2M9 13v2"/>`,
  boxes: `<path d="M2.97 12.92A2 2 0 0 0 2 14.63v3.24a2 2 0 0 0 .97 1.71l3 1.8a2 2 0 0 0 2.06 0L12 19v-5.5l-5-3-4.03 2.42Z"/><path d="m7 16.5-4.74-2.85M7 16.5l5-3M7 16.5v5.17"/><path d="M12 13.5V19l3.97 2.38a2 2 0 0 0 2.06 0l3-1.8a2 2 0 0 0 .97-1.71v-3.24a2 2 0 0 0-.97-1.71L17 10.5l-5 3Z"/>`,
};

export function renderLanding(): string {
  const body = `
<section class="hero">
  <h1>Design it. <em>Run it.</em> Take the Rust.</h1>
  <p class="lede">
    Tailor is a visual interface builder for
    <a href="https://github.com/zed-industries/zed">gpui</a> — Interface Builder
    for Rust. Lay out a screen from real components, write the code behind the
    controls, press Run, and leave with a crate that has no dependency on Tailor
    in it.
  </p>
  <div class="cta">
    <a class="btn primary" href="https://github.com/wess/tailor/releases/latest">
      ${icon(ICONS.play)} Download for macOS
    </a>
    <a class="btn quiet" href="tutorial.html">Read the tutorial</a>
    <code>brew install --cask wess/packages/tailor</code>
  </div>
</section>

<section class="seam" aria-label="A design and the code it generates">
  <div class="seam-frame">
    <div>
      <div class="seam-head"><span>Canvas</span><span>SignInScreen</span></div>
      <div class="seam-body">
        <div class="mock">
          <div class="title"></div>
          <div class="line" style="width:78%"></div>
          <div class="row"><div class="field"></div></div>
          <div class="row"><div class="field"></div></div>
          <div class="row"><div class="button selected"></div></div>
        </div>
      </div>
    </div>
    <div class="seam-right">
      <div class="seam-head"><span>sign_in_screen.rs</span><span>plain</span></div>
      <div class="seam-body"><pre><code class="hljs">${highlight(
        GENERATED,
        "rust",
      )}</code></pre></div>
    </div>
  </div>
</section>

<section class="band">
  <h2>The loop Interface Builder never closed</h2>
  <p class="sub">
    Drawing a screen is the easy half. The half that matters is what happens
    when it does not compile.
  </p>
  <ol class="loop">
    <li>
      <h3>Design</h3>
      <p>Drag real components onto a canvas. A <code>Button</code> on it is a
      real one, reading the same theme — there is no second rendering path to
      keep in step.</p>
    </li>
    <li>
      <h3>Write the code behind it</h3>
      <p>Wire a click to an action, then write the method, with completion over
      what is actually in scope. Your code lives in the project file, so
      regenerating writes <em>around</em> it.</p>
    </li>
    <li>
      <h3>Run</h3>
      <p>Press <kbd>⌘R</kbd>. Tailor writes the crate, cargo builds it, your app
      opens. A compiler error comes back as a row you can click — and it selects
      the component that caused it.</p>
    </li>
  </ol>
</section>

<section class="band">
  <h2>What is in it</h2>
  <p class="sub">
    A builder, a compiler, and an editor that knows what the other two are
    doing.
  </p>
  <div class="grid">
    <article>
      <h3>${icon(ICONS.play)} Run, Build, Stop</h3>
      <p>No setup: a project you have never exported gets its own build
      directory. Debug or release. The console tags the compiler's output apart
      from your app's, and the app runs with backtraces on.</p>
    </article>
    <article>
      <h3>${icon(ICONS.bug)} Errors point at components</h3>
      <p>Codegen tags every node with the line it generated. A type error at
      <code>main_screen.rs:41</code> resolves back to the button that produced
      it, in the gutter and in Problems.</p>
    </article>
    <article>
      <h3>${icon(ICONS.code)} A real editor</h3>
      <p>Every file a build compiles, parsed by tree-sitter, with line numbers,
      inline diagnostics and <kbd>⌘F</kbd>. <kbd>⌥⌘O</kbd> hands the file to
      Zed, VS Code, or whatever you use.</p>
    </article>
    <article>
      <h3>${icon(ICONS.layers)} Your code, kept</h3>
      <p>Action bodies live in the <code>.tailor</code> file. Modules you add
      are declared by <code>main.rs</code> and written once — an export reports
      them as <em>kept</em>, never replaced.</p>
    </article>
    <article>
      <h3>${icon(ICONS.boxes)} A component library is a plug-in</h3>
      <p>Tailor draws with <a href="https://github.com/wess/guise">guise</a> and
      <em>targets</em> it through three traits. Another library is two crates
      and a line in <code>main</code>.</p>
    </article>
    <article>
      <h3>${icon(ICONS.bot)} Drive it from an agent</h3>
      <p>An MCP server over the same document model. It saves after every
      change and the app watches the file, so a screen built by an agent
      appears on the canvas as it is built.</p>
    </article>
  </div>
</section>

<section class="band">
  <h2>What it is not</h2>
  <div class="note">
    <p><strong>Not a runtime.</strong> The <code>.tailor</code> file is a design
    document, not something your app loads. What ships is a Rust file you own,
    and the ending Tailor is built for is the one where you take that file and
    stop opening the builder.</p>
    <p style="margin-bottom:0"><strong>Not cross-platform, yet.</strong> gpui
    targets the desktop, so Tailor does too — one artboard size, which is the
    window your app opens at. A wasm backend is the point at which a second one
    would mean something.</p>
  </div>
</section>

<section class="band">
  <h2>Documentation</h2>
  <p class="sub">Every page is written for someone using Tailor, not for someone selling it.</p>
  <div class="index-grid">
    ${GROUPS.flatMap((group) => group.pages)
      .map(
        (page) =>
          `<a href="${page.out}"><strong>${page.title}</strong><span>${page.blurb}</span></a>`,
      )
      .join("\n    ")}
  </div>
</section>
`;
  return shell({ title: "Tailor", bodyClass: "landing" }, body);
}

/** The docs home: the same list, grouped, with the sidebar beside it. */
export function renderDocsIndex(sidebar: string): string {
  const body = `<div class="docs">
${sidebar}
<article class="prose">
  <h1>Tailor documentation</h1>
  <p>Tailor is a visual interface builder for gpui. You lay out a screen by
  dragging real components onto a canvas, write the code behind the controls,
  press Run, and export idiomatic Rust that has no dependency on Tailor left
  in it.</p>
  <p><strong>New here?</strong> The <a href="tutorial.html">tutorial</a> builds
  a complete app from an empty project to a running binary, and every code block
  in it is output Tailor actually produced.</p>
  ${GROUPS.map(
    (group) => `<h2>${group.name}</h2>
  <div class="index-grid">
    ${group.pages
      .map(
        (page) =>
          `<a href="${page.out}"><strong>${page.title}</strong><span>${page.blurb}</span></a>`,
      )
      .join("\n    ")}
  </div>`,
  ).join("\n  ")}
  <h2>Getting it</h2>
  <p>Every <a href="https://github.com/wess/tailor/releases">release</a> attaches
  <strong><code>Tailor.dmg</code></strong>, signed and notarized, with the MCP
  server beside the executable in the bundle. Or:</p>
  <pre><code class="hljs">${highlight(
    "brew install --cask wess/packages/tailor",
    "bash",
  )}</code></pre>
  <p>From a checkout, <code>cargo run -p tailor-app</code> — the binary is
  <code>tailordev</code> so a development build never collides with an
  installed <code>tailor</code>.</p>
</article>
</div>`;
  return shell(
    { title: "Documentation", current: "docs", bodyClass: "doc" },
    body,
  );
}

export { mark };
