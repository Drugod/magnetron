import { chromium } from 'playwright';
import axe from 'axe-core';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import fs from 'node:fs';
const server=spawn(process.execPath,['node_modules/vite/bin/vite.js','--host','127.0.0.1','--port','1420','--strictPort'],{stdio:'ignore',detached:process.platform!=='win32'});
let browser;
const results=[];
try {
  let ready=false;
  for(let n=0;n<100;n++) { try { if((await fetch('http://127.0.0.1:1420')).ok){ready=true;break;} }catch{} await new Promise(r=>setTimeout(r,100)); }
  assert(ready,'Preview server failed to start');
  browser=await chromium.launch({headless:true});
  const page=await browser.newPage({viewport:{width:1250,height:850}});
  await page.emulateMedia({reducedMotion:'reduce'});
  const errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.goto('http://127.0.0.1:1420');
  await page.addScriptTag({content:axe.source});
  fs.mkdirSync('artifacts/accessibility',{recursive:true});
  for(const language of ['es','en']) for(const theme of ['light','dark']) {
    await page.selectOption('#language-select',language);
    await page.selectOption('#appearance-select',theme);
    assert.equal(await page.getAttribute('html','lang'),language);
    assert.equal(await page.getAttribute('html','data-theme'),theme);
    for(const view of ['search','downloads','settings']) {
      await page.click('#nav-'+view);
      await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(resolve)));
      const report=await page.evaluate(async()=>window.axe.run(document,{runOnly:{type:'tag',values:['wcag2a','wcag2aa','wcag21aa','wcag22aa']}}));
      const entry={language,theme,view,violations:report.violations,incomplete:report.incomplete};results.push(entry);
      if(report.violations.length) console.error(JSON.stringify({language,theme,view,violations:report.violations.map(v=>({id:v.id,impact:v.impact,help:v.help,targets:v.nodes.map(n=>n.target)}))}));
      if(view==='search')await page.screenshot({path:`artifacts/accessibility/${language}-${theme}.png`,fullPage:true});
    }
  }
  // Keyboard reachability and visible focus on the language selector.
  await page.click('#nav-search');
  await page.locator('#language-select').focus();
  await page.keyboard.press('Tab');
  assert.equal(await page.evaluate(()=>document.activeElement?.id),'appearance-select');
  assert.deepEqual(errors,[],'Unexpected UI errors');
  assert.equal(results.reduce((n,r)=>n+r.violations.length,0),0,'Accessibility violations found; inspect the report artifact');
  console.log('PASS: WCAG automated checks in Spanish/English, light/dark, Explore/Downloads/Settings; keyboard controls reachable');
} finally {
  fs.mkdirSync('artifacts/accessibility',{recursive:true});
  fs.writeFileSync('artifacts/accessibility/report.json',JSON.stringify(results,null,2));
  await browser?.close();
  try { process.platform==='win32'?server.kill():process.kill(-server.pid,'SIGTERM'); }catch{}
}
