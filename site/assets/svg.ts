// The favicon and the social card, written rather than drawn: two small SVGs
// that stay sharp, weigh nothing, and cannot fall out of step with the
// wordmark in `shell.ts` because they are the same shape — a stitch, which is
// the one thing a tailor's tool is unambiguously about.

const BG = "#0c0d10";
const ACCENT = "#4c8dff";
const TEXT = "#e7e9ee";
const DIM = "#8b93a1";

export function favicon(): string {
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
  <rect width="24" height="24" rx="5.5" fill="${BG}"/>
  <path d="M7.5 16.8 16.5 7.2" stroke="${ACCENT}" stroke-width="1.9" stroke-linecap="round"/>
  <circle cx="7.5" cy="16.8" r="2.3" fill="${ACCENT}"/>
  <circle cx="16.5" cy="7.2" r="2.3" fill="none" stroke="${ACCENT}" stroke-width="1.9"/>
</svg>
`;
}

export function ogImage(): string {
  const sans = "-apple-system, BlinkMacSystemFont, system-ui, sans-serif";
  return `<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630" viewBox="0 0 1200 630">
  <rect width="1200" height="630" fill="${BG}"/>
  <rect x="0" y="0" width="1200" height="4" fill="${ACCENT}"/>
  <g transform="translate(96 132)">
    <g transform="scale(2.4)">
      <path d="M7.5 16.8 16.5 7.2" stroke="${ACCENT}" stroke-width="1.9" stroke-linecap="round"/>
      <circle cx="7.5" cy="16.8" r="2.3" fill="${ACCENT}"/>
      <circle cx="16.5" cy="7.2" r="2.3" fill="none" stroke="${ACCENT}" stroke-width="1.9"/>
    </g>
    <text x="76" y="42" fill="${TEXT}" font-family="${sans}" font-size="46" font-weight="600"
      letter-spacing="-1.2">Tailor</text>
    <text x="0" y="160" fill="${TEXT}" font-family="${sans}" font-size="74" font-weight="600"
      letter-spacing="-2.8">Design it. Run it.</text>
    <text x="0" y="248" fill="${ACCENT}" font-family="${sans}" font-size="74" font-weight="600"
      letter-spacing="-2.8">Take the Rust.</text>
    <text x="0" y="318" fill="${DIM}" font-family="${sans}" font-size="28">
      A visual interface builder for gpui
    </text>
  </g>
</svg>
`;
}
