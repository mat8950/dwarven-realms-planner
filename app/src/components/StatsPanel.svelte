<script lang="ts">
  import { formatDelta, formatValue } from '../lib/format'
  import { asset } from '../lib/game'
  import { onDestroy } from 'svelte'
  import { debug } from '../lib/debug.svelte'
  import { raceName, stanceName as stanceLabel, t } from '../lib/i18n.svelte'
  import { planner } from '../lib/store.svelte'
  import type { Objective } from '../lib/types'
  import DpsView from './DpsView.svelte'
  import ImpactBadge from './ImpactBadge.svelte'

  const CR = 'EasyRPG.Attributes.Base.CombatRating'

  let tab = $state<'stats' | 'caps' | 'dps'>('stats')
  let hovered = $state<string | null>(null)
  // Seconds the running stat analysis has taken (a slow one points to the debug panel).
  let now = $state(Date.now())
  const tick = setInterval(() => (now = Date.now()), 1000)
  onDestroy(() => clearInterval(tick))
  const elapsed = $derived(planner.analyzing ? Math.max(0, Math.floor((now - planner.analyzeStarted) / 1000)) : 0)
  let showAllAbilities = $state(false)

  const value = (tag: string, s = planner.stats) => (tag === CR ? s?.combat_rating : s?.attributes[tag]) ?? 0
  const info = (tag: string) => planner.catalog?.attributes[tag]
  const label = (tag: string) => (tag === CR ? t('objective.cr') : info(tag)?.name || tag.split('.').pop())
  const fmtInfo = (tag: string) => (tag === CR ? { value_type: 1, digits: 2, name: '', description: '', icon: null } : info(tag))
  const icon = (tag: string) => info(tag.replace('.Display', ''))?.icon ?? info(tag)?.icon

  /** Panel rows like the game: collapsed when zero, optional filter on stats useless for the objective. */
  const rows = $derived.by(() => {
    const out: ({ kind: 'sep' } | { kind: 'row'; tag: string; sign: boolean })[] = []
    for (const e of planner.catalog?.panel ?? []) {
      if (e.kind === 'sep') {
        if (out.length && out[out.length - 1].kind !== 'sep') out.push({ kind: 'sep' })
      } else if (!(e.collapse_if_zero && value(e.tag) === 0 && value(e.tag, planner.baseline) === 0)) {
        if (planner.hideUseless && e.tag !== CR && planner.useless(e.tag)) continue
        out.push({ kind: 'row', tag: e.tag, sign: e.always_sign })
      }
    }
    if (out.at(-1)?.kind === 'sep') out.pop()
    return out
  })

  /** Stats of the panel that move the objective most, shown first. */
  const keyStats = $derived.by(() => {
    const tags = new Set((planner.catalog?.panel ?? []).flatMap((e) => (e.kind === 'row' ? [e.tag] : [])))
    return [...tags]
      .filter((t) => planner.sourceTags.has(t))
      .map((t) => [t, planner.impact(t) ?? 0] as const)
      .filter(([, v]) => v > 1e-7)
      .sort((a, b) => b[1] - a[1])
      .slice(0, 8)
      .map(([t]) => t)
  })

  const stanceName = $derived.by(() => {
    const n = planner.catalog?.stances.find(([v]) => v === planner.stats?.stance)?.[1] ?? ''
    return stanceLabel(n)
  })
  const attacks = $derived(
    (planner.catalog?.attacks ?? []).map((a, index) => ({ a, index })).filter(({ a }) => a.stance === planner.stats?.stance),
  )
  /** Offhand abilities granted by equipped items (an item carries one of its attributes). */
  const equippedAbilities = $derived.by(() => {
    const tags = new Set([...(planner.character?.items ?? []).map((_, i) => i), 'weapon' as const].flatMap((s) => planner.effAttrs(s).map((a) => a.tag)))
    return new Set(
      (planner.catalog?.abilities ?? []).flatMap((ab, i) => ([...tags].some((t) => t.startsWith(ab.tag + '.') || t === ab.damage_tag || t === ab.bonus_tag) ? [i] : [])),
    )
  })
  const abilities = $derived(
    (planner.catalog?.abilities ?? [])
      .map((a, index) => ({ a, index, equipped: equippedAbilities.has(index) }))
      .sort((x, y) => Number(y.equipped) - Number(x.equipped) || (planner.stats?.abilities[y.index] ?? 0) - (planner.stats?.abilities[x.index] ?? 0)),
  )

  const big = (n: number) => new Intl.NumberFormat('en-US', { maximumFractionDigits: 0 }).format(n)
  const pct = (a: number, b: number) => (b ? (a / b - 1) * 100 : 0)
  const isObjective = (o: Objective) => JSON.stringify(o) === JSON.stringify(planner.objective)
  const setObjective = (o: Objective) => (planner.objective = isObjective(o) ? { kind: 'cr' } : o)
</script>

<section class="panel frame">
  <header>
    <div class="tabs">
      <button class="tab title" class:active={tab === 'stats'} onclick={() => (tab = 'stats')}>{t('tab.stats')}</button>
      <button class="tab title" class:active={tab === 'caps'} onclick={() => (tab = 'caps')}>{t('tab.caps')}</button>
      <button class="tab title" class:active={tab === 'dps'} onclick={() => (tab = 'dps')}>{t('tab.dps')}</button>
    </div>
    {#if planner.character}
      <div class="who">
        <div class="name title">{planner.character.name}</div>
        <div class="lvl label">{t('stats.level', { n: planner.character.level, race: raceName(planner.character.race), stance: stanceName })}</div>
      </div>
    {/if}
    <hr class="gold-rule" />
  </header>

  {#if tab === 'stats'}
    <div class="rows scroll">
      {#if planner.character}
        <div class="key">
          <div class="key-title label" title={t('stats.keysTitle')}>
            {t('stats.keys', { obj: planner.objectiveLabel() })}
            {#if planner.analyzing}<span class="spin" title={t('stats.keysLoading')}>◌</span>{/if}
          </div>
          {#if planner.analyzeError}
            <button class="kmsg err" title={planner.analyzeError} onclick={() => (debug.open = true)}>{t('stats.keysError')}</button>
          {/if}
          {#each keyStats as t (t)}
            <div class="krow">
              {#if icon(t)}<img src={asset(icon(t))} alt="" />{:else}<span class="noicon"></span>{/if}
              <span class="lbl">{label(t)}</span>
              <ImpactBadge tag={t} />
            </div>
          {:else}
            {#if planner.analyzing || (!planner.analyzeError && !Object.keys(planner.impacts).length)}
              <p class="kmsg">
                {t('stats.keysLoading')}
                {#if elapsed >= 5}· {t('stats.keysSlow', { s: elapsed })} <button class="link" onclick={() => (debug.open = true)}>{t('stats.keysDebug')}</button>{/if}
              </p>
              {#each [0, 1, 2] as k (k)}<div class="krow ghost"><span class="noicon"></span><span class="bar"></span></div>{/each}
            {:else if !planner.analyzeError}
              <p class="kmsg">{t('stats.keysNone')}</p>
            {/if}
          {/each}
        </div>
        <hr class="gold-rule sep" />
      {/if}
      {#each rows as r, i (r.kind === 'sep' ? `sep-${i}` : r.tag)}
        {#if r.kind === 'sep'}
          <hr class="gold-rule sep" />
        {:else}
          {@const v = value(r.tag)}
          {@const d = v - value(r.tag, planner.baseline)}
          <div class="row" class:dim={planner.useless(r.tag)} role="listitem" onmouseenter={() => (hovered = r.tag)} onmouseleave={() => (hovered = null)}>
            {#if icon(r.tag)}<img src={asset(icon(r.tag))} alt="" />{:else}<span class="noicon"></span>{/if}
            <span class="lbl">{label(r.tag)}{#if r.tag !== CR && !planner.sourceTags.has(r.tag) && (planner.impact(r.tag) ?? 0) > 1e-7}<em class="maxed" title={t('stats.maxedTitle')}>{t('stats.maxed')}</em>{/if}</span>
            <span class="val">{formatValue(v, fmtInfo(r.tag), r.sign && r.tag !== CR)}</span>
            <span class="delta" class:good={d > 0} class:bad={d < 0}>{planner.baseline ? formatDelta(d, fmtInfo(r.tag)) : ''}</span>
            {#if hovered === r.tag && info(r.tag)?.description}
              <div class="desc parchment">{info(r.tag)?.description}</div>
            {/if}
          </div>
        {/if}
      {/each}
    </div>
  {:else if tab === 'dps'}
    <DpsView />
  {:else}
    <div class="rows scroll">
      <p class="note">{t('caps.note')}</p>
      <h3 class="label">{t('caps.attacks', { stance: stanceName })}</h3>
      {#each attacks as { a, index } (a.row)}
        {@const o = { kind: 'attack', index } as Objective}
        {@const v = planner.stats?.attacks[index] ?? 0}
        {@const b = planner.baseline?.attacks[index] ?? 0}
        <button class="cap" class:sel={isObjective(o)} onclick={() => setObjective(o)} title={a.description}>
          {#if a.icon}<img src={asset(a.icon)} alt="" />{:else}<span class="noicon"></span>{/if}
          <span class="cname">{a.name}<small>{a.kind === 0 ? t('common.primary') : t('common.secondary')} · ×{a.multiplier}</small></span>
          <span class="val">{big(v)}</span>
          <span class="delta" class:good={v > b} class:bad={v < b}>{Math.abs(pct(v, b)) > 0.005 ? `${pct(v, b) > 0 ? '+' : ''}${pct(v, b).toFixed(2)}%` : ''}</span>
        </button>
      {/each}
      <p class="fine">{t('caps.attackFine')}</p>

      <h3 class="label">{t('caps.offhands')}</h3>
      {#each abilities.filter((x) => x.equipped || showAllAbilities) as { a, index, equipped } (a.row)}
        {@const o = { kind: 'ability', index } as Objective}
        {@const v = planner.stats?.abilities[index] ?? 0}
        {@const b = planner.baseline?.abilities[index] ?? 0}
        <button class="cap" class:sel={isObjective(o)} class:dimmed={!equipped} onclick={() => setObjective(o)} title={a.description}>
          {#if a.icon}<img src={asset(a.icon)} alt="" />{:else}<span class="noicon"></span>{/if}
          <span class="cname">{a.name}<small>{a.element || t('common.physical')} · {a.affinities.map((x) => x.split('.').pop()).join(', ')}</small></span>
          <span class="val">{big(v)}</span>
          <span class="delta" class:good={v > b} class:bad={v < b}>{Math.abs(pct(v, b)) > 0.005 ? `${pct(v, b) > 0 ? '+' : ''}${pct(v, b).toFixed(2)}%` : ''}</span>
        </button>
      {/each}
      <button class="btn ghost more" onclick={() => (showAllAbilities = !showAllAbilities)}>{showAllAbilities ? t('caps.hideOthers') : t('caps.showOthers', { n: abilities.length - equippedAbilities.size })}</button>
      <p class="fine">{t('caps.offhandFine')}</p>
    </div>
  {/if}
</section>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 14px 16px 10px;
  }
  .tabs {
    display: flex;
    justify-content: center;
    gap: 6px;
    padding-bottom: 6px;
  }
  .tab {
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    padding: 0 16px 4px;
    font-size: 18px;
    cursor: pointer;
    opacity: 0.55;
  }
  .tab.active {
    opacity: 1;
    border-bottom-color: var(--gold);
  }
  .who {
    padding: 8px 4px 0;
  }
  .name {
    font-size: 22px;
  }
  .lvl {
    font-size: 14px;
    margin-top: 2px;
  }
  .rows {
    flex: 1;
    min-height: 0;
    padding-right: 6px;
  }
  .key {
    padding: 4px 6px 2px;
    background: linear-gradient(180deg, rgba(127, 211, 107, 0.06), transparent);
    border-left: 2px solid rgba(127, 211, 107, 0.45);
  }
  .key-title {
    font-size: 11px;
    margin-bottom: 4px;
  }
  .krow {
    display: grid;
    grid-template-columns: 20px minmax(0, 1fr) auto;
    gap: 8px;
    align-items: center;
    font-size: 15px;
    padding: 1px 0;
  }
  .krow img,
  .krow .noicon {
    width: 18px;
    height: 18px;
  }
  .krow.ghost .bar {
    height: 10px;
    border-radius: 5px;
    background: linear-gradient(90deg, rgba(201, 162, 74, 0.08), rgba(201, 162, 74, 0.2), rgba(201, 162, 74, 0.08));
    background-size: 200% 100%;
    animation: shimmer 1.4s linear infinite;
  }
  @keyframes shimmer {
    to {
      background-position: -200% 0;
    }
  }
  .kmsg {
    margin: 2px 0 4px;
    font-family: var(--font-ui);
    font-size: 12.5px;
    color: var(--text-dim);
  }
  .kmsg.err {
    display: block;
    width: 100%;
    padding: 2px 0;
    border: none;
    background: none;
    color: var(--bad);
    text-align: left;
    cursor: pointer;
  }
  .link {
    padding: 0;
    border: none;
    background: none;
    color: var(--gold);
    font: inherit;
    text-decoration: underline;
    cursor: pointer;
  }
  .spin {
    display: inline-block;
    margin-left: 6px;
    color: var(--gold);
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .row {
    position: relative;
    display: grid;
    grid-template-columns: 22px minmax(0, 1fr) auto auto;
    align-items: center;
    column-gap: 8px;
    padding: 3px 4px;
    font-size: 16px;
    border-radius: 2px;
  }
  .row:hover {
    background: rgba(201, 162, 74, 0.07);
  }
  .row.dim {
    opacity: 0.4;
  }
  img,
  .noicon {
    width: 22px;
    height: 22px;
    border-radius: 50%;
  }
  .lbl {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .val {
    font-variant-numeric: tabular-nums;
    text-align: right;
  }
  .delta {
    font-family: var(--font-ui);
    font-size: 12px;
    min-width: 0;
    text-align: right;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .sep {
    margin: 8px 0;
  }
  .desc {
    position: absolute;
    right: calc(100% + 12px);
    top: -4px;
    width: 260px;
    padding: 8px 10px;
    font-size: 13px;
    z-index: 40;
    pointer-events: none;
  }
  h3 {
    font-size: 13px;
    margin: 14px 0 6px;
  }
  .note,
  .fine {
    color: var(--text-dim);
    font-size: 13px;
    margin: 4px 0;
  }
  .fine {
    font-size: 11.5px;
    color: var(--text-faint);
  }
  .cap {
    display: grid;
    grid-template-columns: 30px minmax(0, 1fr) auto 62px;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 5px 6px;
    background: none;
    border: 1px solid transparent;
    border-radius: 2px;
    text-align: left;
    cursor: pointer;
  }
  .cap img,
  .cap .noicon {
    width: 30px;
    height: 30px;
    border-radius: 3px;
  }
  .cap:hover {
    background: rgba(201, 162, 74, 0.07);
  }
  .cap.sel {
    border-color: var(--gold);
    background: rgba(201, 162, 74, 0.12);
  }
  .cap.dimmed {
    opacity: 0.6;
  }
  .cname {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .cname small {
    font-family: var(--font-ui);
    color: var(--text-dim);
    font-size: 12px;
  }
  .more {
    width: 100%;
    margin-top: 4px;
  }
  .maxed {
    font-style: normal;
    font-family: var(--font-ui);
    font-size: 10.5px;
    color: var(--text-faint);
    border: 1px solid var(--line);
    border-radius: 7px;
    padding: 0 5px;
    margin-left: 6px;
  }
</style>
