<script lang="ts">
  import { shortNumber } from '../lib/format'
  import { changes, type Kind } from '../lib/history'
  import { fmtDate, fmtNumber, t } from '../lib/i18n.svelte'
  import { planner } from '../lib/store.svelte'
  import type { Snapshot } from '../lib/types'

  // One measure at a time (one axis): the curve picker chooses which. Each gear set (damage,
  // farm) is its own series, so swapping sets does not break the curve.
  type Metric = 'dps_boss' | 'dps' | 'combat_rating' | 'level' | 'tree'
  const METRICS: { key: Metric; label: string }[] = $derived([
    { key: 'dps_boss', label: t('history.metricDpsBoss') },
    { key: 'dps', label: t('history.metricDps') },
    { key: 'combat_rating', label: t('history.metricCr') },
    { key: 'level', label: t('history.metricLevel') },
    { key: 'tree', label: t('history.metricTree') },
  ])
  let metric = $state<Metric>('dps_boss')
  let show = $state<Kind | 'all'>('dps')
  const value = (s: Snapshot, m: Metric) => (m === 'tree' ? s.tree.length : m === 'level' ? s.level : s[m])
  const fmt = (v: number, m: Metric) => (m === 'level' || m === 'tree' ? fmtNumber(v) : shortNumber(v))
  const COLOR: Record<Kind, string> = { dps: '#b8892c', farm: '#5b8fd0' }
  const kindName = (k: Kind) => t(k === 'farm' ? 'history.kindFarm' : 'history.kindDps')

  const points = $derived(planner.history.map((s, i) => ({ s, i, kind: planner.kindOf(s) })))
  /** Previous point of the same set. */
  const prevOf = $derived.by(() => {
    const last: Partial<Record<Kind, Snapshot>> = {}
    return points.map((p) => {
      const prev = last[p.kind]
      last[p.kind] = p.s
      return prev
    })
  })
  const rows = $derived(points.map((p) => ({ ...p, changes: changes(planner.catalog, prevOf[p.i], p.s, p.kind) })).reverse())
  const kinds = $derived<Kind[]>(show === 'all' ? ['dps', 'farm'] : [show])
  const shown = $derived(points.filter((p) => kinds.includes(p.kind)))
  const series = $derived(kinds.map((k) => ({ k, pts: shown.filter((p) => p.kind === k) })).filter((x) => x.pts.length))

  // Chart geometry.
  let width = $state(900)
  const H = 260
  const PAD = { l: 64, r: 96, t: 16, b: 28 }
  const xs = $derived(shown.map((p) => p.s.save_modified))
  const ys = $derived(shown.map((p) => value(p.s, metric)))
  const x0 = $derived(Math.min(...xs))
  const x1 = $derived(Math.max(...xs))
  /** Clean tick values around the data (no forced zero: a line shows change). */
  const ticks = $derived.by(() => {
    if (!ys.length) return [0, 1]
    let lo = Math.min(...ys)
    let hi = Math.max(...ys)
    if (hi === lo) [lo, hi] = [lo * 0.9, hi * 1.1 || 1]
    const raw = (hi - lo) / 4
    const mag = 10 ** Math.floor(Math.log10(raw))
    const step = [1, 2, 2.5, 5, 10].map((k) => k * mag).find((k) => k >= raw) ?? raw
    const out: number[] = []
    for (let v = Math.floor(lo / step) * step; v <= hi + step * 0.999; v += step) out.push(v)
    return out
  })
  const yMin = $derived(ticks[0])
  const yMax = $derived(ticks[ticks.length - 1])
  const px = (x: number) => PAD.l + (x1 > x0 ? ((x - x0) / (x1 - x0)) * (width - PAD.l - PAD.r) : (width - PAD.l - PAD.r) / 2)
  const py = (y: number) => PAD.t + (1 - (y - yMin) / (yMax - yMin || 1)) * (H - PAD.t - PAD.b)
  const path = (pts: { s: Snapshot }[]) => pts.map((p, i) => `${i ? 'L' : 'M'}${px(p.s.save_modified).toFixed(1)},${py(value(p.s, metric)).toFixed(1)}`).join(' ')
  const xTicks = $derived.by(() => {
    if (x1 <= x0) return xs.slice(0, 1)
    const n = Math.max(2, Math.min(6, Math.floor((width - PAD.l - PAD.r) / 140)))
    return Array.from({ length: n }, (_, i) => x0 + ((x1 - x0) * i) / (n - 1))
  })
  const day = (s: number) => new Intl.DateTimeFormat(undefined, { day: '2-digit', month: '2-digit' }).format(new Date(s * 1000))

  // Hover: nearest shown point by x (index into `points`).
  let hover = $state<number | null>(null)
  function move(e: PointerEvent) {
    const r = (e.currentTarget as SVGElement).getBoundingClientRect()
    const x = e.clientX - r.left
    let best: number | null = null
    let d = Infinity
    for (const p of shown) {
      const dx = Math.abs(px(p.s.save_modified) - x)
      if (dx < d) [best, d] = [p.i, dx]
    }
    hover = best
  }
  const delta = (i: number) => {
    const prev = prevOf[i]
    if (!prev) return ''
    const a = value(points[i].s, metric)
    const b = value(prev, metric)
    if (metric === 'level' || metric === 'tree') return a === b ? '' : `${a > b ? '+' : ''}${fmtNumber(a - b)}`
    if (!b) return ''
    const p = (a / b - 1) * 100
    return Math.abs(p) < 0.005 ? '' : `${p > 0 ? '+' : ''}${p.toFixed(1)}%`
  }

  let capturing = $state(false)
  async function captureNow() {
    capturing = true
    await planner.captureHistory()
    await planner.loadHistory()
    capturing = false
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && (planner.historyOpen = false)} />

<div class="overlay">
  <section class="panel frame">
    <header>
      <div>
        <span class="title">{t('history.title')}</span>
        {#if planner.character}<span class="who">{planner.character.name}</span>{/if}
        {#if points.length}<span class="span">{t('history.points', { n: points.length, from: fmtDate(points[0].s.save_modified), to: fmtDate(points[points.length - 1].s.save_modified) })}</span>{/if}
      </div>
      <div class="actions">
        <label class="pick">
          {t('history.show')}
          <select class="field" bind:value={show}>
            <option value="dps">{t('history.kindDps')}</option>
            <option value="farm">{t('history.kindFarm')}</option>
            <option value="all">{t('history.showAll')}</option>
          </select>
        </label>
        <label class="pick">
          {t('history.metric')}
          <select class="field" bind:value={metric}>
            {#each METRICS as m (m.key)}<option value={m.key}>{m.label}</option>{/each}
          </select>
        </label>
        <button class="btn" disabled={capturing} title={t('history.captureTitle')} onclick={captureNow}>{t('history.capture')}</button>
        <button class="btn ghost" title={t('common.close')} onclick={() => (planner.historyOpen = false)}>✕</button>
      </div>
    </header>

    {#if !points.length}
      <p class="empty">{t('history.empty')}</p>
    {:else}
      {#if series.length > 1}
        <div class="legend">
          {#each series as sr (sr.k)}<span><i style:background={COLOR[sr.k]}></i>{kindName(sr.k)}</span>{/each}
        </div>
      {/if}
      <div class="chart" bind:clientWidth={width}>
        {#if shown.length}
          <svg {width} height={H} role="img" aria-label={METRICS.find((m) => m.key === metric)?.label} onpointermove={move} onpointerleave={() => (hover = null)}>
            {#each ticks as v (v)}
              <line class="grid" x1={PAD.l} x2={width - PAD.r} y1={py(v)} y2={py(v)} />
              <text class="tick" x={PAD.l - 8} y={py(v) + 4} text-anchor="end">{fmt(v, metric)}</text>
            {/each}
            {#each xTicks as x (x)}
              <text class="tick" x={px(x)} y={H - 8} text-anchor="middle">{day(x)}</text>
            {/each}
            {#each series as sr (sr.k)}
              <path class="line" d={path(sr.pts)} style:stroke={COLOR[sr.k]} />
              {#each sr.pts as p (p.i)}
                <circle class="dot" cx={px(p.s.save_modified)} cy={py(value(p.s, metric))} r={hover === p.i ? 6 : 4} style:fill={COLOR[sr.k]} />
              {/each}
              {@const last = sr.pts[sr.pts.length - 1]}
              <text class="end" x={px(last.s.save_modified) + 10} y={py(value(last.s, metric)) + 4}>{fmt(value(last.s, metric), metric)}{series.length > 1 ? ` ${kindName(sr.k)}` : ''}</text>
            {/each}
            {#if hover !== null}
              <line class="cross" x1={px(points[hover].s.save_modified)} x2={px(points[hover].s.save_modified)} y1={PAD.t} y2={H - PAD.b} />
            {/if}
          </svg>
          {#if hover !== null}
            {@const p = points[hover]}
            <div class="tip" style:left="{Math.min(px(p.s.save_modified) + 14, width - 250)}px">
              <div class="tdate"><i style:background={COLOR[p.kind]}></i>{kindName(p.kind)} · {fmtDate(p.s.save_modified)} · {t('top.level', { n: p.s.level })}</div>
              <div class="tval">{fmt(value(p.s, metric), metric)} <em>{delta(hover)}</em></div>
              {#each changes(planner.catalog, prevOf[hover], p.s, p.kind).slice(0, 4) as c, k (k)}<div class="tchg">{c}</div>{/each}
            </div>
          {/if}
        {:else}
          <p class="empty">—</p>
        {/if}
      </div>

      <div class="table scroll">
        <div class="trow head">
          <span>{t('history.colDate')}</span><span>{t('history.show')}</span><span>{t('history.colLevel')}</span><span>{t('history.colCr')}</span><span>{t('history.colDps')}</span><span>{t('history.colDpsBoss')}</span><span>{t('history.colTree')}</span><span>{t('history.changes')}</span>
        </div>
        {#each rows as r (r.s.save_modified)}
          <div class="trow" class:hl={hover === r.i} class:other={!kinds.includes(r.kind)} role="listitem" onmouseenter={() => (hover = kinds.includes(r.kind) ? r.i : null)} onmouseleave={() => (hover = null)}>
            <span>{fmtDate(r.s.save_modified)}</span>
            <span><button class="kind" title={t('history.kindTitle')} style:border-color={COLOR[r.kind]} onclick={() => planner.setKind(r.s, r.kind === 'farm' ? 'dps' : 'farm')}>{kindName(r.kind)}</button></span>
            <span class="num">{r.s.level}</span>
            <span class="num">{shortNumber(r.s.combat_rating)}</span>
            <span class="num">{shortNumber(r.s.dps)}</span>
            <span class="num">{shortNumber(r.s.dps_boss)}</span>
            <span class="num">{r.s.tree.length}</span>
            <span class="chg">{#each r.changes as c, k (k)}<span>{c}</span>{/each}</span>
          </div>
        {/each}
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
  .span {
    margin-left: 12px;
    font-family: var(--font-ui);
    font-size: 13px;
    color: var(--text-dim);
  }
  .actions {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .pick {
    display: flex;
    gap: 6px;
    align-items: center;
    font-family: var(--font-ui);
    font-size: 13px;
    color: var(--text-dim);
  }
  .empty {
    font-family: var(--font-ui);
    color: var(--text-dim);
  }
  .legend {
    display: flex;
    gap: 16px;
    font-family: var(--font-ui);
    font-size: 12.5px;
    color: var(--text-dim);
  }
  .legend i,
  .tdate i {
    display: inline-block;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    margin-right: 6px;
    vertical-align: -1px;
  }
  .chart {
    position: relative;
    flex: none;
    min-height: 40px;
  }
  svg {
    display: block;
    touch-action: none;
  }
  .grid {
    stroke: #2a261d;
    stroke-width: 1;
  }
  .tick {
    fill: var(--text-faint);
    font-family: var(--font-ui);
    font-size: 11.5px;
  }
  .line {
    fill: none;
    stroke-width: 2;
    stroke-linejoin: round;
    stroke-linecap: round;
  }
  .dot {
    stroke: var(--panel);
    stroke-width: 2;
  }
  .end {
    fill: var(--text);
    font-family: var(--font-ui);
    font-size: 12.5px;
  }
  .cross {
    stroke: var(--text-faint);
    stroke-width: 1;
  }
  .tip {
    position: absolute;
    top: 8px;
    width: 240px;
    padding: 8px 10px;
    background: var(--panel-3);
    border: 1px solid var(--gold-dim);
    font-family: var(--font-ui);
    font-size: 12.5px;
    pointer-events: none;
  }
  .tdate {
    color: var(--text-dim);
  }
  .tval {
    font-size: 16px;
    color: var(--text);
  }
  .tval em {
    font-style: normal;
    font-size: 12.5px;
    color: var(--text-dim);
  }
  .tchg {
    color: var(--text-dim);
  }
  .table {
    flex: 1;
    min-height: 0;
    font-family: var(--font-ui);
    font-size: 13px;
  }
  .trow {
    display: grid;
    grid-template-columns: 130px 64px 50px 80px 80px 90px 60px minmax(0, 1fr);
    gap: 10px;
    padding: 5px 6px;
    border-bottom: 1px solid #241f17;
    align-items: start;
  }
  .trow.head {
    position: sticky;
    top: 0;
    background: var(--panel);
    color: var(--text-faint);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .trow.hl {
    background: rgba(201, 162, 74, 0.08);
  }
  .trow.other {
    opacity: 0.45;
  }
  .kind {
    padding: 0 8px;
    border: 1px solid;
    border-radius: 9px;
    background: none;
    color: var(--text);
    font-family: var(--font-ui);
    font-size: 12px;
    cursor: pointer;
  }
  .num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .chg {
    display: flex;
    flex-direction: column;
    color: var(--text-dim);
  }
</style>
