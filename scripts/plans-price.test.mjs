import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import ts from 'typescript';
const code=ts.transpileModule(fs.readFileSync(new URL('../src/plans-price.ts',import.meta.url),'utf8'),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ESNext}}).outputText;
const {proPrice,money,REGULAR_CENTS,LOYALTY_CENTS}=await import('data:text/javascript;base64,'+Buffer.from(code).toString('base64'));
const offer={regular:999,reference:999,price:499};
test('the Halloween offer is drawn only for people paying the standard price',()=>{assert.deepEqual(proPrice(false,false,offer),{shown:offer,price:499});assert.deepEqual(proPrice(true,false,offer),{shown:null,price:LOYALTY_CENTS});assert.deepEqual(proPrice(false,true,offer),{shown:null,price:999});});
test('no offer from the server means the regular price, never a struck one',()=>{assert.deepEqual(proPrice(false,false,null),{shown:null,price:REGULAR_CENTS});assert.deepEqual(proPrice(true,false,null),{shown:null,price:LOYALTY_CENTS});assert.equal(proPrice(false,true,{regular:1299,reference:999,price:499}).price,1299);});
test('euro amounts read in the UI language',()=>{assert.equal(money(999,'en'),'€9.99');assert.equal(money(0,'en'),'€0');assert.equal(money(499,'it').replace(/\s/g,' '),'4,99 €');assert.equal(money(499,'pt-PT').replace(/\s/g,' '),'4,99 €');assert.equal(money(999,'de').replace(/\s/g,' '),'9,99 €');});
test('plan badges: the account plan reads Current plan, every plan below it Included in your plan',async()=>{const {planBadges}=await import('data:text/javascript;base64,'+Buffer.from(code).toString('base64'));
  // free; trial, annual and the PC Tweaker Lifetime year all make Pro active
  assert.deepEqual(planBadges(false),{free:'current',pro:null});
  for(const state of ['trialing','annual','lifetime_bonus'])assert.deepEqual(planBadges(true),{free:'included',pro:'current'},state);});
