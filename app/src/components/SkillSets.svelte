<script lang="ts">
  import { formatValue } from '../lib/format'
  import { debug } from '../lib/debug.svelte'
  import { asset } from '../lib/game'
  import { t } from '../lib/i18n.svelte'
  import { CARD_TABLE, levelSteps, shownLevel, shownMax, storedLevel, storedMax } from '../lib/skills'
  import { planner } from '../lib/store.svelte'
  import type { SkillInfo, SkillSet } from '../lib/types'
  import ImpactBadge from './ImpactBadge.svelte'

  /** Stance skills, crystal cards and the elven tree: levels are free here (no point budget). Levels are the
   *  ones the game shows (lib/skills.ts: a crystal card's third level is stored as 6). */
  const info = (tag: string) => planner.catalog?.attributes[tag]
  // Skill flags (Skills.Spear.SpearQ = 1) unlock behaviour: they have no name to show.
  const shownAttrs = (s: SkillInfo) => s.attributes.filter(([t]) => info(t)?.name || !t.startsWith('EasyRPG.Attributes.Skills.'))
  const bonusText = (s: SkillInfo, level = 1) =>
    shownAttrs(s).map(([t, v]) => `${formatValue(v * level, info(t))} ${info(t)?.name || t.split('.').pop()}`).join(' · ') || t('skills.special')
  const label = (s: SkillInfo) => s.name || bonusText(s)

  // The current stance's table first, then cards, then the rest.
  const sets = $derived(
    [...(planner.catalog?.skill_sets ?? [])].sort((a, b) => rank(a) - rank(b)),
  )
  const rank = (s: SkillSet) => (s.stance !== null && s.stance === planner.stats?.stance ? 0 : s.stance === null && s.table.includes('Cards') ? 1 : s.stance === null ? 3 : 2)
  let open = $state<Record<string, boolean>>({})
  const isOpen = (s: SkillSet) => open[s.table] ?? rank(s) < 2
  const setName = (s: SkillSet) => t(`skillset.${s.name}`, {}, s.name)

  const stored = (set: SkillSet, s: SkillInfo) => planner.skillLevel(set.table, s.row)
  const level = (set: SkillSet, s: SkillInfo) => shownLevel(set.table, stored(set, s))
  const maxLevel = (set: SkillSet, s: SkillInfo) => shownMax(set.table, s.max_level)
  const saved = (set: SkillSet, s: SkillInfo) => planner.savedSkillLevel(set.table, s.row)
  const steps = (set: SkillSet, s: SkillInfo) => levelSteps(set.table, stored(set, s), s.max_level)
  const gain = (st: SkillSet, s: SkillInfo) => planner.optionImpact(s, false, steps(st, s).up)
  const loss = (st: SkillSet, s: SkillInfo) => (level(st, s) > 0 ? planner.optionImpact(s, true, steps(st, s).down) : undefined)
  const set = (st: SkillSet, s: SkillInfo, l: number) => {
    if (!Number.isFinite(l)) return
    const top = maxLevel(st, s)
    planner.setSkillLevel(st.table, s.row, storedLevel(st.table, Math.max(0, top > 0 ? Math.min(Math.round(l), top) : Math.round(l))), storedMax(st.table, s.max_level))
  }
  const canUp = (st: SkillSet, s: SkillInfo) => steps(st, s).up > 0

  /** Best single levels to add, across all tables. */
  const best = $derived(
    sets
      .flatMap((st) => st.skills.filter((s) => canUp(st, s)).map((s) => ({ st, s, g: gain(st, s) })))
      .filter((x) => x.g !== undefined && Number.isFinite(x.g) && x.g > 1e-7)
      .sort((a, b) => b.g! - a.g!)
      .slice(0, 8),
  )
  const edited = $derived(Object.keys(planner.skillLevels).length)
</script>

<div class="sets scroll">
  <div class="top">
    <div class="label">{t('skills.best')}</div>
    {#if edited}<button class="btn ghost" onclick={() => (planner.skillLevels = {})}>{t('skills.reset', { n: edited })}</button>{/if}
  </div>
  <div class="best">
    {#each best as { st, s, g } (st.table + s.row)}
      <button class="pick" onclick={() => set(st, s, level(st, s) + 1)} title={t('skills.plusOne')}>
        {#if s.icon}<img src={asset(s.icon)} alt="" />{/if}
        <span class="pname">{label(s)}<small>{setName(st)} · {t('skills.level', { n: level(st, s) })}{maxLevel(st, s) > 0 ? `/${maxLevel(st, s)}` : ''}</small></span>
        <ImpactBadge value={g} title={t('skills.gainTitle')} />
      </button>
    {:else}
      {#if planner.treeAnalyzeError}
        <button class="none err" title={planner.treeAnalyzeError} onclick={() => (debug.open = true)}>{t('common.analyzeFailed')}</button>
      {:else}
        <p class="none">{planner.treeAnalyzing || !Object.keys(planner.treeImpacts).length ? t('common.analyzing') : t('skills.none')}</p>
      {/if}
    {/each}
  </div>

  {#each sets as st (st.table)}
    <section>
      <button class="head" onclick={() => (open = { ...open, [st.table]: !isOpen(st) })}>
        <span class="arrow">{isOpen(st) ? '▾' : '▸'}</span>
        <span class="title">{setName(st)}</span>
        {#if st.stance !== null && st.stance === planner.stats?.stance}<span class="cur">{t('skills.current')}</span>{/if}
        <span class="count">{st.skills.filter((s) => level(st, s) > 0).length}/{st.skills.length}</span>
      </button>
      {#if isOpen(st) && st.table === CARD_TABLE}<p class="note">{t('skills.cardsNote')}</p>{/if}
      {#if isOpen(st)}
        {#each st.skills as s (s.row)}
          {@const l = level(st, s)}
          {@const top = maxLevel(st, s)}
          <div class="skill" class:off={l === 0} class:changed={stored(st, s) !== saved(st, s)}>
            {#if s.icon}<img src={asset(s.icon)} alt="" />{:else}<span class="noicon"></span>{/if}
            <span class="sname" title={s.description || s.row}>
              {label(s)}
              <small>{t('skills.perLevel', { bonus: bonusText(s) })}{s.requires ? ` · ${s.requires}` : ''}</small>
              {#if stored(st, s) > 1}<small class="total">{t('skills.total', { bonus: bonusText(s, stored(st, s)) })}</small>{/if}
            </span>
            <span class="badges">
              <ImpactBadge value={canUp(st, s) ? gain(st, s) : undefined} title={t('skills.gainTitle')} />
              {#if l > 0}<ImpactBadge value={loss(st, s)} title={t('skills.lossTitle')} />{/if}
            </span>
            <span class="lvl">
              <button class="btn ghost" disabled={l === 0} onclick={() => set(st, s, l - 1)}>−</button>
              <input class="field num" type="number" min="0" max={top > 0 ? top : undefined} value={l} onchange={(e) => set(st, s, parseFloat(e.currentTarget.value))} />
              <button class="btn ghost" disabled={!canUp(st, s)} onclick={() => set(st, s, l + 1)}>+</button>
              <small>{top > 0 ? `/${top}` : ''}</small>
            </span>
          </div>
        {/each}
      {/if}
    </section>
  {/each}
  <p class="fine">{t('skills.fine')}</p>
</div>

<style>
  .sets {
    min-height: 0;
    padding: 14px 18px;
    font-family: var(--font-ui);
  }
  .top {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .label {
    font-size: 11px;
  }
  .best {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 4px 10px;
    margin: 6px 0 14px;
  }
  .pick,
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    background: none;
    border: 1px solid transparent;
    color: var(--text);
    text-align: left;
    cursor: pointer;
    padding: 4px 6px;
  }
  .pick:hover,
  .head:hover {
    background: rgba(201, 162, 74, 0.08);
  }
  .pick img,
  .skill img,
  .noicon {
    width: 28px;
    height: 28px;
    border-radius: 4px;
    flex: none;
  }
  .pname,
  .sname {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    font-family: var(--font-text);
    font-size: 14.5px;
  }
  small {
    font-family: var(--font-ui);
    font-size: 11.5px;
    color: var(--text-dim);
  }
  .total {
    color: var(--good);
  }
  section {
    border-top: 1px solid var(--line);
    padding: 4px 0;
  }
  .head .title {
    font-family: var(--font-title);
    font-size: 16px;
    color: var(--gold-bright);
  }
  .head .cur {
    font-size: 11px;
    color: var(--good);
  }
  .head .count {
    margin-left: auto;
    color: var(--text-dim);
    font-variant-numeric: tabular-nums;
  }
  .arrow {
    width: 12px;
    color: var(--gold);
  }
  .skill {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 3px 6px 3px 26px;
  }
  .skill.off {
    opacity: 0.6;
  }
  .skill.changed .sname {
    color: var(--gold-bright);
  }
  .badges {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 2px;
    min-width: 70px;
  }
  .lvl {
    display: flex;
    align-items: center;
    gap: 3px;
  }
  .lvl input {
    width: 64px;
  }
  .lvl small {
    width: 34px;
  }
  .none,
  .note {
    margin: 2px 0 6px 22px;
    font-size: 12px;
    color: var(--text-dim);
  }
  .fine {
    font-size: 12px;
    color: var(--text-faint);
  }
  .none.err {
    display: block;
    width: 100%;
    border: none;
    background: none;
    color: var(--bad);
    text-align: left;
    cursor: pointer;
  }
</style>
