/* Token-free, issue-backed kanban. No third-party scripts or HTML injection. */
(function (root) {
  'use strict';
  const columns = ['backlog', 'ready', 'in-progress', 'review', 'blocked', 'done'];
  function statusOf(issue) {
    if (issue.state === 'closed') return 'done';
    return (issue.labels || []).map(label => typeof label === 'string' ? label : label.name)
      .map(label => label.startsWith('status:') ? label.slice(7) : '')
      .find(status => columns.includes(status) && status !== 'done') || 'backlog';
  }
  const repository = 'wulfenbach-jpg/the-aperture';
  function normalize(rows) {
    if (!Array.isArray(rows)) throw new Error('Invalid issue list');
    const seen = new Set();
    return rows.filter(issue => !issue.pull_request).map(issue => {
      if (!Number.isSafeInteger(issue.number) || issue.number < 1 || seen.has(issue.number)) throw new Error('Invalid or duplicate issue number');
      seen.add(issue.number);
      return {
        number: issue.number, title: String(issue.title || 'Untitled issue'),
        state: issue.state === 'closed' ? 'closed' : 'open',
        labels: (issue.labels || []).map(label => typeof label === 'string' ? label : label.name).filter(label => typeof label === 'string'),
        milestone: typeof issue.milestone === 'string' ? issue.milestone : issue.milestone?.title || '',
        url: `https://github.com/${repository}/issues/${issue.number}`
      };
    });
  }
  async function request(fetcher, url) {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), 10000);
    try {
      const response = await fetcher(url, {signal: controller.signal, headers: {Accept: 'application/vnd.github+json'}});
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      return {data: await response.json(), next: /rel="next"/.test(response.headers?.get('link') || '')};
    } finally { clearTimeout(timer); }
  }
  async function load(fetcher) {
    try {
      let rows = [];
      for (let page = 1; page <= 100; page++) {
        const response = await request(fetcher, `https://api.github.com/repos/${repository}/issues?state=all&per_page=100&page=${page}`);
        if (!Array.isArray(response.data)) throw new Error('Invalid GitHub issue response');
        rows = rows.concat(response.data);
        if (!response.next) return {live: true, generated_at: new Date().toISOString(), issues: normalize(rows)};
      }
      throw new Error('Issue pagination limit exceeded; refusing a partial board');
    } catch (error) {
      const response = await request(fetcher, 'issues.json');
      if (response.data.repository !== repository || !response.data.generated_at || !Number.isFinite(Date.parse(response.data.generated_at))) throw new Error('Invalid fallback snapshot');
      return {live: false, reason: String(error.message || error), generated_at: response.data.generated_at, issues: normalize(response.data.issues)};
    }
  }
  function select(issues, {milestone = '', query = ''} = {}) {
    const term = query.trim().toLowerCase();
    const priority = issue => issue.labels.find(label => /^priority:P[012]$/.test(label)) || 'priority:P9';
    return issues.filter(issue => (!milestone || issue.milestone === milestone) &&
      (!term || `${issue.title} #${issue.number} ${issue.milestone} ${issue.labels.join(' ')}`.toLowerCase().includes(term)))
      .sort((a, b) => priority(a).localeCompare(priority(b)) || a.number - b.number);
  }
  const api = {columns, statusOf, normalize, load, select};
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.ApertureBoard = api;
})(globalThis);
