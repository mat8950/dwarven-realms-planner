<script lang="ts">
  import { shortNumber } from '../lib/format'
  import { asset } from '../lib/game'
  import { fmtNumber, stanceName as stanceLabel, t } from '../lib/i18n.svelte'
  import { planner } from '../lib/store.svelte'
  import type { DpsView } from '../lib/types'

  const c = $derived(planner.catalog)
  const s = $derived(planner.stats)
  const b = $derived(planner.baseline)
  const d = $derived(s?.dps)

  const stanceName = $derived.by(() => {
    const n = c?.stances.find(([v]) => v === s?.stance)?.[1] ?? ''
    return stanceLabel(n)
  })
  const attacks = $derived((c?.attacks ?? []).map((a, i) => ({ a, i })).filter(({ a }) => a.stance === s?.stance))
  const basicIndex = (v?: DpsView) => v?.basic.findIndex((x) => x.stance === s?.stance) ?? -1

  // Selection lives in the store so it survives tab switches.
  const main = $derived(planner.dpsMain)
  const chosen = $derived(planner.dpsOffhands)
  let showAll = $state(false)
  const equipped = (i: number) => (d?.abilities[i]?.offhands ?? 0) > 0
  const included = (_row: string, i: number) => planner.offhandIncluded(i)
  const offhands = $derived(
    (c?.abilities ?? [])
      .map((a, i) => ({ a, i }))
      .filter(({ a, i }) => showAll || equipped(i) || chosen[a.row])
      .sort((x, y) => Number(equipped(y.i)) - Number(equipped(x.i)) || (d?.abilities[y.i]?.dps ?? 0) - (d?.abilities[x.i]?.dps ?? 0)),
  )

  /** DPS of the chosen main attack and offhands, for a computed view (current or baseline). */
  const parts = (v: DpsView | undefined, boss: boolean) => planner.dpsParts(v, boss)

  /** Element conversions in effect (GlobalModifiers.X.ToY: one element's % bonus also counts for another), and what grants them. */
  const conversions = $derived.by(() => {
    const attrs = s?.attributes ?? {}
    const ch = planner.character
    const sels = ch ? [...ch.items.map((_, i) => i), 'weapon' as const] : []
    return Object.keys(attrs)
      .filter((t) => /^EasyRPG\.Attributes\.GlobalModifiers\.\w+\.To\w+$/.test(t) && attrs[t] >= 1)
      .map((t) => {
        const holder = sels.map((sel) => planner.item(sel)).find((it) => it && planner.effAttrs(it.slot ?? 'weapon').some((a) => a.tag === t))
        return { tag: t, name: c?.attributes[t]?.name || t.split('.').slice(-2).join(' → '), description: c?.attributes[t]?.description ?? '', from: holder?.name ?? '' }
      })
  })
  const total = (v: DpsView | undefined, boss: boolean) => {
    const p = parts(v, boss)
    return p.main + p.off
  }
  const mainName = $derived(main === 'basic' ? t('dps.basicName') : (attacks.find(({ a }) => a.row === main)?.a.name ?? '—'))

  const big = shortNumber
  const full = (n: number) => fmtNumber(n)
  const pct = (a: number, z: number) => (z ? (a / z - 1) * 100 : 0)
  const deltaText = (a: number, z: number) => (Math.abs(pct(a, z)) > 0.005 ? `${pct(a, z) > 0 ? '+' : ''}${pct(a, z).toFixed(2)}%` : '')
  const share = (p: { main: number; off: number }) => (p.main + p.off ? (p.off / (p.main + p.off)) * 100 : 0)
</script>

{#if s && d && c}
  {@const bi = basicIndex(d)}
  {@const basic = d.basic[bi]}
  {@const basicBase = b?.dps.basic[basicIndex(b?.dps)]}
  <div class="dps scroll">
    <div class="totals">
      {#each [false, true] as boss (boss)}
        {@const v = total(d, boss)}
        {@const z = total(b?.dps, boss)}
        {@const p = parts(d, boss)}
        <div class="total" class:boss>
          <div class="label">{boss ? t('dps.boss', { m: d.boss_mult.toFixed(2) }) : t('dps.normal')}</div>
          <div class="value" title={t('dps.perSecond', { v: full(v) })}>{big(v)}<small>/s</small></div>
          <div class="split">{t('dps.split', { a: big(p.main), o: big(p.off), p: share(p).toFixed(0) })}</div>
          <div class="split meter" title={t('dps.meterTitle')}>{t('dps.meter', { v: big(planner.dpsMeter(d, boss)) })}</div>
          <div class="delta" class:good={v > z} class:bad={v < z}>{deltaText(v, z)}</div>
        </div>
      {/each}
    </div>
    <div class="opts">{t('dps.calc')} <b>{mainName}</b> {t('dps.calcOffhands', { n: (c.abilities ?? []).filter((a, i) => included(a.row, i)).length })}</div>

    <h3 class="label">{t('dps.mainAttack', { stance: stanceName })}</h3>
    <div class="head row atk"><span></span><span>{t('dps.colAttack')}</span><span>{t('dps.colAvg')}</span><span>{t('dps.colRate')}</span><span>{t('dps.colDps')}</span><span title={t('dps.colBossTitle')}>{t('dps.colBoss')}</span></div>
    {#if basic}
      <button class="row atk" class:sel={main === 'basic'} onclick={() => (planner.dpsMain = 'basic')} title={t('dps.basicTitle', { n: basic.hits })}>
        <span class="radio">{main === 'basic' ? '◉' : '○'}</span>
        <span class="name">{t('dps.basic')}<small>{t('dps.basicSub', { n: basic.hits, t: basic.cycle_time.toFixed(2) })}</small></span>
        <span class="num" title={full(basic.avg)}>{big(basic.avg)}</span>
        <span class="num">{basic.rate.toFixed(2)}/s</span>
        <span class="num" title={full(basic.dps)}>{big(basic.dps)}<em class:good={basicBase && basic.dps > basicBase.dps} class:bad={basicBase && basic.dps < basicBase.dps}>{basicBase ? deltaText(basic.dps, basicBase.dps) : ''}</em></span>
        <span class="num bossv" title={full(basic.dps_boss)}>{big(basic.dps_boss)}</span>
      </button>
    {/if}
    {#each attacks as { a, i } (a.row)}
      {@const x = d.attacks[i]}
      {@const z = b?.dps.attacks[i]}
      <button class="row atk" class:sel={main === a.row} onclick={() => (planner.dpsMain = a.row)} title={a.description}>
        {#if a.icon}<img src={asset(a.icon)} alt="" />{:else}<span class="radio">{main === a.row ? '◉' : '○'}</span>{/if}
        <span class="name">{a.name}<small>{a.kind === 0 ? t('common.primary') : t('common.secondary')} · ×{a.multiplier} · normal {big(x.hit)} / crit {big(x.crit)}</small></span>
        <span class="num" title={full(x.avg)}>{big(x.avg)}</span>
        <span class="num" title={t('dps.animTitle', { t: x.cast_time.toFixed(3), e: x.energy_cost.toFixed(0) })}>
          {x.sustained_rate.toFixed(2)}/s{#if x.energy_limited}<span class="warn" title={t('dps.energyLimited', { r: x.burst_rate.toFixed(2) })}> ⚡</span>{/if}
        </span>
        <span class="num" title={full(x.dps)}>{big(x.dps)}<em class:good={z && x.dps > z.dps} class:bad={z && x.dps < z.dps}>{z ? deltaText(x.dps, z.dps) : ''}</em></span>
        <span class="num bossv" title={full(x.dps_boss)}>{big(x.dps_boss)}</span>
      </button>
    {/each}

    <h3 class="label">{t('dps.offhandsIn')}</h3>
    <div class="head row ab"><span></span><span>{t('dps.colAbility')}</span><span>{t('dps.colPerHit')}</span><span>{t('dps.colCooldown')}</span><span>{t('dps.colDps')}</span><span title={t('dps.colBossTitle')}>{t('dps.colBoss')}</span></div>
    {#each offhands as { a, i } (a.row)}
      {@const x = d.abilities[i]}
      {@const z = b?.dps.abilities[i]}
      {@const on = included(a.row, i)}
      <button class="row ab" class:off={!on} onclick={() => (planner.dpsOffhands = { ...chosen, [a.row]: !on })} title={a.description}>
        <span class="check">{on ? '☑' : '☐'}</span>
        <span class="name">{a.name}<small>{a.element || t('common.physical')} · {x.offhands ? t('dps.offhandCount', { n: x.offhands }) : t('dps.notEquipped')}{x.hits_detail ? ` · ${x.hits_detail}` : ''}</small></span>
        <span class="num" title={t('dps.perHitTitle', { v: full(x.per_proc), hits: x.hits !== 1 ? t('dps.hitsPerProc', { n: x.hits.toFixed(0) }) : '', b: full(x.per_proc_boss) })}>{big(x.per_proc)}{#if x.hits !== 1}<em>×{x.hits.toFixed(0)}</em>{/if}</span>
        <span class="num" title={x.cycle ? t('dps.cycleTitle', { t: x.cycle.toFixed(2) }) : ''}>{x.cooldown ? `${x.cooldown.toFixed(2)} s` : '—'}</span>
        <span class="num" title={full(x.dps)}>{x.cooldown ? big(x.dps) : '—'}<em class:good={z && x.dps > z.dps} class:bad={z && x.dps < z.dps}>{z ? deltaText(x.dps, z.dps) : ''}</em></span>
        <span class="num bossv" title={full(x.dps_boss)}>{x.cooldown ? big(x.dps_boss) : '—'}</span>
      </button>
    {/each}
    <button class="btn ghost more" onclick={() => (showAll = !showAll)}>{showAll ? t('dps.onlyEquipped') : t('dps.showAll')}</button>

    <div class="params">
      <span title={t('dps.critTitle', { t: (d.crit_total * 100).toFixed(1) })}>{t('dps.crit', { c: (d.crit_chance * 100).toFixed(0), t: (d.crit_total * 100).toFixed(0) })}</span>
      {#if d.offhand_crit_chance > 0}
        <span title={t('dps.offCritTitle')}>
          {t('dps.offCrit', { c: (d.offhand_crit_chance * 100).toFixed(1) })}{d.crit_total >= 10 ? t('dps.capReached', { m: ((d.crit_total - 10) * 100).toFixed(1) }) : t('dps.capMissing', { m: ((10 - d.crit_total) * 100).toFixed(1) })}
        </span>
      {:else}
        <span title={t('dps.noOffCritTitle')}>{t('dps.noOffCrit')}</span>
      {/if}
      <span>{t('dps.animSpeed', { v: (d.attack_speed_bonus * 100).toFixed(1) })}</span>
      <span title={t('dps.cdTitle')}>{t('dps.cd', { x: d.cast_speed.toFixed(2), r: ((1 - 1 / d.cast_speed) * 100).toFixed(1) })}</span>
      {#each conversions as cv (cv.tag)}
        <span class="conv" title={cv.description}>{cv.name}{cv.from ? ` (${cv.from})` : ''}</span>
      {/each}
      <span>{t('dps.energy', { v: d.energy_regen.toFixed(1) })}</span>
    </div>
    <p class="fine">{t('dps.fine')}</p>
  </div>
{/if}

<style>
  .dps {
    flex: 1;
    min-height: 0;
    padding-right: 6px;
    font-family: var(--font-ui);
  }
  .totals {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    margin: 4px 0 6px;
  }
  .total {
    border: 1px solid var(--gold-dim);
    background: linear-gradient(180deg, rgba(201, 162, 74, 0.1), transparent);
    padding: 8px 10px;
    text-align: center;
  }
  .total.boss {
    border-color: rgba(229, 115, 95, 0.55);
    background: linear-gradient(180deg, rgba(229, 115, 95, 0.12), transparent);
  }
  .total .label {
    font-size: 11px;
  }
  .value {
    font-family: var(--font-title);
    font-size: 26px;
    color: var(--gold-bright);
    font-variant-numeric: tabular-nums;
  }
  .value small {
    font-size: 13px;
    color: var(--text-dim);
    margin-left: 2px;
  }
  .split {
    font-size: 11.5px;
    color: var(--text-dim);
  }
  .split.meter {
    color: var(--gold);
  }
  .delta {
    font-size: 13px;
    min-height: 16px;
  }
  .opts {
    font-size: 13px;
    color: var(--text-dim);
  }
  h3 {
    font-size: 13px;
    margin: 12px 0 4px;
  }
  .row {
    display: grid;
    grid-template-columns: 26px minmax(0, 1fr) 62px 60px 78px 70px;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 4px 4px;
    background: none;
    border: 1px solid transparent;
    border-radius: 2px;
    text-align: left;
    font-size: 13.5px;
  }
  button.row {
    cursor: pointer;
  }
  button.row:hover {
    background: rgba(201, 162, 74, 0.07);
  }
  .row.sel {
    border-color: var(--gold);
    background: rgba(201, 162, 74, 0.1);
  }
  .row.off {
    opacity: 0.5;
  }
  .radio,
  .check {
    color: var(--gold-bright);
    font-size: 17px;
    text-align: center;
  }
  .head {
    color: var(--text-faint);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .head span:nth-child(n + 3) {
    text-align: right;
  }
  .row img {
    width: 26px;
    height: 26px;
    border-radius: 3px;
  }
  .name {
    display: flex;
    flex-direction: column;
    min-width: 0;
    font-family: var(--font-text);
    font-size: 15px;
  }
  .name small {
    font-family: var(--font-ui);
    font-size: 11.5px;
    color: var(--text-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .num {
    text-align: right;
    font-variant-numeric: tabular-nums;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
  }
  .num em {
    font-style: normal;
    font-size: 11px;
  }
  .bossv {
    color: #f3a593;
  }
  .warn {
    color: #f0c46b;
  }
  .more {
    width: 100%;
    margin-top: 4px;
  }
  .conv {
    color: var(--good);
  }
  .params {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 14px;
    margin-top: 10px;
    font-size: 12px;
    color: var(--text-dim);
  }
  .fine {
    font-size: 11.5px;
    color: var(--text-faint);
    margin: 6px 0;
    line-height: 1.35;
  }
</style>
