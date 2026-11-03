import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import ts from 'typescript';
// Grouping and default selection of the leftovers dialog: only items proven to
// come from the removed program start selected, Keep items can never be ticked,
// and the free/Pro split matches the one Rust enforces.
const code = ts.transpileModule(fs.readFileSync(new URL('../src/leftovers.ts', import.meta.url), 'utf8'), {
  compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext },
}).outputText;
const L = await import('data:text/javascript;base64,' + Buffer.from(code).toString('base64'));

const item = (id, kind, extra = {}) => ({
  id, kind, scope: 'user', path: `C:/x/${id}`, label: id, sizeKb: 100, deletable: true,
  tier: 'review', evidence: 'name', blocker: null, command: null, missing: null, restore: 'recycle', ...extra,
});
const report = (mode, items) => ({ token: 't', mode, items, totalKb: 0 });

test('only Safe items are pre-selected, and only for a scan that read the registry', () => {
  const items = [
    item('dir', 'install-dir', { tier: 'safe', evidence: 'path' }),
    item('data', 'app-data'),
    item('key', 'registry-user', { restore: 'backup' }),
    item('kept', 'install-dir', { tier: 'keep', deletable: false, blocker: 'otherProgram' }),
  ];
  assert.deepEqual(L.defaultSelection(report('registry', items), true), ['dir']);
  assert.deepEqual(L.defaultSelection(report('registry', items), false), ['dir']);
  // Manual and startup-sweep scans start empty, even when an item says Safe.
  assert.deepEqual(L.defaultSelection(report('manual', items), true), []);
  assert.deepEqual(L.defaultSelection(report('sweep', items), true), []);
  // A name match is never pre-selected, whatever else is true of it.
  const named = [item('n', 'app-data', { evidence: 'name', tier: 'review' })];
  assert.deepEqual(L.defaultSelection(report('registry', named), true), []);
});

test('a Safe item that needs Pro is not pre-selected without Pro', () => {
  const items = [item('run', 'startup-run', { tier: 'safe', evidence: 'path' })];
  assert.deepEqual(L.defaultSelection(report('registry', items), false), []);
  assert.deepEqual(L.defaultSelection(report('registry', items), true), ['run']);
});

test('Keep items and report-only items can never be ticked', () => {
  for (const pro of [true, false]) {
    assert.equal(L.isSelectable(item('k', 'app-data', { tier: 'keep', deletable: false }), pro), false);
    assert.equal(L.isSelectable(item('k2', 'app-data', { tier: 'keep', deletable: true }), pro), false);
    assert.equal(L.isSelectable(item('hklm', 'registry-machine', { deletable: false }), pro), false);
  }
});

test('Free ticks files and folders only; Pro ticks everything cleanable', () => {
  for (const kind of ['install-dir', 'app-data', 'shortcut']) {
    assert.equal(L.isProOnly(kind), false, kind);
    assert.equal(L.isSelectable(item('a', kind), false), true, kind);
  }
  for (const kind of ['registry-user', 'registry-machine', 'startup-run', 'startup-shortcut']) {
    assert.equal(L.isProOnly(kind), true, kind);
    assert.equal(L.isSelectable(item('a', kind), false), false, kind);
    assert.equal(L.isSelectable(item('a', kind), true), true, kind);
  }
});

test('the free kinds are the same ones Rust leaves ungated', () => {
  const rust = fs.readFileSync(new URL('../src-tauri/src/residue_rules.rs', import.meta.url), 'utf8');
  const gate = rust.match(/pub fn pro_only\(self\) -> bool \{\s*!matches!\(self, ([^)]*)\)/);
  assert.ok(gate, 'pro_only not found');
  const free = gate[1].split('|').map((k) => k.trim().replace('Kind::', ''));
  assert.deepEqual(free.sort(), ['AppData', 'InstallDir', 'Shortcut']);
});

test('groups come in a fixed order, Keep items only in "kept"', () => {
  const items = [
    item('k', 'startup-run', { tier: 'keep', deletable: false }),
    item('reg', 'registry-user'),
    item('run', 'startup-run'),
    item('lnk', 'startup-shortcut'),
    item('dir', 'install-dir', { tier: 'safe' }),
    item('hklm', 'registry-machine', { deletable: false }),
  ];
  const groups = L.groupItems(items);
  assert.deepEqual(groups.map((g) => g.id), ['files', 'startup', 'registry', 'kept']);
  assert.deepEqual(groups.map((g) => g.items.map((i) => i.id)), [['dir'], ['run', 'lnk'], ['reg', 'hklm'], ['k']]);
  assert.deepEqual(L.groupItems([]), []);
  assert.deepEqual(L.groupItems([item('d', 'app-data')]).map((g) => g.id), ['files']);
});

test('"select all to review" takes the Review items of a group that can be ticked', () => {
  const items = [
    item('a', 'app-data'),
    item('b', 'shortcut'),
    item('c', 'install-dir', { tier: 'safe' }),
    item('d', 'registry-user'),
    item('e', 'app-data', { tier: 'keep', deletable: false }),
  ];
  assert.deepEqual(L.reviewIds(items, false), ['a', 'b']);
  assert.deepEqual(L.reviewIds(items, true), ['a', 'b', 'd']);
});

test('the summary counts what can be cleaned and what still starts with Windows', () => {
  const items = [
    item('a', 'app-data', { sizeKb: 1024 }),
    item('b', 'startup-run', { sizeKb: null }),
    item('c', 'startup-shortcut', { sizeKb: 1 }),
    item('k', 'app-data', { tier: 'keep', sizeKb: 99999 }),
  ];
  assert.deepEqual(L.summarize(items), { count: 3, sizeKb: 1025, startup: 2 });
  assert.deepEqual(L.summarize([]), { count: 0, sizeKb: 0, startup: 0 });
});

test('an undo is summed up as put back, left as it is, or not put back', () => {
  const report = {
    refused: null,
    outcomes: ['restored', 'restored', 'conflict', 'missing', 'refused', 'failed'].map((outcome, i) => ({
      id: String(i),
      outcome,
    })),
  };
  assert.deepEqual(L.undoSummary(report), { restored: 2, kept: 1, failed: 3 });
  assert.deepEqual(L.undoSummary({ refused: 'x', outcomes: [] }), { restored: 0, kept: 0, failed: 0 });
});

test('items of all users can be ticked like any other: folders free, keys with Pro', () => {
  const machine = { scope: 'machine' };
  assert.equal(L.isSelectable(item('pd', 'app-data', machine), false), true);
  assert.equal(L.isSelectable(item('hklm', 'registry-machine', { ...machine, restore: 'backup' }), false), false);
  assert.equal(L.isSelectable(item('hklm', 'registry-machine', { ...machine, restore: 'backup' }), true), true);
});
