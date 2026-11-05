<script lang="ts">
  // Farming runs: each run the session journal shows (launch saves, then the end save with the gains), with what
  // the player adds by hand (card, essence, rerolls...), the farm level in force, and averages to compare setups.
  import { api } from '../lib/api'
  import { shortNumber } from '../lib/format'
  import { asset } from '../lib/game'
  import { fmtDate, fmtNumber, i18n, t } from '../lib/i18n.svelte'
  import { findRuns, gainOf, resourceGains, type Run } from '../lib/runs'
  import { planner } from '../lib/store.svelte'
  import type { RunBook, RunNote, SessionSnapshot } from '../lib/types'

  let { journal }: { journal: SessionSnapshot[] } = $props()

  const MAIN = 'ElvenFragments'
  const save = $derived(planner.character?.save ?? '')
  let book = $state<RunBook>({ notes: {}, levels: [] })
  let error = $state<string | null>(null)
  $effect(() => {
    if (save) api.runBook(save).then((b) => (book = b), (e) => (error = String(e)))
  })

  const runs = $derived(findRuns(journal).reverse())
  const fortunes = $derived(planner.catalog?.fortunes ?? [])
  const boons = $derived(fortunes.filter((f) => f.positive))
  const banes = $derived(fortunes.filter((f) => !f.positive))
  const fortune = (tag: string | undefined) => fortunes.find((f) => f.tag === tag)
  const resourceInfo = $derived(new Map((planner.catalog?.resources ?? []).map((r) => [r.tag, r])))
  const humanize = (k: string) => k.replace(/^KeyMaterial\./, '').replace(/([a-z])([A-Z])/g, '$1 $2').replace(/[._]/g, ' ')
  const resName = (k: string) => resourceInfo.get(k)?.name ?? humanize(k)

  const levelAt = (time: number) => [...book.levels].reverse().find(([since]) => since <= time)?.[1]
  const noteOf = (r: Run): RunNote => book.notes[r.id] ?? {}
  const levelOf = (r: Run) => noteOf(r).rupture ?? levelAt(r.end.time)
  const hours = (r: Run) => (r.end.time - r.start) / 3600

  // Filters.
  let level = $state<number | 'all'>('all')
  let boon = $state<string>('all')
  let onlyFortune = $state(true)
  const levels = $derived([...new Set(runs.map(levelOf).filter((l): l is number => l !== undefined))].sort((a, b) => b - a))
  const shown = $derived(runs.filter((r) => (!onlyFortune || r.fortune) && (level === 'all' || levelOf(r) === level) && (boon === 'all' || noteOf(r).boon === boon)))

  const total = (rs: Run[]) => rs.reduce((n, r) => n + gainOf(r, MAIN), 0)
  const time = (rs: Run[]) => rs.reduce((n, r) => n + hours(r), 0)
  const perRun = (rs: Run[]) => (rs.length ? total(rs) / rs.length : 0)
  const perHour = (rs: Run[]) => (time(rs) > 0 ? total(rs) / time(rs) : 0)
  /** Average by card (boon, then bane) among the shown runs that have one. */
  function byCard(kind: 'boon' | 'bane') {
    const groups = new Map<string, Run[]>()
    for (const r of shown) {
      const tag = noteOf(r)[kind]
      if (tag) groups.set(tag, [...(groups.get(tag) ?? []), r])
    }
    return [...groups].map(([tag, rs]) => ({ tag, rs })).sort((a, b) => perRun(b.rs) - perRun(a.rs))
  }

  const cardGroups = $derived([
    { kind: 'boon', groups: byCard('boon') },
    { kind: 'bane', groups: byCard('bane') },
  ])

  async function setNote(r: Run, patch: Partial<RunNote>) {
    const note: RunNote = { ...noteOf(r), ...patch }
    for (const k of Object.keys(note) as (keyof RunNote)[]) if (note[k] === undefined || note[k] === '' || Number.isNaN(note[k])) delete note[k]
    try {
      book = await api.setRunNote(save, r.id, note)
    } catch (e) {
      error = String(e)
    }
  }
  const num = (v: string) => (v.trim() === '' ? undefined : Number(v))

  // The level the player farms now: runs take it until the next power-up.
  let newLevel = $state('')
  const currentLevel = $derived(book.levels.length ? book.levels[book.levels.length - 1] : undefined)
  async function saveLevel() {
    const l = Number(newLevel)
    if (!Number.isFinite(l) || l <= 0) return
    // From the last run on: the run just played counts at the new level if it was entered after it.
    try {
      book = await api.setFarmLevel(save, Date.now() / 1000, l)
      newLevel = ''
    } catch (e) {
      error = String(e)
    }
  }

  let open = $state<string | null>(null)
  const signed = (v: number) => (v > 0 ? '+' : '') + shortNumber(v)
  const clock = (unix: number) => new Intl.DateTimeFormat(i18n.language.intl, { dateStyle: 'short', timeStyle: 'short' }).format(new Date(unix * 1000))
  const minutes = (r: Run) => `${Math.round((r.end.time - r.start) / 60)} min`

  // Rows as in the players' spreadsheets, to paste into one.
  let copied = $state(false)
  async function copy() {
    const head = ['Date', 'Rupture', 'Boon', 'Bane', 'Essence', 'Rerolls', 'Before', 'After', 'Elven Frags', 'Elites', 'Mobs', 'EF cont', 'Minutes', 'Notes']
    const rows = [...shown].reverse().map((r) => {
      const n = noteOf(r)
      return [fmtDate(r.end.time), levelOf(r) ?? '', fortune(n.boon)?.description ?? '', fortune(n.bane)?.description ?? '', n.essence ?? '', n.rerolls ?? (r.fortune ? r.rerolls : ''), r.base.resources[MAIN] ?? 0, r.end.resources[MAIN] ?? 0, gainOf(r, MAIN), n.elites ?? '', n.mobs ?? '', n.containers ?? '', Math.round((r.end.time - r.start) / 60), n.notes ?? '']
    })
    try {
      await navigator.clipboard.writeText([head, ...rows].map((r) => r.join('\t')).join('\n'))
      copied = true
      setTimeout(() => (copied = false), 1500)
    } catch (e) {
      error = String(e)
    }
  }
</script>

<div class="runs">
  <div class="bar">
    <label class="pick">
      {t('runs.farmLevel')}
      <input class="field lvl" type="number" min="1" placeholder={currentLevel ? String(currentLevel[1]) : '—'} bind:value={newLevel} onkeydown={(e) => e.key === 'Enter' && saveLevel()} />
      <button class="btn" disabled={!newLevel} onclick={saveLevel}>{t('runs.setLevel')}</button>
    </label>
    {#if currentLevel}<span class="dim">{t('runs.levelSince', { level: currentLevel[1], date: fmtDate(currentLevel[0]) })}</span>{/if}
    <span class="grow"></span>
    <label class="pick"><input type="checkbox" bind:checked={onlyFortune} />{t('runs.onlyFortune')}</label>
    {#if levels.length}
      <label class="pick">
        {t('runs.level')}
        <select class="field" bind:value={level}>
          <option value="all">{t('runs.all')}</option>
          {#each levels as l (l)}<option value={l}>{l}</option>{/each}
        </select>
      </label>
    {/if}
    <label class="pick">
      {t('runs.boon')}
      <select class="field" bind:value={boon}>
        <option value="all">{t('runs.all')}</option>
        {#each boons as f (f.tag)}<option value={f.tag}>{f.description}</option>{/each}
      </select>
    </label>
    <button class="btn" disabled={!shown.length} onclick={copy}>{copied ? t('runs.copied') : t('runs.copy')}</button>
  </div>
  {#if error}<pre class="error">{error}</pre>{/if}

  {#if !runs.length}
    <p class="dim">{t('runs.empty')}</p>
  {:else}
    <div class="tiles">
      <div class="tile"><span class="k">{t('runs.count')}</span><span class="v">{shown.length}</span><span class="d">{t('runs.time', { h: fmtNumber(time(shown), 1) })}</span></div>
      <div class="tile main">
        <span class="k">{#if resourceInfo.get(MAIN)?.icon}<img src={asset(resourceInfo.get(MAIN)?.icon)} alt="" />{/if}{t('runs.perRun', { name: resName(MAIN) })}</span>
        <span class="v">{shortNumber(perRun(shown))}</span>
        <span class="d">{t('runs.total', { n: shortNumber(total(shown)) })}</span>
      </div>
      <div class="tile"><span class="k">{t('runs.perHour')}</span><span class="v">{shortNumber(perHour(shown))}</span><span class="d">{resName(MAIN)}</span></div>
    </div>

    {#each cardGroups as c (c.kind)}
      {#if c.groups.length}
        <div class="cards">
          <span class="dim">{t(c.kind === 'boon' ? 'runs.byBoon' : 'runs.byBane')}</span>
          {#each c.groups as g (g.tag)}
            {@const f = fortune(g.tag)}
            <span class="card" title={f?.description}>
              {#if f?.icon}<img src={asset(f.icon)} alt="" />{/if}{f?.description ?? g.tag}
              <b>{shortNumber(perRun(g.rs))}</b><em>× {g.rs.length}</em>
            </span>
          {/each}
        </div>
      {/if}
    {/each}

    <div class="table scroll">
      <div class="trow head">
        <span>{t('runs.colEnd')}</span><span>{t('runs.colTime')}</span><span>{t('runs.colLevel')}</span><span>{t('runs.boon')}</span><span>{t('runs.bane')}</span><span>{t('runs.colEssence')}</span><span>{t('runs.colRerolls')}</span><span class="num">{resName(MAIN)}</span><span class="num">{t('runs.colRate')}</span><span>{t('runs.colNotes')}</span>
      </div>
      {#each shown as r (r.id)}
        {@const n = noteOf(r)}
        <div class="trow" class:open={open === r.id}>
          <button class="when" title={t('runs.detailTitle')} onclick={() => (open = open === r.id ? null : r.id)}>
            {clock(r.end.time)}{#if r.fortune}<i class="fortune" title={t('runs.fortune')}>F</i>{/if}
          </button>
          <span class="dim">{minutes(r)}</span>
          <input class="field" type="number" placeholder={levelAt(r.end.time) ? String(levelAt(r.end.time)) : ''} value={n.rupture ?? ''} onchange={(e) => setNote(r, { rupture: num(e.currentTarget.value) })} />
          <select class="field" value={n.boon ?? ''} onchange={(e) => setNote(r, { boon: e.currentTarget.value || undefined })}>
            <option value="">—</option>
            {#each boons as f (f.tag)}<option value={f.tag}>{f.description}</option>{/each}
          </select>
          <select class="field" value={n.bane ?? ''} onchange={(e) => setNote(r, { bane: e.currentTarget.value || undefined })}>
            <option value="">—</option>
            {#each banes as f (f.tag)}<option value={f.tag}>{f.description}</option>{/each}
          </select>
          <input class="field" type="number" value={n.essence ?? ''} onchange={(e) => setNote(r, { essence: num(e.currentTarget.value) })} />
          <input class="field" type="number" placeholder={r.fortune ? String(r.rerolls) : ''} title={t('runs.rerollsTitle', { n: r.rerolls })} value={n.rerolls ?? ''} onchange={(e) => setNote(r, { rerolls: num(e.currentTarget.value) })} />
          <span class="num gain">{signed(gainOf(r, MAIN))}</span>
          <span class="num dim">{shortNumber(hours(r) > 0 ? gainOf(r, MAIN) / hours(r) : 0)}</span>
          <input class="field" type="text" value={n.notes ?? ''} onchange={(e) => setNote(r, { notes: e.currentTarget.value || undefined })} />
        </div>
        {#if open === r.id}
          <div class="detail">
            <div class="extra">
              <label>{t('runs.colContainers')} <input class="field" type="number" value={n.containers ?? ''} onchange={(e) => setNote(r, { containers: num(e.currentTarget.value) })} /></label>
              <label>{t('runs.colElites')} <input class="field" type="number" value={n.elites ?? ''} onchange={(e) => setNote(r, { elites: num(e.currentTarget.value) })} /></label>
              <label>{t('runs.colMobs')} <input class="field" type="number" value={n.mobs ?? ''} onchange={(e) => setNote(r, { mobs: num(e.currentTarget.value) })} /></label>
              <span class="dim">{t('runs.map', { zone: r.zone })} · {t('runs.xp', { n: signed(r.end.experience - r.base.experience) })}</span>
            </div>
            <div class="gains">
              {#each resourceGains(r) as [k, g] (k)}
                <span class:neg={g < 0}>{#if resourceInfo.get(k)?.icon}<img src={asset(resourceInfo.get(k)?.icon)} alt="" />{/if}{signed(g)} {resName(k)}</span>
              {/each}
            </div>
          </div>
        {/if}
      {/each}
    </div>
  {/if}
</div>

<style>
  .runs {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
    font-family: var(--font-ui);
    font-size: 13px;
  }
  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 14px;
  }
  .grow {
    flex: 1;
  }
  .pick {
    display: flex;
    gap: 6px;
    align-items: center;
    color: var(--text-dim);
  }
  .pick select {
    max-width: 240px;
  }
  .lvl {
    width: 80px;
  }
  .dim {
    color: var(--text-dim);
  }
  .error {
    margin: 0;
    padding: 8px;
    white-space: pre-wrap;
    color: var(--bad);
    background: var(--bg-2);
  }
  .tiles {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }
  .tile {
    flex: 1 1 160px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 10px 12px;
    background: var(--panel-2);
    border: 1px solid var(--line);
  }
  .tile.main {
    border-color: var(--gold-dim);
  }
  .tile .k {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-dim);
  }
  .tile img,
  .card img,
  .gains img {
    width: 18px;
    height: 18px;
  }
  .tile .v {
    font-size: 22px;
    font-variant-numeric: tabular-nums;
  }
  .tile .d {
    font-size: 12px;
    color: var(--text-dim);
  }
  .cards {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 10px;
  }
  .card {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 8px;
    background: var(--panel-2);
    border: 1px solid var(--line);
  }
  .card b {
    color: var(--gold-bright);
    font-weight: 400;
  }
  .card em {
    font-style: normal;
    color: var(--text-dim);
  }
  .table {
    flex: 1;
    min-height: 0;
  }
  .trow {
    display: grid;
    grid-template-columns: 130px 52px 72px minmax(120px, 1fr) minmax(120px, 1fr) 76px 60px 84px 76px minmax(120px, 1fr);
    gap: 8px;
    padding: 4px 6px;
    border-bottom: 1px solid #241f17;
    align-items: center;
  }
  .trow.head {
    position: sticky;
    top: 0;
    z-index: 1;
    background: var(--panel);
    color: var(--text-faint);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .trow.open {
    background: rgba(201, 162, 74, 0.08);
  }
  .trow .field {
    width: 100%;
    min-width: 0;
  }
  .when {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0;
    border: none;
    background: none;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .fortune {
    padding: 0 4px;
    border: 1px solid var(--gold-dim);
    color: var(--gold-bright);
    font-size: 10px;
    font-style: normal;
  }
  .num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .gain {
    color: var(--good);
  }
  .neg {
    color: var(--bad);
  }
  .detail {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 8px 10px 12px;
    border-bottom: 1px solid #241f17;
    background: rgba(201, 162, 74, 0.04);
  }
  .extra {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 16px;
    color: var(--text-dim);
  }
  .extra input {
    width: 90px;
  }
  .gains {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 14px;
    color: var(--text-dim);
  }
  .gains span {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
</style>
