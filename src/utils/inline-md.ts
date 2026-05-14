/// Tiny inline-only Markdown parser for task text.
///
/// Returns an array of `InlineSegment`s that the renderer maps to plain
/// Vue elements (no v-html / no innerHTML) — keeps task text free of XSS
/// even when a .md file is hand-edited with hostile content.
///
/// Supported:
///   `code`            inline code (its content is *not* re-parsed)
///   **bold** / __bold__
///   ~~strike~~
///   [text](url)       link
///   *italic* / _italic_   (skips word-internal underscores like `foo_bar`)
///
/// Not supported (yet): images, raw HTML, autolinks, Obsidian Tasks emoji
/// metadata, nested emphasis. A future v0.3 task will add Obsidian metadata.

export type InlineSegment =
  | { type: 'text'; text: string }
  | { type: 'code'; text: string }
  | { type: 'bold'; text: string }
  | { type: 'italic'; text: string }
  | { type: 'strike'; text: string }
  | { type: 'link'; text: string; href: string };

type TokenKind = Exclude<InlineSegment['type'], 'text'>;

interface PatternRule {
  kind: TokenKind;
  re: RegExp;
  build: (m: RegExpMatchArray) => InlineSegment;
}

const RULES: PatternRule[] = [
  // Code first — its content stays literal even if it contains * or _.
  {
    kind: 'code',
    re: /`([^`\n]+)`/,
    build: m => ({ type: 'code', text: m[1] }),
  },
  // Strong (two-char delimiters) before emphasis so `**x**` isn't read as `*` + `x*`.
  {
    kind: 'bold',
    re: /\*\*([^*\n]+?)\*\*/,
    build: m => ({ type: 'bold', text: m[1] }),
  },
  {
    kind: 'bold',
    re: /__([^_\n]+?)__/,
    build: m => ({ type: 'bold', text: m[1] }),
  },
  {
    kind: 'strike',
    re: /~~([^~\n]+?)~~/,
    build: m => ({ type: 'strike', text: m[1] }),
  },
  {
    kind: 'link',
    re: /\[([^\]\n]+?)\]\(([^)\s\n]+)\)/,
    build: m => ({ type: 'link', text: m[1], href: m[2] }),
  },
  // Single-char emphasis: avoid matching inside identifiers (`foo_bar`).
  {
    kind: 'italic',
    re: /(?<![*\w])\*([^*\n]+?)\*(?!\w)/,
    build: m => ({ type: 'italic', text: m[1] }),
  },
  {
    kind: 'italic',
    re: /(?<![_\w])_([^_\n]+?)_(?!\w)/,
    build: m => ({ type: 'italic', text: m[1] }),
  },
];

interface Hit {
  start: number;
  end: number;
  segment: InlineSegment;
}

function findEarliest(text: string): Hit | null {
  let best: Hit | null = null;
  for (const rule of RULES) {
    const m = rule.re.exec(text);
    if (!m || m.index === undefined) continue;
    if (best && m.index >= best.start) continue;
    best = {
      start: m.index,
      end: m.index + m[0].length,
      segment: rule.build(m),
    };
  }
  return best;
}

export function parseInline(text: string): InlineSegment[] {
  const out: InlineSegment[] = [];
  let rest = text;
  while (rest.length > 0) {
    const hit = findEarliest(rest);
    if (!hit) {
      out.push({ type: 'text', text: rest });
      break;
    }
    if (hit.start > 0) {
      out.push({ type: 'text', text: rest.slice(0, hit.start) });
    }
    out.push(hit.segment);
    rest = rest.slice(hit.end);
  }
  return out;
}
