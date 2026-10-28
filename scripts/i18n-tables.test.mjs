import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import ts from 'typescript';
// Every table that holds user-facing text, in all six languages: same shape as
// English, and the house rules (plan names, pt-PT, French spacing, German
// dashes and "Sie", Italian accents, no "illustrative") hold everywhere.
globalThis.localStorage = { getItem: () => null, setItem: () => {} };
const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'pcu-i18n-'));
for (const name of ['i18n', 'plans-copy', 'trial-copy', 'workspace-copy', 'inventory-copy', 'backend-copy']) {
  const source = fs.readFileSync(new URL(`../src/${name}.ts`, import.meta.url), 'utf8');
  let js = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext } }).outputText;
  js = js.replace(/from "\.\/([\w-]+)"/g, 'from "./$1.mjs"');
  fs.writeFileSync(path.join(dir, `${name}.mjs`), js);
}
const load = (name) => import(pathToFileURL(path.join(dir, `${name}.mjs`)).href);
const i18n = await load('i18n');
const tables = {
  dictionary: i18n.dictionaries,
  plans: (await load('plans-copy')).plansCopy,
  trial: (await load('trial-copy')).trialCopy,
  workspace: (await load('workspace-copy')).workspaceCopy,
  inventory: (await load('inventory-copy')).inventoryCopy,
};
const LANGS = ['en', 'it', 'fr', 'es', 'de', 'pt'];

const shape = (value) => {
  if (typeof value === 'function') return 'fn';
  if (Array.isArray(value)) return value.map(shape);
  if (value && typeof value === 'object') return Object.fromEntries(Object.keys(value).sort().map((k) => [k, shape(value[k])]));
  return 'str';
};
const strings = (value, out = []) => {
  if (typeof value === 'string') out.push(value);
  else if (typeof value === 'function') out.push(value.toString().replace(/\$\{[^}]*\}/g, ''));
  else if (value && typeof value === 'object') Object.values(value).forEach((v) => strings(v, out));
  return out;
};
const placeholders = (s) => (s.match(/\{\w+\}/g) ?? []).sort().join();

for (const [name, table] of Object.entries(tables)) {
  test(`${name}: every language matches the English shape`, () => {
    assert.deepEqual(Object.keys(table).sort(), [...LANGS].sort());
    for (const l of LANGS) assert.deepEqual(shape(table[l]), shape(table.en), `${name}.${l}`);
  });
  test(`${name}: house rules`, () => {
    for (const l of LANGS) {
      for (const s of strings(table[l])) {
        assert.ok(!/illustrat/i.test(s), `${name}.${l}: banned word in "${s}"`);
        if (l === 'fr') assert.ok(!/[^\s] [:;!?](?=[\s"'`.)]|$)/.test(s), `${name}.fr: needs a no-break space: "${s}"`);
        if (l === 'de') assert.ok(!/ - |—/.test(s), `${name}.de: use an en dash: "${s}"`);
        if (l === 'it') assert.ok(!/\b(?:E'|piu'|perche'|puo|piu|gia|cosi|pero)\b/.test(s), `${name}.it: accent missing: "${s}"`);
        if (l === 'pt') assert.ok(!/\bbrowser\b/i.test(s), `${name}.pt: say "navegador": "${s}"`);
      }
    }
  });
}
test('trial and plans keep the same placeholders as English', () => {
  for (const key of ['trial', 'plans']) {
    const en = tables[key].en;
    for (const l of LANGS) {
      for (const k of Object.keys(en)) {
        if (typeof en[k] === 'string') assert.equal(placeholders(tables[key][l][k]), placeholders(en[k]), `${key}.${l}.${k}`);
      }
    }
  }
});
test('German never uses the informal "du" in any table', () => {
  const words = /(?:du|dich|dir|dein(?:e[mnrs]?)?|finde|prüfe|beende|versuche)/i;
  for (const [name, table] of Object.entries(tables)) {
    for (const s of strings(table.de)) assert.ok(!words.test(s), `${name}.de: "${s}"`);
  }
});
test('plan names stay in English everywhere', () => {
  for (const l of LANGS) assert.equal(tables.dictionary[l].menu.planLifetime, 'Lifetime');
  for (const l of LANGS) {
    assert.equal(tables.plans[l].freeName, 'Free');
    assert.equal(tables.plans[l].proName, 'Pro');
  }
});
test('backend messages are translated and fall back to the original', async () => {
  const { backendText } = await load('backend-copy');
  const ok = 'Uninstalled successfully.';
  const seen = new Set();
  for (const l of LANGS.slice(1)) {
    i18n.setLocale(l);
    const out = backendText(ok);
    assert.notEqual(out, ok, `${l} not translated`);
    seen.add(out);
    assert.equal(backendText('Something unknown.'), 'Something unknown.');
    assert.match(backendText('The uninstaller exited with code 5. The program may not be fully removed — refresh the list to check.'), /5/);
  }
  assert.equal(seen.size, 5);
  i18n.setLocale('en');
  assert.equal(backendText(ok), ok);
});
