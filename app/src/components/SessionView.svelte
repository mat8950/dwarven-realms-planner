<script lang="ts">
  // Play session: what the character earned since a starting point, read from the save each time
  // the game writes it (the backend keeps one journal entry per save change). The journal splits
  // into play sessions at each long break; the latest session is the default starting point.
  import { onDestroy } from 'svelte'
  import { api } from '../lib/api'
  import { shortNumber } from '../lib/format'
  import { asset } from '../lib/game'
  import { fmtDate, fmtNumber, i18n, t } from '../lib/i18n.svelte'
  import { planner } from '../lib/store.svelte'
  import type { SessionSnapshot } from '../lib/types'
  import RunsView from './RunsView.svelte'

  /** A break this long starts a new play session. */
  const BREAK = 20 * 60
  /** The resource the farming players follow; shown first. */
  const MAIN = 'ElvenFragments'

  let journal = $state<SessionSnapshot[]>([])
  let error = $state<string | null>(null)
  let startTime = $state<number | null>(null)
  let tab = $state<'session' | 'runs'>('session')

  async function refresh() {
    const save = planner.character?.save
    if (!save) return
    try {
      journal = await api.session(save)
      error = null
    } catch (e) {
      error = String(e)
    }
  }
  void refresh()
  // The game writes the save now and then: look for a new version every 5 s.
  const timer = setInterval(() => void refresh(), 5000)
  onDestroy(() => clearInterval(timer))

  /** Index of the first entry of each play session, newest session first. */
  const sessions = $derived.by(() => {
    const starts: number[] = []
    journal.forEach((s, i) => {
      if (i === 0 || s.time - journal[i - 1].time > BREAK) starts.push(i)
    })
    return starts.reverse()
  })
  const startIndex = $derived.by(() => {
    if (startTime !== null) {
      const i = journal.findIndex((s) => s.time >= startTime!)
      if (i >= 0) return i
    }
    return sessions[0] ?? 0
  })
  const start = $derived(journal[startIndex])
  const last = $derived(journal[journal.length - 1])
  const hours = $derived(start && last ? (last.time - start.time) / 3600 : 0)

  const resourceInfo = $derived(new Map((planner.catalog?.resources ?? []).map((r) => [r.tag, r])))
  const humanize = (k: string) => k.replace(/^KeyMaterial\./, '').replace(/([a-z])([A-Z])/g, '$1 $2').replace(/[._]/g, ' ')
  const resName = (k: string) => resourceInfo.get(k)?.name ?? humanize(k)

  type Group = 'resources' | 'counters' | 'skills' | 'character'
  interface Gain {
    group: Group
    key: string
    name: string
    icon: string | null
    now: number
    gain: number
  }
  function gains(a: SessionSnapshot, b: SessionSnapshot): Gain[] {
    const out: Gain[] = []
    const one = (group: Group, key: string, name: string, x: number, y: number, icon: string | null = null) => {
      if (x !== y) out.push({ group, key, name, icon, now: y, gain: y - x })
    }
    one('character', 'level', t('session.level'), a.level, b.level)
    one('character', 'experience', t('session.xp'), a.experience, b.experience)
    one('character', 'deaths', t('session.deaths'), a.deaths, b.deaths)
    one('character', 'crafting', t('session.crafting'), a.crafting_level, b.crafting_level)
    one('character', 'ruptures', t('session.ruptures'), a.ruptures, b.ruptures)
    for (const k of new Set([...Object.keys(a.resources), ...Object.keys(b.resources)]))
      one('resources', k, resName(k), a.resources[k] ?? 0, b.resources[k] ?? 0, resourceInfo.get(k)?.icon ?? null)
    for (const k of new Set([...Object.keys(a.counters), ...Object.keys(b.counters)])) one('counters', k, humanize(k), a.counters[k] ?? 0, b.counters[k] ?? 0)
    for (const k of new Set([...Object.keys(a.skills), ...Object.keys(b.skills)])) one('skills', k, humanize(k), a.skills[k] ?? 0, b.skills[k] ?? 0)
    return out
  }
  const ORDER: Group[] = ['resources', 'counters', 'character', 'skills']
  const total = $derived.by(() => {
    if (!start || !last || start === last) return []
    return gains(start, last).sort((x, y) => ORDER.indexOf(x.group) - ORDER.indexOf(y.group) || Number(y.key === MAIN) - Number(x.key === MAIN) || x.name.localeCompare(y.name))
  })
  /** Each save since the start, newest first, with what it brought. */
  const steps = $derived(
    journal
      .slice(startIndex + 1)
      .map((s, k) => {
        const prev = journal[startIndex + k]
        const g = gains(prev, s).filter((x) => x.group !== 'skills')
        return { s, prev, main: g.find((x) => x.key === MAIN)?.gain ?? 0, other: g.filter((x) => x.key !== MAIN && x.key !== 'experience') }
      })
      .reverse(),
  )

  const res = (s: SessionSnapshot | undefined, k: string) => s?.resources[k] ?? 0
  const signed = (v: number) => (v > 0 ? '+' : '') + shortNumber(v)
  const perHour = (v: number) => (hours > 0.02 ? `${signed(v / hours)}${t('session.perHour')}` : '')
  function duration(seconds: number) {
    const m = Math.max(0, Math.round(seconds / 60))
    return m >= 60 ? `${Math.floor(m / 60)} h ${String(m % 60).padStart(2, '0')}` : `${m} min`
  }
  const clock = (unix: number) => new Intl.DateTimeFormat(i18n.language.intl, { timeStyle: 'short' }).format(new Date(unix * 1000))
  const zoneName = (z: string) => t(`session.zone.${z}`, {}, humanize(z))
  const groupName = (g: Group) => t(`session.group.${g}`)

  function startNow() {
    if (last) startTime = last.time
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && (planner.sessionOpen = false)} />

<div class="overlay">
  <section class="panel frame">
    <header>
      <div>
        <span class="title">{t('session.title')}</span>
        {#if planner.character}<span class="who">{planner.character.name}</span>{/if}
        <span class="tabs">
          <button class="tab" class:on={tab === 'session'} onclick={() => (tab = 'session')}>{t('session.tabSession')}</button>
          <button class="tab" class:on={tab === 'runs'} onclick={() => (tab = 'runs')}>{t('session.tabRuns')}</button>
        </span>
      </div>
      <div class="actions">
        {#if journal.length && tab === 'session'}
          <label class="pick">
            {t('session.since')}
            <select class="field" value={start?.time} onchange={(e) => (startTime = Number(e.currentTarget.value))}>
              {#each sessions as i (i)}
                <option value={journal[i].time}>{t('session.sessionOf', { date: fmtDate(journal[i].time) })}</option>
              {/each}
              {#if startTime !== null && !sessions.includes(startIndex)}<option value={start?.time}>{fmtDate(start.time)}</option>{/if}
            </select>
          </label>
          <button class="btn" title={t('session.startNowTitle')} onclick={startNow}>{t('session.startNow')}</button>
        {/if}
        <button class="btn ghost" title={t('common.close')} onclick={() => (planner.sessionOpen = false)}>✕</button>
      </div>
    </header>
    <p class="hint">{t('session.hint')}</p>
    {#if error}<pre class="error">{error}</pre>{/if}

    {#if tab === 'runs'}
      <RunsView {journal} />
    {:else if !journal.length}
      <p class="empty">{t('session.empty')}</p>
    {:else}
      <div class="tiles">
        <div class="tile">
          <span class="k">{t('session.duration')}</span>
          <span class="v">{duration(last.time - start.time)}</span>
          <span class="d">{t('session.saves', { n: journal.length - startIndex })} · {t('session.lastSave', { time: clock(last.time) })}</span>
        </div>
        <div class="tile main">
          <span class="k">
            {#if resourceInfo.get(MAIN)?.icon}<img src={asset(resourceInfo.get(MAIN)?.icon)} alt="" />{/if}{resName(MAIN)}
          </span>
          <span class="v">{signed(res(last, MAIN) - res(start, MAIN))}</span>
          <span class="d">{perHour(res(last, MAIN) - res(start, MAIN))} · {t('session.total', { n: shortNumber(res(last, MAIN)) })}</span>
        </div>
        <div class="tile">
          <span class="k">{t('session.xp')}</span>
          <span class="v">{signed(last.experience - start.experience)}</span>
          <span class="d">{perHour(last.experience - start.experience)}</span>
        </div>
        <div class="tile">
          <span class="k">{t('session.level')}</span>
          <span class="v">{last.level}</span>
          <span class="d">{last.level !== start.level ? signed(last.level - start.level) : ''}</span>
        </div>
        <div class="tile">
          <span class="k">{t('session.deaths')}</span>
          <span class="v">{fmtNumber(last.deaths - start.deaths)}</span>
          <span class="d">{zoneName(last.zone)}</span>
        </div>
      </div>

      <div class="cols">
        <div class="col scroll">
          <h3>{t('session.gains')}</h3>
          {#if !total.length}
            <p class="empty">{t('session.noGain')}</p>
          {:else}
            {#each ORDER as g (g)}
              {@const rows = total.filter((x) => x.group === g)}
              {#if rows.length}
                <div class="group">{groupName(g)}</div>
                {#each rows as r (r.group + r.key)}
                  <div class="row" class:main={r.key === MAIN}>
                    <span class="name">{#if r.icon}<img src={asset(r.icon)} alt="" />{/if}{r.name}</span>
                    <span class="num">{shortNumber(r.now)}</span>
                    <span class="num gain" class:neg={r.gain < 0}>{signed(r.gain)}</span>
                    <span class="num rate">{perHour(r.gain)}</span>
                  </div>
                {/each}
              {/if}
            {/each}
          {/if}
        </div>

        <div class="col scroll">
          <h3>{t('session.steps')}</h3>
          {#if !steps.length}
            <p class="empty">{t('session.noStep')}</p>
          {/if}
          {#each steps as st (st.s.time)}
            <div class="step">
              <div class="when">
                <span>{clock(st.s.time)}</span>
                <span class="dim">{duration(st.s.time - st.prev.time)} · {zoneName(st.s.zone)}</span>
              </div>
              <div class="what">
                {#if st.main}<span class="mainchip">{signed(st.main)} {resName(MAIN)}</span>{/if}
                {#each st.other.slice(0, 6) as o (o.group + o.key)}<span class:neg={o.gain < 0}>{signed(o.gain)} {o.name}</span>{/each}
                {#if st.other.length > 6}<span class="dim">{t('session.more', { n: st.other.length - 6 })}</span>{/if}
                {#if !st.main && !st.other.length}<span class="dim">{t('session.onlyXp')}</span>{/if}
              </div>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </section>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 600;
    display: flex;
    padding: 24px;
    background: rgba(8, 7, 5, 0.86);
  }
  .panel {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px 18px;
    background: var(--panel);
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }
  header .title {
    font-size: 20px;
  }
  .who {
    margin-left: 12px;
    font-family: var(--font-title);
    color: var(--gold-bright);
  }
  .tabs {
    display: inline-flex;
    gap: 4px;
    margin-left: 18px;
    vertical-align: middle;
  }
  .tab {
    padding: 3px 12px;
    border: 1px solid var(--line);
    background: none;
    color: var(--text-dim);
    font-family: var(--font-ui);
    font-size: 13px;
    cursor: pointer;
  }
  .tab.on {
    border-color: var(--gold-dim);
    color: var(--gold-bright);
  }
  .actions {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }
  .pick {
    display: flex;
    gap: 6px;
    align-items: center;
    font-family: var(--font-ui);
    font-size: 13px;
    color: var(--text-dim);
  }
  .hint,
  .empty {
    margin: 0;
    font-family: var(--font-ui);
    font-size: 13px;
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
    font-family: var(--font-ui);
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
  .tile .k img {
    width: 18px;
    height: 18px;
  }
  .tile .v {
    font-size: 22px;
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }
  .tile .d {
    min-height: 1.2em;
    font-size: 12px;
    color: var(--text-dim);
  }
  .cols {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1.2fr);
    gap: 14px;
  }
  .col {
    min-height: 0;
    font-family: var(--font-ui);
    font-size: 13px;
  }
  h3 {
    position: sticky;
    top: 0;
    margin: 0 0 4px;
    padding: 2px 0;
    background: var(--panel);
    font-family: var(--font-label);
    font-size: 14px;
    color: var(--gold);
  }
  .group {
    margin-top: 8px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-faint);
  }
  .row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 72px 72px 84px;
    gap: 8px;
    padding: 3px 4px;
    border-bottom: 1px solid #241f17;
    align-items: center;
  }
  .row.main {
    background: rgba(201, 162, 74, 0.08);
  }
  .name {
    display: flex;
    align-items: center;
    gap: 6px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .name img {
    width: 18px;
    height: 18px;
    flex: none;
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
  .rate {
    color: var(--text-dim);
  }
  .step {
    display: grid;
    grid-template-columns: 150px minmax(0, 1fr);
    gap: 10px;
    padding: 5px 4px;
    border-bottom: 1px solid #241f17;
  }
  .when {
    display: flex;
    flex-direction: column;
  }
  .dim {
    color: var(--text-dim);
    font-size: 12px;
  }
  .what {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    color: var(--text-dim);
  }
  .mainchip {
    color: var(--gold-bright);
  }
</style>
