import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import ts from 'typescript';
// English is the default for everyone who never chose a language: no reading of
// the Windows or browser locale. A saved choice is kept. (Owner decision, 2026-10-11.)
const code=ts.transpileModule(fs.readFileSync(new URL('../src/i18n.ts',import.meta.url),'utf8'),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ESNext}}).outputText;
const url='data:text/javascript;base64,'+Buffer.from(code).toString('base64');
let n=0;
const load=async(stored)=>{globalThis.localStorage={getItem:()=>{if(stored instanceof Error)throw stored;return stored;},setItem:()=>{}};Object.defineProperty(globalThis,'navigator',{value:{language:'pt-BR',languages:['pt-BR','it-IT']},configurable:true});return import(url+'#'+(n++));};
test('first start is English even when the system language is not',async()=>{assert.equal((await load(null)).currentLocale(),'en');});
test('a saved choice is kept',async()=>{for(const l of ['it','fr','es','de','pt']) assert.equal((await load(l)).currentLocale(),l);});
test('unknown, prototype or unreadable values fall back to English',async()=>{for(const v of ['pt-BR','toString','__proto__','',new Error('blocked')]) assert.equal((await load(v)).currentLocale(),'en');});
test('Portuguese is European Portuguese',async()=>{const m=await load('pt');assert.equal(m.LOCALES.find(l=>l.code==='pt').native,'Português');const pt=JSON.stringify(m.text).toLowerCase();for(const w of ['arquivo','usuário','baixar','senha','tela ','celular','gerenciar','lixeira']) assert.ok(!pt.includes(w),`Brazilian form found: ${w}`);assert.ok(pt.includes('palavra-passe'));});
