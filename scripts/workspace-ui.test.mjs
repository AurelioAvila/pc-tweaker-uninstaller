import assert from 'node:assert/strict';
import { mkdir } from 'node:fs/promises';
const { chromium } = await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
const browser = await chromium.launch({ executablePath: process.env.BROWSER_PATH || 'C:/Program Files/Google/Chrome/Application/chrome.exe', headless: true });
const page = await browser.newPage({ viewport: { width: 1120, height: 800 } });
const errors = [];
page.on('pageerror', error => errors.push(error.message));
await page.route('https://**/*', route => route.abort());
await page.addInitScript(() => {
  localStorage.clear();
  const row = (name, size, date, extra = {}) => ({ id:name, name, publisher:'Example publisher', version:'1.2.3', source:'machine64', installDate:date, estimatedSizeKb:size, installLocation:'C:\\Apps\\Example', uninstall:'executable', hidden:false, confidence:{level:'review', reasons:[]}, relations:{dependents:[], installedVia:null, publisherSiblings:0}, ...extra });
  const today = new Date().toISOString().slice(0,10);
  const rows = [row('Creative Studio', 2400000, today), row('Microsoft Visual C++ Redistributable — a deliberately long program name', 42000, '2020-01-01', {confidence:{level:'keep',reasons:[]}}), row('Music player', 320000, today), row('PC Tweaker', 50000, today), row('Unknown size utility', null, null), ...Array.from({length:14}, (_,i)=>row(`Utility ${i+1}`,20000, '2020-01-01'))];
  window.testState = { mode:'offer', checks:0, installs:0, relaunch:0, clears:0, clearError:true };
  let receipts = [{ ts:1780000000, programName:'Example removal', method:'executable', success:true, rebootRequired:false, verifiedFreedKb:1000, estimatedSizeKb:1000, restorePoint:'created', message:'Removed successfully.' }];
  window.__TAURI_INTERNALS__ = {
    transformCallback: callback => { const id=Math.floor(Math.random()*1e9); window[`_${id}`]=callback; return id; },
    unregisterCallback: id => { delete window[`_${id}`]; },
    invoke: async (command, args) => {
      if(command==='list_programs') { await new Promise(r=>setTimeout(r,120)); return rows; }
      if(command==='list_store_apps') return [];
      if(command==='list_removal_ledger') return receipts;
      if(command==='clear_removal_ledger') { window.testState.clears++; if(window.testState.clearError) throw 'Test write failure'; receipts=[]; return; }
      if(command==='program_icon') return null;
      if(command==='app_version') return '0.12.0';
      if(command==='plugin:updater|check') { window.testState.checks++; await new Promise(r=>setTimeout(r,100)); if(window.testState.mode==='error') throw Error('offline'); return window.testState.mode==='current' ? null : {rid:1,currentVersion:'0.12.0',version:'0.12.1',body:'Test release',rawJson:{}}; }
      if(command==='plugin:updater|download_and_install') {
        window.testState.installs++;
        const send = (index,message) => window[`_${args.onEvent.id}`]({index,message});
        send(0,{event:'Started',data: window.testState.mode === 'success' ? {contentLength:100} : {}});
        await new Promise(r=>setTimeout(r,250));
        send(1,{event:'Progress',data:{chunkLength:50}});
        await new Promise(r=>setTimeout(r,250));
        if(window.testState.mode === 'success') { send(2,{event:'Finished'}); return; }
        throw Error('Test download failure');
      }
      if(command==='plugin:process|restart') { window.testState.relaunch++; return; }
      if(command==='plugin:resources|close') return;
      throw Error(`Unexpected command: ${command}`);
    }
  };
});
try {
  await page.goto(process.env.UI_URL || 'http://127.0.0.1:1421');
  await page.locator('.row').first().waitFor();
  await page.locator('.update-card').waitFor();
  assert.equal(await page.evaluate(()=>window.testState.checks),1,'StrictMode must not duplicate update checks');
  await page.locator('.update-card .button-ghost').click();
  await mkdir('ui-evidence', {recursive:true});
  for (const [width,height] of [[1120,800],[940,560],[1920,1000]]) {
    await page.setViewportSize({width,height});
    await page.screenshot({path:`ui-evidence/inventory-${width}.png`});
    assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth > innerWidth),false,'No horizontal page overflow');
    const overlaps=await page.locator('.row').first().evaluate(row=>Array.from(row.children).some((cell,i,cells)=>i>0 && cells[i-1].getBoundingClientRect().right > cell.getBoundingClientRect().left+1));
    assert.equal(overlaps,false,'Columns must not overlap');
  }
  await page.setViewportSize({width:1120,height:800});
  await page.locator('.row-detail-hint').first().click();
  assert.equal(await page.locator('.row').first().getAttribute('aria-expanded'),'true','Details button expands once');
  await page.getByRole('button',{name:'History',exact:true}).click();
  await page.locator('.ledger-clear').click();
  await page.getByRole('button',{name:'Cancel',exact:true}).click();
  assert.equal(await page.evaluate(()=>window.testState.clears),0,'Cancel never deletes history');
  await page.locator('.ledger-clear').click();
  await page.locator('.ledger-delete').click();
  await page.getByRole('alert').filter({hasText:'Test write failure'}).waitFor();
  assert.equal(await page.locator('.ledger-row').count(),1,'Failed clear retains receipts');
  await page.screenshot({path:'ui-evidence/history-confirmation.png'});
  await page.evaluate(()=>{window.testState.clearError=false;});
  await page.locator('.ledger-delete').click();
  await page.locator('.ledger-row').waitFor({state:'detached'});
  await page.getByRole('button',{name:'Close',exact:true}).click();
  assert.equal(await page.evaluate(()=>window.testState.clears),2,'Clear only runs after confirmation');
  await page.locator('.inventory-summary button').nth(1).click();
  assert.equal(await page.locator('.row').count(),1,'Large apps shortcut');
  await page.locator('.inventory-summary button').first().click();
  await page.locator('.toolbar input').fill('Unknown size');
  assert.equal(await page.locator('.row').count(),1,'Search works');
  await page.locator('.row').first().press('Enter');
  assert.equal(await page.locator('.row').first().getAttribute('aria-expanded'),'true');
  await page.locator('.toolbar input').fill('');
  await page.locator('.advanced-controls > summary').click();
  await page.screenshot({path:'ui-evidence/filters.png'});
  await page.locator('.advanced-controls > summary').click();
  await page.locator('.menu-trigger').click();
  assert.equal(await page.locator('.menu-section[open]').count(),0,'Preferences are collapsed');
  await page.screenshot({path:'ui-evidence/profile.png'});
  await page.locator('.menu-section > summary').nth(2).click();
  await page.getByRole('button',{name:'Italiano',exact:true}).click();
  assert.equal(await page.locator('.workspace-heading h2').textContent(),'App installate');
  await page.getByRole('button',{name:'Português',exact:true}).click();
  assert.equal(await page.locator('.workspace-heading h2').textContent(),'Aplicações instaladas');
  await page.getByRole('button',{name:'English',exact:true}).click();
  await page.locator('.menu-section > summary').nth(3).click();
  const themeButtons=page.locator('.theme-option');
  assert.equal(await themeButtons.count(),8);
  const accents=new Set();
  for(let i=0;i<8;i++) { await themeButtons.nth(i).click(); assert.equal(await themeButtons.nth(i).getAttribute('aria-pressed'),'true'); accents.add(await page.evaluate(()=>getComputedStyle(document.documentElement).getPropertyValue('--accent'))); }
  assert.equal(accents.size,8,'Every theme applies a distinct accent');
  await themeButtons.first().click();
  await page.keyboard.press('Escape');
  assert.equal(await page.locator('.menu-trigger').evaluate(el=>el===document.activeElement),true,'Escape restores focus');
  await page.locator('.menu-trigger').click();
  await page.getByRole('button',{name:'Check for updates',exact:true}).click();
  await page.locator('.update-card').waitFor();
  await page.keyboard.press('Escape');
  await page.screenshot({path:'ui-evidence/update.png'});
  await page.locator('.update-card .primary').click();
  await page.getByText('Downloading update…',{exact:true}).waitFor();
  await page.locator('.update-card [role=alert]').waitFor();
  assert.equal(await page.evaluate(()=>window.testState.installs),1);
  assert.equal(await page.evaluate(()=>window.testState.relaunch),0,'Failed download must not restart');
  await page.locator('.update-card .button-ghost').click();
  await page.evaluate(()=>{window.testState.mode='current';});
  await page.locator('.menu-trigger').click();
  await page.getByRole('button',{name:'Check for updates',exact:true}).click();
  await page.getByText("You're up to date",{exact:true}).waitFor();
  await page.evaluate(()=>{window.testState.mode='error';});
  await page.getByRole('button',{name:'Check for updates',exact:true}).click();
  await page.getByText("Couldn't check. Try again when you're online.",{exact:true}).waitFor();
  await page.evaluate(()=>{window.testState.mode='success';});
  await page.getByRole('button',{name:'Check for updates',exact:true}).click();
  await page.locator('.update-card').waitFor();
  await page.keyboard.press('Escape');
  await page.locator('.update-card .primary').evaluate(button=>{button.click();button.click();});
  await page.waitForFunction(()=>document.querySelector('.update-progress progress')?.value === 50);
  assert.equal(await page.locator('main').evaluate(el=>el.inert),true,'Inventory is inert during installation');
  await page.waitForFunction(()=>window.testState.relaunch===1);
  assert.equal(await page.evaluate(()=>window.testState.installs),2,'Duplicate clicks must not start another install');
  assert.deepEqual(errors,[]);
  console.log('PASS: inventory, responsive columns, search, filters, profile/locales/keyboard, updater offer/dismiss/manual/progress/failure/current/offline. No real apps or updates modified.');
} finally { await browser.close(); }
