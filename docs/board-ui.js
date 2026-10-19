/* DOM rendering uses textContent for every remote value. */
(function () {
  'use strict';
  const model = globalThis.ApertureBoard;
  const byId = id => document.getElementById(id);
  const names = {backlog: 'Backlog', ready: 'Ready', 'in-progress': 'In progress', review: 'Review', blocked: 'Blocked', done: 'Done'};
  let current = null;
  function element(tag, text, className) {
    const node = document.createElement(tag);
    if (text !== undefined) node.textContent = text;
    if (className) node.className = className;
    return node;
  }
  function render() {
    if (!current) return;
    const selected = model.select(current.issues, {milestone: byId('milestone').value, query: byId('search').value});
    const board = byId('board');
    board.replaceChildren();
    for (const status of model.columns) {
      const section = element('section', undefined, 'column');
      section.dataset.status = status;
      section.setAttribute('aria-labelledby', `heading-${status}`);
      const items = selected.filter(issue => model.statusOf(issue) === status);
      const title = element('h2', names[status]);
      title.id = `heading-${status}`;
      title.append(element('span', String(items.length)));
      const list = element('div', undefined, 'cards');
      list.tabIndex = 0;
      list.setAttribute('aria-label', `${names[status]} issues`);
      for (const issue of items) {
        const ticket = element('article', undefined, 'ticket');
        const tags = issue.labels.filter(label => label.startsWith('priority:') || label.startsWith('area:')).join(' / ');
        ticket.append(element('p', tags, 'tags'));
        const link = element('a', `#${issue.number} ${issue.title}`);
        link.href = issue.url;
        ticket.append(link, element('small', issue.milestone || 'No milestone'));
        if (issue.state !== 'closed' && issue.labels.filter(label => label.startsWith('status:')).length !== 1) {
          ticket.append(element('small', 'Check status labels on GitHub', 'conflict'));
        }
        list.append(ticket);
      }
      if (!items.length) list.append(element('p', 'No issues in this column.', 'empty-column'));
      section.append(title, list);
      board.append(section);
    }
    const closed = current.issues.filter(issue => issue.state === 'closed').length;
    byId('counts').textContent = `${selected.length} shown / ${current.issues.length} total · ${closed} closed · ${current.issues.length - closed} open`;
    byId('empty').hidden = selected.length !== 0;
  }
  async function refresh() {
    const button = byId('refresh');
    button.disabled = true;
    button.textContent = 'Refreshing…';
    try {
      current = await model.load(globalThis.fetch.bind(globalThis));
      const selectedMilestone = byId('milestone').value;
      const options = byId('milestone');
      options.replaceChildren();
      const all = element('option', 'All milestones');
      all.value = '';
      options.append(all);
      const milestones = [...new Set(current.issues.map(issue => issue.milestone).filter(Boolean))].sort();
      for (const milestone of milestones) {
        const option = element('option', milestone);
        option.value = milestone;
        options.append(option);
      }
      options.value = milestones.includes(selectedMilestone) ? selectedMilestone : '';
      const source = byId('source');
      source.dataset.live = String(current.live);
      source.textContent = `${current.live ? 'Live GitHub read' : 'SNAPSHOT — not live'} · ${new Date(current.generated_at).toLocaleString()}${current.live ? '' : ` · API unavailable (${current.reason})`}`;
      render();
    } catch (error) {
      current = null;
      byId('board').replaceChildren();
      byId('counts').textContent = 'No issue data loaded';
      byId('source').dataset.live = 'false';
      byId('source').textContent = `Unavailable: ${String(error.message || error)}. Open the GitHub issue list above.`;
      byId('empty').hidden = true;
    } finally {
      button.disabled = false;
      button.textContent = 'Refresh from GitHub';
    }
  }
  byId('search').addEventListener('input', render);
  byId('milestone').addEventListener('change', render);
  byId('refresh').addEventListener('click', refresh);
  refresh();
})();
