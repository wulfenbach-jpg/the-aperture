// Manual real-browser smoke. Launch an isolated Chromium CDP endpoint first.
// Usage: node scripts/board-browser-smoke.cjs http://127.0.0.1:9337 33 OUTPUT_DIR
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const base = process.argv[2] || 'http://127.0.0.1:9337';
const expected = Number(process.argv[3] || 0);
const output = process.argv[4];
const targetUrl = process.argv[5] || 'http://127.0.0.1:9876/';
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));

(async () => {
  const endpoint = new URL(base);
  assert.ok(['127.0.0.1', 'localhost', '[::1]'].includes(endpoint.hostname), 'CDP must be loopback');
  const tabs = await (await fetch(`${base}/json`)).json();
  const tab = tabs.find(item => item.type === 'page' && item.url === 'about:blank');
  assert.ok(tab, 'Use a fresh isolated browser with an about:blank page');
  const socket = new WebSocket(tab.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {socket.onopen = resolve; socket.onerror = reject;});
  let serial = 0;
  const pending = new Map();
  const exceptions = [];
  socket.onmessage = event => {
    const message = JSON.parse(event.data);
    if (message.method === 'Runtime.exceptionThrown') exceptions.push(message.params.exceptionDetails.text);
    if (message.id && pending.has(message.id)) {
      const item = pending.get(message.id);
      pending.delete(message.id);
      clearTimeout(item.timer);
      if (message.error) item.reject(new Error(JSON.stringify(message.error)));
      else item.resolve(message.result);
    }
  };
  function send(method, params = {}) {
    const id = ++serial;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {pending.delete(id); reject(new Error(`CDP timeout: ${method}`));}, 15000);
      pending.set(id, {resolve, reject, timer});
      socket.send(JSON.stringify({id, method, params}));
    });
  }
  async function evaluate(expression) {
    const result = await send('Runtime.evaluate', {expression, returnByValue: true, awaitPromise: true});
    if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
    return result.result.value;
  }
  async function until(expression) {
    for (let i = 0; i < 80; i++) {
      if (await evaluate(expression)) return;
      await sleep(250);
    }
    throw new Error(`Browser condition timed out: ${expression}`);
  }
  async function click(selector) {
    const point = await evaluate(`(() => {const r=document.querySelector(${JSON.stringify(selector)}).getBoundingClientRect();return {x:r.x+r.width/2,y:r.y+r.height/2};})()`);
    await send('Input.dispatchMouseEvent', {type: 'mousePressed', button: 'left', clickCount: 1, ...point});
    await send('Input.dispatchMouseEvent', {type: 'mouseReleased', button: 'left', clickCount: 1, ...point});
  }
  async function screenshot(name) {
    if (!output) return;
    const result = await send('Page.captureScreenshot', {format: 'png', captureBeyondViewport: false});
    fs.mkdirSync(output, {recursive: true});
    fs.writeFileSync(path.join(output, name), Buffer.from(result.data, 'base64'));
  }
  try {
    await send('Runtime.enable');
    await send('Page.enable');
    await send('Network.enable');
    await send('Emulation.setDeviceMetricsOverride', {width: 1600, height: 1000, deviceScaleFactor: 1, mobile: false});
    await send('Page.navigate', {url: targetUrl});
    await until("document.querySelector('#source')?.dataset.live === 'true' && !document.querySelector('#refresh').disabled");
    const initial = await evaluate("({tickets:document.querySelectorAll('.ticket').length,columns:document.querySelectorAll('.column').length,summary:document.querySelector('#counts').textContent,source:document.querySelector('#source').textContent})");
    assert.equal(initial.columns, 6);
    assert.ok(initial.tickets > 0);
    if (expected) assert.equal(initial.tickets, expected);
    await screenshot('kanban-desktop.png');
    await evaluate("document.querySelector('#milestone').value=[...document.querySelector('#milestone').options].find(o=>o.value.startsWith('M1 ')).value;document.querySelector('#milestone').dispatchEvent(new Event('change'))");
    assert.equal(await evaluate("document.querySelectorAll('.ticket').length"), 5);
    await click('#search');
    await send('Input.insertText', {text: '#6'});
    await until("document.querySelectorAll('.ticket').length === 1");
    assert.equal(await evaluate("document.querySelector('.ticket a').href"), 'https://github.com/wulfenbach-jpg/the-aperture/issues/6');
    await evaluate("document.querySelector('#search').value='';document.querySelector('#milestone').value='';document.querySelector('#search').dispatchEvent(new Event('input'))");
    await send('Emulation.setDeviceMetricsOverride', {width: 390, height: 844, deviceScaleFactor: 1, mobile: true});
    assert.ok(await evaluate('document.documentElement.scrollWidth <= window.innerWidth'), 'Mobile page overflows horizontally');
    await screenshot('kanban-mobile.png');
    await send('Emulation.setDeviceMetricsOverride', {width: 1600, height: 1000, deviceScaleFactor: 1, mobile: false});
    await send('Network.setBlockedURLs', {urls: ['*api.github.com*']});
    await click('#refresh');
    await until("document.querySelector('#source').dataset.live === 'false' && document.querySelector('#source').textContent.startsWith('SNAPSHOT') && !document.querySelector('#refresh').disabled");
    const fallback = await evaluate("document.querySelector('#source').textContent");
    await send('Network.setBlockedURLs', {urls: ['*api.github.com*', '*issues.json*']});
    await click('#refresh');
    await until("document.querySelector('#counts').textContent === 'No issue data loaded' && !document.querySelector('#refresh').disabled");
    assert.equal(await evaluate("document.querySelectorAll('.ticket').length"), 0);
    await send('Network.setBlockedURLs', {urls: []});
    await click('#refresh');
    await until("document.querySelector('#source').dataset.live === 'true' && !document.querySelector('#refresh').disabled");
    assert.deepEqual(exceptions, []);
    const report = {passed: true, target: targetUrl, initial, milestone_filter: true, real_search_input: true, mobile_no_horizontal_overflow: true, dated_snapshot_fallback: fallback, complete_failure_state: true, live_recovery: true, javascript_exceptions: exceptions};
    if (output) fs.writeFileSync(path.join(output, 'browser-report.json'), JSON.stringify(report, null, 2));
    console.log(JSON.stringify(report, null, 2));
  } finally {
    await send('Page.navigate', {url: 'about:blank'}).catch(() => {});
    socket.close();
  }
})().catch(error => {console.error(error.stack); process.exitCode = 1;});
