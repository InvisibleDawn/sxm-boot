// SXM Boot — © 2026 Laurent Honoré — GPL-3.0-or-later
const T = window.__TAURI__;
const invoke = T.core.invoke;
const $ = (s) => document.querySelector(s);
const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
const TICK = '<svg viewBox="0 0 24 24"><path d="M5 12.5 10 17l9-10"/></svg>';

const state = {
  drives: [], drive: null, mount: null, onKey: [],
  q: '', picks: {}, customs: [],
  jobs: [], running: false, cancelled: false,
};

/* ---------- utilitaires ---------- */
function fmt(b) {
  if (!b) return '0 Go';
  return b >= 1e9 ? (b / 1e9).toFixed(1).replace('.', ',') + ' Go' : Math.round(b / 1e6) + ' Mo';
}
function toast(msg, ms = 4000) {
  const t = $('#toast'); t.textContent = msg; t.hidden = false;
  clearTimeout(toast.h); toast.h = setTimeout(() => (t.hidden = true), ms);
}
function log(msg, kind = '') {
  const d = new Date(), hh = String(d.getHours()).padStart(2, '0'), mm = String(d.getMinutes()).padStart(2, '0'), ss = String(d.getSeconds()).padStart(2, '0');
  const li = document.createElement('li');
  li.className = kind; li.innerHTML = `<time>[${hh}:${mm}:${ss}]</time>${esc(msg)}`;
  $('#log').appendChild(li); $('#log').scrollTop = 1e9;
}
const basename = (p) => p.split('/').pop();
const urlName = (u) => { try { return decodeURIComponent(new URL(u).pathname.split('/').pop()) || 'image.iso'; } catch { return 'image.iso'; } };
const allItems = () => [...window.CATALOG, ...state.customs];

/* ---------- navigation ---------- */
function show(view) {
  ['home', 'onkey', 'about'].forEach((v) => ($('#view-' + v).hidden = v !== view));
  document.querySelectorAll('.nav-item').forEach((b) => b.classList.toggle('on', b.dataset.view === view));
}
document.querySelectorAll('.nav-item').forEach((b) => (b.onclick = () => show(b.dataset.view)));

/* ---------- clés USB ---------- */
async function loadDrives(quiet) {
  const before = state.drives.map((d) => d.path).join();
  try { state.drives = await invoke('list_drives'); }
  catch (e) { if (!quiet) log('Impossible de lister les clés : ' + e, 'e'); state.drives = []; }
  const after = state.drives.map((d) => d.path).join();
  if (before !== after) {
    state.drives.filter((d) => !before.includes(d.path)).forEach((d) => log(`Clé USB détectée : ${d.path} (${d.name}, ${fmt(d.size)})`));
  }
  if (state.drive) state.drive = state.drives.find((d) => d.path === state.drive.path) || null;
  if (!state.drive && state.drives.length) await selectDrive(state.drives[0].path);
  else renderDrive();
}

async function selectDrive(path) {
  state.drive = state.drives.find((d) => d.path === path) || null;
  state.mount = null; state.onKey = [];
  if (state.drive && state.drive.ventoy) {
    try { state.mount = await invoke('mount_drive', { device: path }); await refreshOnKey(); }
    catch (e) { log('Impossible d\'ouvrir la clé : ' + e, 'e'); }
  }
  renderDrive(); renderCards();
}

function renderDrive() {
  const sel = $('#drive-sel');
  sel.innerHTML = state.drives.length
    ? state.drives.map((d) => `<option value="${esc(d.path)}" ${state.drive && state.drive.path === d.path ? 'selected' : ''}>${esc(d.name)} (${esc(d.path)})</option>`).join('')
    : '<option>Aucune clé USB détectée</option>';
  sel.disabled = !state.drives.length || state.running;
  const d = state.drive;
  $('#drive-meta').innerHTML = !d ? 'Branche une clé USB : elle apparaîtra ici automatiquement.'
    : `Capacité : <b>${fmt(d.size)}</b> · ${d.ventoy ? '<span class="tag ok">Ventoy installé</span>' : `<span class="tag warn">${state.os === 'macos' ? 'Clé vierge — à préparer sous Linux' : 'Clé vierge — à préparer'}</span>`}`;
  $('#space').hidden = !(d && d.ventoy && d.avail != null);
  $('#prep').hidden = !d || d.ventoy || state.os === 'macos';
  updateSpace(); updateGo();
}

$('#drive-sel').onchange = (e) => selectDrive(e.target.value);
$('#refresh').onclick = () => loadDrives();

async function ensureVentoy() {
  if (await invoke('ventoy_status')) return true;
  log('Téléchargement de Ventoy…');
  try { await invoke('fetch_ventoy'); log('Ventoy est prêt.', 's'); return true; }
  catch (e) { log('Échec du téléchargement de Ventoy : ' + e, 'e'); toast('Impossible de télécharger Ventoy.'); return false; }
}

$('#prep').onclick = async () => {
  const d = state.drive; if (!d) return;
  if (!(await ensureVentoy())) return;
  const ok = await T.dialog.ask(`Tout le contenu de « ${d.name} » (${d.path}, ${fmt(d.size)}) va être effacé.\n\nContinuer ?`,
    { title: 'Préparer la clé', kind: 'warning', okLabel: 'Effacer et préparer', cancelLabel: 'Annuler' });
  if (!ok) return;
  $('#prep').disabled = true; $('#prep').textContent = 'Préparation en cours…';
  log(`Installation de Ventoy sur ${d.path}…`);
  try {
    await invoke('install_ventoy', { device: d.path });
    await new Promise((r) => setTimeout(r, 2500));
    await loadDrives(true);
    if (!state.drive || !state.drive.ventoy) throw 'La clé n\'a pas été reconnue après installation. Rebranche-la.';
    await selectDrive(state.drive.path);
    log('Clé préparée avec Ventoy.', 's');
  } catch (e) { log(String(e), 'e'); toast(String(e), 8000); }
  $('#prep').disabled = false; $('#prep').textContent = 'Préparer une clé vierge (Ventoy)';
};

/* ---------- cartes ---------- */
function renderCards() {
  const q = state.q.toLowerCase();
  const onKeyNames = state.onKey.map((f) => f.name);
  const items = allItems().filter((x) => (x.name + ' ' + (x.desc || '')).toLowerCase().includes(q));
  $('#cards').innerHTML = items.map((x) => {
    const p = state.picks[x.id];
    const present = x.url && onKeyNames.includes(urlName(x.url));
    const src = p && p.kind === 'file' ? `<div class="src">Fichier local : ${esc(basename(p.path))}</div>` : '';
    return `<div class="card ${p ? 'sel' : ''} ${present ? 'onkey-mark' : ''}" data-id="${x.id}" role="checkbox" aria-checked="${!!p}" tabindex="0" aria-label="${esc(x.name)}">
      <div class="banner" style="--c:${x.color}"><img class="wm" src="${x.logo}" alt=""><span class="tick">${p ? TICK : ''}</span></div>
      <div class="top"><span class="disc" style="background:${x.color}"><img src="${x.logo}" alt=""></span>
        <div><h4>${esc(x.name)}</h4></div></div>
      <p>${esc(x.desc || '')}</p>${src}
      <div class="foot"><span class="pill">${esc(x.version || 'ISO perso')}</span>
        ${x.url ? `<span class="size">≈ ${fmt(x.size)}</span>` : `<span class="size">${fmt(p && p.size)}</span>`}</div>
      ${x.url ? '<button class="alt" data-local>J\'ai déjà l\'ISO sur mon PC…</button>' : ''}
    </div>`;
  }).join('') + `<button class="card add" id="add-iso"><span class="disc"><svg viewBox="0 0 24 24"><path d="M12 5v14M5 12h14"/></svg></span>
      <h4>Ajouter une autre ISO</h4><p>N'importe quel fichier .iso de ton PC</p></button>`;

  $('#cards').querySelectorAll('.card[data-id]').forEach((c) => {
    const toggle = () => togglePick(c.dataset.id);
    c.onclick = toggle;
    c.onkeydown = (e) => { if (e.key === ' ' || e.key === 'Enter') { e.preventDefault(); toggle(); } };
    const loc = c.querySelector('[data-local]');
    if (loc) loc.onclick = async (e) => { e.stopPropagation(); const p = await pickFile(); if (p) { state.picks[c.dataset.id] = p; renderCards(); } };
  });
  $('#add-iso').onclick = async () => {
    const p = await pickFile(); if (!p) return;
    const id = 'c' + Date.now();
    state.customs.push({ id, logo: 'img/disc.svg', color: '#5A6878', name: basename(p.path).replace(/\.(iso|img)$/i, ''), desc: 'ISO ajoutée depuis ton PC.' });
    state.picks[id] = p; renderCards();
  };
  updateSpace(); updateGo();
}

function togglePick(id) {
  if (state.running) return;
  const x = allItems().find((i) => i.id === id);
  if (state.picks[id]) { delete state.picks[id]; if (!x.url) state.customs = state.customs.filter((c) => c.id !== id); }
  else if (x.url) state.picks[id] = { kind: 'url', url: x.url, sums: x.sums, filename: urlName(x.url), size: x.size };
  renderCards();
}

async function pickFile() {
  const path = await T.dialog.open({ multiple: false, filters: [{ name: 'Images disque', extensions: ['iso', 'img'] }] });
  if (!path) return null;
  let size = 0; try { size = await invoke('file_size', { path }); } catch {}
  return { kind: 'file', path, size };
}

$('#q').addEventListener('input', (e) => { state.q = e.target.value; renderCards(); });

function selectedSize() { return Object.values(state.picks).reduce((s, p) => s + (p.size || 0), 0); }

function updateSpace() {
  const d = state.drive; if (!d || d.avail == null) return;
  const total = d.used + d.avail, add = selectedSize();
  $('#sp-used').style.width = (d.used / total) * 100 + '%';
  $('#sp-add').style.width = Math.min(100, (add / total) * 100) + '%';
  $('#sp-u').textContent = fmt(d.used);
  $('#sp-a').textContent = fmt(add);
  $('#sp-f').textContent = fmt(Math.max(0, d.avail - add));
  $('#sp-f').style.color = add > d.avail ? 'var(--danger)' : '';
}

function updateGo() {
  const n = Object.keys(state.picks).length, d = state.drive;
  $('#go').disabled = state.running || !n || !d || !d.ventoy || !state.mount;
  $('#go-label').textContent = state.running ? 'Création en cours…' : n ? `Créer / Mettre à jour la clé (${n})` : 'Créer / Mettre à jour la clé';
}

/* ---------- sur la clé ---------- */
async function refreshOnKey() {
  if (!state.mount) { state.onKey = []; }
  else { try { state.onKey = await invoke('list_isos', { mountpoint: state.mount }); } catch { state.onKey = []; } }
  const b = $('#onkey-badge'); b.textContent = state.onKey.length; b.hidden = !state.onKey.length;
  $('#onkey-sub').textContent = state.drive ? `${state.drive.name} (${state.drive.path})` : 'Choisis d\'abord une clé sur l\'accueil.';
  $('#onkey').innerHTML = state.onKey.length
    ? state.onKey.map((f) => `<li><div><div class="n">${esc(f.name)}</div><div class="z">${fmt(f.size)}</div></div><button class="danger" data-n="${esc(f.name)}">Retirer</button></li>`).join('')
    : '<li class="z">Aucune ISO sur la clé pour l\'instant.</li>';
  $('#onkey').querySelectorAll('[data-n]').forEach((btn) => (btn.onclick = async () => {
    const ok = await T.dialog.ask(`Retirer ${btn.dataset.n} de la clé ?`, { title: 'Retirer une distribution', kind: 'warning' });
    if (!ok) return;
    try {
      await invoke('delete_iso', { mountpoint: state.mount, name: btn.dataset.n });
      log(`${btn.dataset.n} retiré de la clé.`);
      await loadDrives(true); await refreshOnKey(); renderCards();
    } catch (e) { toast(String(e)); }
  }));
}

/* ---------- création ---------- */
function renderJobs() {
  $('#jobs').innerHTML = state.jobs.map((j) => {
    const pct = j.total ? Math.round((j.done / j.total) * 100) : 0;
    return `<div class="job ${j.status}"><span class="jd" style="background:${j.color}"><img src="${j.logo}" alt=""></span>
      <div class="mid"><strong>${esc(j.name)}</strong>${j.status === 'run' ? `<div class="bar"><div class="bar-add" style="width:${pct}%"></div></div>` : ''}</div>
      <span class="st">${esc(j.info)}</span></div>`;
  }).join('');
  const tot = state.jobs.reduce((s, j) => s + (j.total || 1), 0);
  const done = state.jobs.reduce((s, j) => s + (j.status === 'ok' ? (j.total || 1) : j.total ? j.done : 0), 0);
  const pct = tot ? Math.floor((done / tot) * 100) : 0;
  $('#pct').textContent = pct + ' %'; $('#gbar').style.width = pct + '%';
}

let tick = { t: 0, b: 0 }, speed = 0;
T.event.listen('progress', ({ payload }) => {
  const j = state.jobs.find((x) => x.id === payload.id); if (!j) return;
  j.done = payload.done; if (payload.total) j.total = payload.total;
  const now = performance.now();
  if (now - tick.t > 1000) { speed = ((payload.done - tick.b) / (now - tick.t)) * 1000; tick = { t: now, b: payload.done }; }
  const pct = j.total ? Math.round((j.done / j.total) * 100) : 0;
  j.info = { download: `Téléchargement ${pct} %`, copy: `Copie ${pct} %`, verify: 'Vérification SHA256…', done: 'Terminé' }[payload.phase] || j.info;
  if (speed > 0 && j.total) $('#gtext').textContent = `${j.name} : ${fmt(j.done)} sur ${fmt(j.total)} · ${(speed / 1e6).toFixed(0)} Mo/s`;
  renderJobs();
});

$('#go').onclick = async () => {
  const d = state.drive;
  if (d.avail != null && selectedSize() > d.avail) { toast('Il n\'y a pas assez de place sur la clé.'); return; }
  const verify = $('#opt-sha').checked;
  state.jobs = Object.entries(state.picks).map(([id, p]) => {
    const x = allItems().find((i) => i.id === id) || {};
    return { id, p, name: x.name || id, logo: x.logo, color: x.color, status: 'wait', done: 0, total: p.size || 0, info: 'En attente' };
  });
  state.running = true; state.cancelled = false; updateGo(); renderDrive();
  $('#prog-actions').hidden = false; $('#cancel').hidden = false; $('#eject').hidden = true;
  log(`Création lancée : ${state.jobs.length} élément(s) vers ${d.path}.`);
  let errors = 0;
  for (const j of state.jobs) {
    if (state.cancelled) { j.status = 'err'; j.info = 'Annulé'; continue; }
    j.status = 'run'; j.done = 0; tick = { t: performance.now(), b: 0 }; renderJobs();
    try {
      if (j.p.kind === 'file') {
        log(`Copie de ${j.name}…`);
        await invoke('copy_iso', { id: j.id, src: j.p.path, mountpoint: state.mount });
        j.info = 'Copiée';
      } else {
        let sha = null;
        if (verify && j.p.sums) {
          j.info = 'Récupération de la somme SHA256…'; renderJobs();
          try { sha = await invoke('fetch_sha', { sumsUrl: j.p.sums, filename: j.p.filename }); } catch {}
        }
        log(`Téléchargement de ${j.name}…`);
        await invoke('download_iso', { id: j.id, url: j.p.url, filename: j.p.filename, mountpoint: state.mount, sha256: sha || null });
        j.info = sha ? 'Ajoutée · SHA256 vérifié' : 'Ajoutée';
      }
      j.status = 'ok'; log(`${j.name} ajoutée à la clé.`, 's');
    } catch (e) { j.status = 'err'; j.info = String(e); errors++; log(`${j.name} : ${e}`, 'e'); }
    renderJobs();
  }
  state.running = false;
  state.picks = Object.fromEntries(state.jobs.filter((j) => j.status !== 'ok').map((j) => [j.id, j.p]));
  $('#cancel').hidden = true; $('#eject').hidden = false;
  $('#gtext').textContent = state.cancelled ? 'Opération annulée.' : errors ? 'Terminé avec des erreurs : les éléments en échec restent cochés.' : 'Ta clé est prête ! Démarre un PC dessus (F12, F11 ou Échap au démarrage selon la marque).';
  log(state.cancelled ? 'Opération annulée.' : errors ? 'Terminé avec des erreurs.' : 'Clé prête.', errors || state.cancelled ? 'e' : 's');
  await loadDrives(true); await refreshOnKey(); renderCards();
  if (!errors && !state.cancelled && $('#opt-eject').checked) $('#eject').click();
};

$('#cancel').onclick = async () => { state.cancelled = true; await invoke('cancel'); };
$('#eject').onclick = async () => {
  try { await invoke('eject', { device: state.drive.path }); log('Clé éjectée.', 's'); toast('Clé éjectée, tu peux la retirer.'); }
  catch (e) { toast('Éjection impossible : ' + e); return; }
  Object.assign(state, { drive: null, mount: null, onKey: [], jobs: [] });
  $('#prog-actions').hidden = true; renderJobs(); $('#pct').textContent = '0 %';
  await loadDrives(true); await refreshOnKey(); renderCards();
};

/* ---------- démarrage ---------- */
log('Application démarrée.');
invoke('platform').then((os) => { state.os = os; renderDrive(); if (os === 'macos') log('Version macOS : la préparation d\'une clé vierge se fait sous Linux.'); }).catch(() => {});
renderCards(); renderJobs();
loadDrives().then(() => { if (!state.drives.length) log('Aucune clé USB détectée.'); else log('Prêt.'); });
invoke('ventoy_status').then((p) => log(p ? 'Moteur Ventoy prêt.' : 'Ventoy sera téléchargé au besoin.'));
setInterval(() => { if (!state.running) loadDrives(true); }, 4000);
