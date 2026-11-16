// SXM Boot — © 2026 Laurent Honoré — GPL-3.0-or-later
// Mises à jour : vérification au démarrage, petite fenêtre extensible avec les nouveautés.
(function () {
  const T = window.__TAURI__, invoke = T.core.invoke;
  const $ = (s) => document.querySelector(s);
  const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
  let info = null, busy = false;

  // Transforme les notes de version (titres ### et puces -) en sections.
  function parseNotes(md) {
    const sections = []; let cur = null;
    for (const raw of (md || '').split(/\r?\n/)) {
      const line = raw.trim();
      const h = line.match(/^#{2,4}\s+(.+)/);
      if (h) { cur = { title: h[1].replace(/\*\*/g, ''), items: [] }; sections.push(cur); continue; }
      const li = line.match(/^[-*]\s+(.+)/);
      if (li) {
        if (!cur) { cur = { title: t('Nouveautés'), items: [] }; sections.push(cur); }
        cur.items.push(li[1].replace(/\*\*(.+?)\*\*/g, '$1').replace(/`(.+?)`/g, '$1'));
      }
    }
    return sections.filter((s) => s.items.length);
  }

  function kind(title) {
    const x = title.toLowerCase();
    if (/correct|fix|bug/.test(x)) return 'fix';
    if (/sécurité|securit|fiabilit/.test(x)) return 'sec';
    return 'new';
  }

  function render() {
    const box = $('#upd');
    if (!info) { box.hidden = true; return; }
    const sections = parseNotes(info.notes);
    const counts = { new: 0, fix: 0, sec: 0 };
    sections.forEach((s) => (counts[kind(s.title)] += s.items.length));
    const summary = [
      counts.new && t('{n} nouveauté(s)', { n: counts.new }),
      counts.fix && t('{n} correction(s)', { n: counts.fix }),
      counts.sec && t('{n} amélioration(s) de sécurité', { n: counts.sec }),
    ].filter(Boolean).join(' · ');
    $('#upd-title').textContent = t('SXM Boot {v} est disponible', { v: info.version });
    $('#upd-sub').textContent = summary || t('Tu as la version {v}.', { v: info.current });
    $('#upd-list').innerHTML = sections.length ? sections.map((s) => `
      <section class="upd-sec ${kind(s.title)}"><h4>${esc(s.title)}</h4>
        <ul>${s.items.map((i) => `<li>${esc(i)}</li>`).join('')}</ul></section>`).join('')
      : `<p class="upd-empty">${t('Aucun détail fourni pour cette version.')}</p>`;
    $('#upd-toggle').hidden = !sections.length;
    box.hidden = false;
  }

  function setState(msg, opts = {}) {
    $('#upd-status').textContent = msg || '';
    $('#upd-status').hidden = !msg;
    $('#upd-bar').hidden = !opts.progress;
    $('#upd-actions').hidden = !!opts.hideActions;
    $('#upd-restart').hidden = !opts.restart;
  }

  async function check(manual) {
    try {
      info = await invoke('check_update');
      if (info) { render(); if (window.log) log(t('Mise à jour disponible : version {v}.', { v: info.version })); }
      else if (manual) window.toast && toast(t('Tu as déjà la dernière version.'));
    } catch (e) {
      if (manual) window.toast && toast(t('Impossible de vérifier les mises à jour : {e}', { e }));
    }
  }

  T.event.listen('progress', ({ payload }) => {
    if (payload.id !== 'update') return;
    const pct = payload.total ? Math.round((payload.done / payload.total) * 100) : 0;
    $('#upd-fill').style.width = pct + '%';
    if (payload.phase === 'verify') setState(t('Vérification SHA256…'), { progress: true, hideActions: true });
    else if (payload.phase === 'download') setState(t('Téléchargement de la mise à jour… {p} %', { p: pct }), { progress: true, hideActions: true });
  });

  $('#upd-toggle').onclick = () => {
    const open = $('#upd').classList.toggle('open');
    $('#upd-toggle').setAttribute('aria-expanded', open);
    $('#upd-toggle-label').textContent = open ? t('Masquer les nouveautés') : t('Voir les nouveautés');
  };
  $('#upd-later').onclick = () => { $('#upd').hidden = true; };
  $('#upd-close').onclick = () => { if (!busy) $('#upd').hidden = true; };
  $('#upd-restart').onclick = () => invoke('restart_app');

  $('#upd-go').onclick = async () => {
    if (busy || !info) return;
    if (typeof state !== 'undefined' && state.running) { toast(t('Attends la fin de la création de la clé.')); return; }
    busy = true;
    setState(t('Téléchargement de la mise à jour… {p} %', { p: 0 }), { progress: true, hideActions: true });
    try {
      const r = await invoke('install_update', { url: info.asset_url, name: info.asset_name, digest: info.digest || null });
      if (r === 'installed') setState(t('Mise à jour installée. Redémarre SXM Boot pour l\'utiliser.'), { hideActions: true, restart: true });
      else if (r === 'dmg') setState(t('Le fichier .dmg est ouvert : glisse SXM Boot dans Applications pour remplacer l\'ancienne version.'), { hideActions: true });
      else setState(t('Installation en cours… SXM Boot va redémarrer.'), { hideActions: true });
    } catch (e) {
      setState(t('Échec de la mise à jour : {e}', { e }));
    }
    busy = false;
  };

  window.checkForUpdates = check;
  $('#about-update').onclick = () => check(true);
  window.addEventListener('langchange', render);

  // Vérification 5 secondes après le démarrage, puis toutes les 6 heures.
  setTimeout(() => check(false), 5000);
  setInterval(() => check(false), 6 * 3600 * 1000);
})();
