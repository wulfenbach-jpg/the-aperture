const {test} = require('node:test');
const assert = require('node:assert/strict');
const board = require('../docs/board.js');

test('closed issues are Done, regardless of a stale status label', () => {
  assert.equal(board.statusOf({state: 'closed', labels: ['status:in-progress']}), 'done');
  assert.equal(board.statusOf({state: 'open', labels: ['status:review']}), 'review');
  assert.equal(board.statusOf({state: 'open', labels: []}), 'backlog');
  assert.equal(board.statusOf({state: 'open', labels: ['status:invalid']}), 'backlog');
});

test('loads every API page, filters pull requests, normalizes labels and builds safe issue links', async () => {
  const calls = [];
  const fetcher = async (url) => {
    calls.push(url);
    return {ok: true, headers: {get: () => calls.length === 1 ? '<next>; rel="next"' : ''}, json: async () => calls.length === 1
      ? [{number: 6, title: '<script>not HTML</script>', state: 'open', labels: [{name: 'status:ready'}], milestone: {title: 'M1'}}, {number: 7, title: 'PR', pull_request: {}}]
      : [{number: 8, title: 'Views', state: 'open', labels: [{name: 'status:backlog'}], milestone: {title: 'M1'}}]};
  };
  const result = await board.load(fetcher);
  assert.equal(result.live, true);
  assert.equal(calls.length, 2);
  assert.equal(result.issues.length, 2);
  assert.equal(result.issues[0].title, '<script>not HTML</script>');
  assert.equal(result.issues[0].url, 'https://github.com/wulfenbach-jpg/the-aperture/issues/6');
  assert.equal(board.statusOf(result.issues[0]), 'ready');
});

test('an API failure uses an explicitly dated snapshot, never a fake live result', async () => {
  const result = await board.load(async (url) => url === 'issues.json'
    ? {ok: true, json: async () => ({repository: 'wulfenbach-jpg/the-aperture', generated_at: '2026-10-19T00:00:00Z', issues: [{number: 1, title: 'Snapshot', state: 'open', labels: []}]})}
    : {ok: false, status: 403});
  assert.equal(result.live, false);
  assert.equal(result.generated_at, '2026-10-19T00:00:00Z');
  assert.match(result.reason, /403/);
  assert.equal(result.issues.length, 1);
});

test('filters milestone and search without hiding the applied filter, and sorts by priority', () => {
  const issues = board.normalize([
    {number: 8, title: 'Views', milestone: 'M1', labels: ['priority:P1']},
    {number: 6, title: 'Seats', milestone: 'M1', labels: ['priority:P0']},
    {number: 1, title: 'Public baseline', milestone: 'M0', labels: ['priority:P0']}
  ]);
  assert.deepEqual(board.select(issues, {milestone: 'M1', query: ''}).map(i => i.number), [6, 8]);
  assert.deepEqual(board.select(issues, {milestone: '', query: '  #6  '}).map(i => i.number), [6]);
  assert.equal(board.select(issues, {milestone: 'M0', query: 'Seats'}).length, 0);
  assert.throws(() => board.normalize([{number: 1}, {number: 1}]), /duplicate/);
  assert.throws(() => board.normalize([{number: -1}]), /Invalid/);
});
