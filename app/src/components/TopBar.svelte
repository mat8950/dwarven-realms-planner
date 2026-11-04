<script lang="ts">
  import { formatValue, fromDisplay, toDisplay } from '../lib/format'
  import { asset } from '../lib/game'
  import { LANGUAGES, i18n, stanceName, t } from '../lib/i18n.svelte'
  import { debug } from '../lib/debug.svelte'
  import { planner } from '../lib/store.svelte'
  import ObjectiveSelect from './ObjectiveSelect.svelte'

  const maxPotions = $derived(planner.stats?.max_potions ?? 0)
  // The engine's stance follows a swapped weapon; the save's is only the fallback.
  const stance = $derived(planner.stance ?? planner.stats?.stance ?? planner.character?.default_stance ?? 0)
  const weaponStance = $derived(planner.weaponInfo()?.stance ?? planner.character?.default_stance)
  const editedCount = $derived(
    (planner.character ? [...planner.character.items.map((_, i) => i), 'weapon' as const].filter((s) => planner.isEdited(s)).length : 0) + (planner.tree ? 1 : 0) + (Object.keys(planner.skillLevels).length ? 1 : 0),
  )

  let open = $state<'effects' | 'builds' | null>(null)
  let buildName = $state('')
  let effectTag = $state('')
  let effectValue = $state(0)
  const info = (tag: string) => planner.catalog?.attributes[tag]

  const effectChoices = $derived(
    Object.entries(planner.catalog?.attributes ?? {})
      .filter(([t, a]) => a.name && a.value_type !== 3 && !t.startsWith('EasyRPG.Items.Modifiers.'))
      .map(([tag, a]) => ({ tag, name: a.name }))
      .sort((a, b) => a.name.localeCompare(b.name)),
  )

  let statusRow = $state('')
  let statusQuery = $state('')
  const statusInfo = (row: string) => planner.catalog?.status_effects.find((e) => e.row === row)
  const statusLabel = (e: { row: string; name: string }) => (e.name && e.name !== 'None' ? e.name : e.row)
  /** Buffs to add. The game has several rows for one buff (one per source): rows with the same name, stacks
   *  and stats show once; rows with the same name but other stats show their bonus to tell them apart. */
  const statusChoices = $derived.by(() => {
    const q = statusQuery.trim().toLowerCase()
    const seen = new Set<string>()
    const unique = (planner.catalog?.status_effects ?? [])
      .filter((e) => e.positive)
      .filter((e) => {
        const key = `${statusLabel(e)}|${e.max_stack}|${JSON.stringify(e.attributes)}`
        if (seen.has(key)) return false
        seen.add(key)
        return true
      })
    const named = new Map<string, number>()
    for (const e of unique) named.set(statusLabel(e), (named.get(statusLabel(e)) ?? 0) + 1)
    return unique
      .filter((e) => !(e.row in planner.statusEffects))
      .filter((e) => !q || `${statusLabel(e)} ${e.row} ${e.description}`.toLowerCase().includes(q))
      .map((e) => ({ ...e, label: (named.get(statusLabel(e)) ?? 0) > 1 ? `${statusLabel(e)} (${statusBonus(e.row, e.max_stack) || e.row})` : statusLabel(e) }))
      .sort((a, b) => a.label.localeCompare(b.label))
  })

  /** Keeps a popup inside the window: shifted left when it would overflow, scrolled when too tall. */
  function keepInView(node: HTMLElement) {
    const fit = () => {
      node.style.left = '0px'
      const r = node.getBoundingClientRect()
      const over = r.right - (window.innerWidth - 12)
      if (over > 0) node.style.left = `${-Math.min(over, Math.max(0, r.left - 12))}px`
      node.style.maxHeight = `${Math.max(200, window.innerHeight - r.top - 12)}px`
    }
    fit()
    window.addEventListener('resize', fit)
    return { destroy: () => window.removeEventListener('resize', fit) }
  }
  const statusBonus = (row: string, stacks: number) =>
    (statusInfo(row)?.attributes ?? []).map(([t, v]) => `${formatValue(v * stacks, info(t))} ${info(t)?.name || t.split('.').pop()}`).join(' · ')
  const activeCount = $derived(planner.effects.length + Object.keys(planner.statusEffects).length + (planner.ruptureOrbs > 0 ? 1 : 0) + Number(planner.colossus) + Number(planner.unholyVoid))
  function setStatus(row: string, stacks: number) {
    const max = statusInfo(row)?.max_stack ?? 1
    if (!Number.isFinite(stacks)) return
    const n = Math.max(0, Math.min(Math.round(stacks), max))
    const { [row]: _, ...rest } = planner.statusEffects
    planner.statusEffects = n > 0 ? { ...rest, [row]: n } : rest
  }

  function addEffect() {
    if (!effectTag || !Number.isFinite(effectValue)) return
    planner.effects = [...planner.effects.filter((e) => e.tag !== effectTag), { tag: effectTag, value: fromDisplay(effectValue, info(effectTag)) }]
    effectValue = 0
  }
</script>

<header class="bar frame">
  <div class="brand">
    <span class="title">Dwarven Realms</span>
    <span class="label sub">Planner</span>
  </div>

  <label class="group">
    <span class="label">{t('top.character')}</span>
    <select class="field" value={planner.character?.save} onchange={(e) => planner.open(e.currentTarget.value)}>
      {#each planner.saves as s (s.path)}
        <option value={s.path}>{s.name} · {t('top.level', { n: s.level })}</option>
      {/each}
    </select>
  </label>

  <label class="group">
    <span class="label">{t('top.stance')}</span>
    <select class="field" value={stance} onchange={(e) => (planner.stance = +e.currentTarget.value)}>
      {#each planner.catalog?.stances ?? [] as [v, n] (v)}
        <option value={v}>{stanceName(n)}{v === weaponStance ? t('top.weaponStance') : ''}</option>
      {/each}
    </select>
  </label>

  <label class="group potions">
    <span class="label">{t('top.potions')}</span>
    <input
      type="range"
      min="0"
      max={maxPotions}
      value={planner.potions ?? maxPotions}
      oninput={(e) => (planner.potions = +e.currentTarget.value >= maxPotions ? null : +e.currentTarget.value)}
    />
    <span class="pv">{planner.potions ?? maxPotions}/{maxPotions}</span>
  </label>

  <label class="group">
    <span class="label">{t('top.objective')}</span>
    <span class="objective"><ObjectiveSelect /></span>
    <span class="spin" class:on={planner.analyzing} title={t('top.analyzing')}>◌</span>
  </label>

  <label class="group">
    <span class="label">{t('top.forge')}</span>
    <select class="field" bind:value={planner.forgeMode}>
      <option value="current">{t('top.forgeCurrent')}</option>
      <option value="none">{t('top.forgeNone')}</option>
      <option value="max">{t('top.forgeMax')}</option>
    </select>
  </label>

  <label class="group check">
    <input type="checkbox" bind:checked={planner.hideUseless} />
    <span>{t('top.hideUseless')}</span>
  </label>

  <div class="group pop-anchor">
    <button class="btn" onclick={() => (open = open === 'effects' ? null : 'effects')}>{t('top.effects')} {activeCount ? `(${activeCount})` : ''}</button>
    {#if open === 'effects'}
      <div class="pop frame" use:keepInView>
        <div class="label">{t('fx.game')}</div>
        <p class="hint">{t('fx.gameHint')}</p>
        {#each Object.entries(planner.statusEffects) as [row, stacks] (row)}
          {@const e = statusInfo(row)}
          <div class="eff status">
            {#if e?.icon}<img src={asset(e.icon)} alt="" />{/if}
            <span class="sname" title={e?.description}>{e ? statusLabel(e) : row}<small>{statusBonus(row, stacks) || e?.description}</small></span>
            <input class="field stacks" type="number" min="1" max={e?.max_stack} value={stacks} onchange={(ev) => setStatus(row, parseFloat(ev.currentTarget.value))} />
            <span class="v">/{e?.max_stack}</span>
            <button class="btn ghost" onclick={() => setStatus(row, 0)}>✕</button>
          </div>
        {/each}
        <div class="eff-add">
          <input class="field" placeholder={t('fx.search')} bind:value={statusQuery} />
          <select class="field" bind:value={statusRow}>
            <option value="">{t('fx.pick')}</option>
            {#each statusChoices as e (e.row)}<option value={e.row}>{e.max_stack > 1 ? t('fx.option', { name: e.label, n: e.max_stack }) : e.label}</option>{/each}
          </select>
          <button class="btn" disabled={!statusRow} onclick={() => (setStatus(statusRow, statusInfo(statusRow)?.max_stack ?? 1), (statusRow = ''))}>{t('common.add')}</button>
        </div>
        <div class="states">
          <label title={t('fx.orbsTitle')}>{t('fx.orbs')} <input class="field stacks" type="number" min="0" value={planner.ruptureOrbs} onchange={(e) => (planner.ruptureOrbs = Math.max(0, Math.round(parseFloat(e.currentTarget.value) || 0)))} /></label>
          <label><input type="checkbox" bind:checked={planner.unholyVoid} /> {t('fx.void')}</label>
          <label><input type="checkbox" bind:checked={planner.colossus} /> {t('fx.colossus')}</label>
        </div>
        <hr class="gold-rule" />
        <div class="label">{t('fx.free')}</div>
        <p class="hint">{t('fx.freeHint')}</p>
        {#each planner.effects as e (e.tag)}
          <div class="eff">
            <span>{info(e.tag)?.name || e.tag}</span>
            <span class="v">{+toDisplay(e.value, info(e.tag)).toFixed(3)}{info(e.tag)?.value_type === 2 ? '%' : ''}</span>
            <button class="btn ghost" onclick={() => (planner.effects = planner.effects.filter((x) => x.tag !== e.tag))}>✕</button>
          </div>
        {/each}
        <div class="eff-add">
          <select class="field" bind:value={effectTag}>
            <option value="">{t('fx.attribute')}</option>
            {#each effectChoices as c (c.tag)}<option value={c.tag}>{c.name}</option>{/each}
          </select>
          <input class="field" type="number" step="any" bind:value={effectValue} />
          <button class="btn" onclick={addEffect} disabled={!effectTag}>{t('common.add')}</button>
        </div>
      </div>
    {/if}
  </div>

  <button class="btn" class:edited-tree={planner.tree !== null} onclick={() => (planner.treeOpen = true)} title={t('top.treeTitle')}>
    {t('top.tree', { n: planner.treeRows().length, max: planner.treeBudget() })}
  </button>
  <button class="btn" onclick={() => (planner.historyOpen = true)} title={t('top.historyTitle')}>{t('top.history')}</button>
  <button class="btn" disabled={!planner.character} onclick={() => (planner.inventoryOpen = true)} title={t('top.inventoryTitle')}>{t('top.inventory')}</button>
  {#if planner.setup}
    <button class="btn" class:stale={planner.setup.install && !planner.setup.up_to_date} onclick={() => (planner.setupOpen = true)} title={planner.setup.install && !planner.setup.up_to_date ? t('top.dataOld') : t('top.dataTitle')}>{t('top.data')}</button>
  {/if}

  <div class="group pop-anchor">
    <button class="btn" onclick={() => (open = open === 'builds' ? null : 'builds')}>{t('top.builds')}</button>
    {#if open === 'builds'}
      <div class="pop frame" use:keepInView>
        <div class="label">{t('builds.save')}</div>
        <div class="eff-add">
          <input class="field" placeholder={t('builds.name')} bind:value={buildName} />
          <button class="btn" disabled={!buildName} onclick={() => (planner.saveBuild(buildName), (buildName = ''))}>{t('builds.store')}</button>
        </div>
        <hr class="gold-rule" />
        {#each planner.builds.filter((b) => b.save === planner.character?.save) as b (b.name)}
          <div class="eff">
            <span>{b.name}</span>
            <button class="btn" onclick={() => ((planner.loadBuild(b)), (open = null))}>{t('builds.load')}</button>
            <button class="btn ghost" onclick={() => planner.deleteBuild(b.name)}>✕</button>
          </div>
        {:else}
          <p class="hint">{t('builds.none')}</p>
        {/each}
      </div>
    {/if}
  </div>

  <div class="spacer"></div>
  <button class="btn" onclick={() => planner.resetEdits()} disabled={!editedCount && !activeCount && planner.stance === null && planner.potions === null}>
    {t('top.reset')}{#if editedCount}<span class="edited">{t('top.edited', { n: editedCount })}</span>{/if}
  </button>
  <button class="btn ghost dbg" class:alert={debug.errors > 0} title={t('top.debugTitle')} onclick={() => (debug.open = !debug.open)}>
    Debug{#if debug.errors}<span class="nerr">{debug.errors}</span>{/if}
  </button>
  <label class="group lang" title={t('top.language')}>
    <select class="field" value={i18n.lang} onchange={(e) => planner.setLanguage(e.currentTarget.value)}>
      {#each LANGUAGES as l (l.code)}<option value={l.code}>{l.name}</option>{/each}
    </select>
  </label>
  <span class="build" title={t('top.buildTitle')}>{planner.catalog?.build}</span>
</header>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 8px 14px;
    flex-wrap: wrap;
    white-space: nowrap;
    padding: 10px 18px;
  }
  .brand {
    display: flex;
    flex-direction: column;
    line-height: 1;
    margin-right: 8px;
  }
  .brand .title {
    font-size: 21px;
  }
  .brand .sub {
    font-size: 11px;
    margin-top: 3px;
  }
  .group {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .eff.status img {
    width: 26px;
    height: 26px;
    border-radius: 4px;
  }
  .eff.status .sname {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    white-space: normal;
    overflow-wrap: anywhere;
  }
  .eff.status small {
    font-family: var(--font-ui);
    font-size: 11.5px;
    color: var(--text-dim);
  }
  .states {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 14px;
    margin-top: 8px;
    font-family: var(--font-ui);
    font-size: 13px;
    color: var(--text-dim);
  }
  .states label {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .eff .stacks {
    width: 64px;
  }
  .edited-tree {
    border-color: var(--gold-bright);
  }
  .stale {
    border-color: var(--bad);
  }
  .group .label {
    font-size: 11px;
  }
  .potions input {
    width: 110px;
    accent-color: var(--gold);
  }
  .pv {
    font-family: var(--font-ui);
    font-variant-numeric: tabular-nums;
    color: var(--text-dim);
    min-width: 64px;
  }
  .pop-anchor {
    position: relative;
  }
  .pop {
    position: absolute;
    top: calc(100% + 10px);
    left: 0;
    width: 380px;
    max-width: calc(100vw - 24px);
    overflow-y: auto;
    /* The top bar keeps its controls on one line; popup texts wrap. */
    white-space: normal;
    padding: 12px 14px;
    z-index: 100;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .hint {
    margin: 0;
    color: var(--text-dim);
    font-size: 13px;
  }
  .eff {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .eff span:first-child {
    flex: 1;
  }
  .eff .v {
    font-variant-numeric: tabular-nums;
  }
  .eff-add {
    display: flex;
    gap: 6px;
  }
  .eff-add select,
  .eff-add input:first-child {
    flex: 1;
    min-width: 0;
  }
  .eff-add input[type='number'] {
    width: 80px;
  }
  .spacer {
    flex: 1;
  }
  .objective {
    max-width: 190px;
  }
  .spin {
    color: var(--gold);
    opacity: 0;
    transition: opacity 0.2s;
  }
  .spin.on {
    opacity: 1;
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .check {
    font-family: var(--font-ui);
    color: var(--text-dim);
    cursor: pointer;
  }
  .check input {
    accent-color: var(--gold);
  }
  .edited {
    color: var(--gold-bright);
    font-family: var(--font-ui);
  }
  .dbg.alert {
    color: var(--bad);
    border-color: rgba(229, 115, 95, 0.5);
  }
  .nerr {
    margin-left: 6px;
    background: var(--bad);
    color: #1a0d0a;
    border-radius: 8px;
    padding: 0 6px;
    font-size: 12px;
  }
  .build {
    color: var(--text-faint);
    font-family: var(--font-ui);
    font-size: 12px;
  }
</style>
