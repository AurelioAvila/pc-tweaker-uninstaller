/**
 * Fails if text on the accent gradient is unreadable on any theme.
 *
 * The primary button shipped white text on every one of the eight themes, and
 * white fails WCAG AA against all sixteen accent stops: 3.46:1 on violet at
 * the very best, 1.48:1 on Slate Mono. Nobody noticed because there are eight
 * palettes and a person checks the one they use.
 *
 * A theme is added by pasting a pair of hex values into themes.ts, which is
 * exactly the moment this breaks again, so the check reads the themes from
 * source rather than taking a list on trust.
 */
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), '..');
const AA_NORMAL_TEXT = 4.5;

const channel = (value) => {
  const c = value / 255;
  return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
};

const luminance = (hex) => {
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16));
  return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
};

const contrast = (a, b) => {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
};

const css = readFileSync(path.join(root, 'src', 'App.css'), 'utf8');
const ink = css.match(/--accent-ink:\s*(#[0-9a-fA-F]{6})/)?.[1];
if (!ink) {
  console.error('--accent-ink is not defined in src/App.css');
  process.exit(1);
}

const themes = [
  ...readFileSync(path.join(root, 'src', 'themes.ts'), 'utf8').matchAll(
    /code:\s*"([^"]+)"[\s\S]*?accent:\s*"(#[0-9a-fA-F]{6})"[\s\S]*?accent2:\s*"(#[0-9a-fA-F]{6})"/g,
  ),
].map(([, code, accent, accent2]) => ({ code, accent, accent2 }));

if (themes.length === 0) {
  console.error('no themes found in src/themes.ts; the pattern this check relies on has changed');
  process.exit(1);
}

const failures = [];
for (const { code, accent, accent2 } of themes) {
  for (const [label, stop] of [['accent', accent], ['accent2', accent2]]) {
    const ratio = contrast(ink, stop);
    if (ratio < AA_NORMAL_TEXT) {
      failures.push(`${code} ${label} ${stop}: ${ratio.toFixed(2)}:1 against ${ink}`);
    }
  }
}

if (failures.length > 0) {
  console.error(`Text on the accent gradient fails WCAG AA (${AA_NORMAL_TEXT}:1):`);
  for (const failure of failures) console.error(`  ${failure}`);
  process.exit(1);
}

console.log(`Accent ink ${ink} clears WCAG AA on all ${themes.length} themes.`);
