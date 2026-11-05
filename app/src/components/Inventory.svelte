<script lang="ts">
  // Inventory and home fort stash of the open character. Two ways to sort items:
  // - wanted stats: each item's reroll pools (Pool1-3) list the affixes a reroll can give; an item
  //   scores the number of pools that can roll a wanted stat (1, 2 or 3);
  // - the build: each equipment item measured in its slot by the engine, gain on the objective.
  import { onDestroy } from 'svelte'
  import { formatValue } from '../lib/format'
  import { RARITY, asset } from '../lib/game'
  import { fmtDate, rarityName, slotName, t } from '../lib/i18n.svelte'
  import { planner } from '../lib/store.svelte'
  import type { InvItem } from '../lib/types'
  import ImpactBadge from './ImpactBadge.svelte'
  import ObjectiveSelect from './ObjectiveSelect.svelte'

  // The panel's choices are kept (local storage, copied by the backend): wanted stats, filter, sort.
  function load<T>(key: string, fallback: T): T {
    try {
      const v = localStorage.getItem(key)
      return v === null ? fallback : JSON.parse(v)
    } catch {
      return fallback
    }
  }
  function keep(key: string, value: unknown) {
    try {
      localStorage.setItem(key, JSON.stringify(value))
    } catch {
      // no storage: the choice lasts for this session
    }
  }
  let wanted = $state<string[]>(load('dr-planner.inventoryWanted', []))
  let minMatch = $state<number>(load('dr-planner.inventoryShow', 0))
  let sortByGain = $state<boolean>(load('dr-planner.inventorySort', false))
  $effect(() => keep('dr-planner.inventoryWanted', wanted))
  $effect(() => keep('dr-planner.inventoryShow', minMatch))
  $effect(() => keep('dr-planner.inventorySort', sortByGain))

  let tab = $state<string | null>(null)
  let query = $state('')
  let picked = $state<string | null>(null)

  // Always read the saves again: the game may have changed them since the last look. While the view is open,
  // a new save of the game is taken in every 5 s.
  void planner.loadInventory()
  const poll = setInterval(() => void planner.refreshInventory(), 5000)
  // How old the game's last save is: loot picked up since is not in it.
  let now = $state(Date.now() / 1000)
  const clock = setInterval(() => (now = Date.now() / 1000), 1000)
  onDestroy(() => (clearInterval(poll), clearInterval(clock)))
  const age = (saved: number) => {
    const s = Math.max(0, Math.round(now - saved))
    return s < 60 ? t('inv.ageSeconds', { n: s }) : s < 3600 ? t('inv.ageMinutes', { n: Math.floor(s / 60) }) : fmtDate(saved)
  }

  const tabs = $derived(planner.inventory)
  // The bag (first tab) by default; the tab picked stays picked when the saves are read again.
  const current = $derived(tabs.find((c) => c.id === tab) ?? tabs[0])
  const info = (tag: string) => planner.catalog?.attributes[tag]
  const attrName = (tag: string) => info(tag)?.name || tag.split('.').pop() || tag

  /** Affix row -> stat tag (DT_Base_Item_Attributes). */
  const rowTag = $derived(new Map((planner.catalog?.affixes ?? []).map((a) => [a.row, a.tag])))
  const tagOf = (row: string) => rowTag.get(row) ?? row
  const wantedSet = $derived(new Set(wanted))

  /** Stats an item can get: one entry per tag among rollable affixes. */
  const choices = $derived.by(() => {
    const q = query.trim().toLowerCase()
    if (!q) return []
    const seen = new Set<string>()
    return (planner.catalog?.affixes ?? [])
      .filter((a) => a.can_roll && !wantedSet.has(a.tag) && !seen.has(a.tag) && (seen.add(a.tag), true))
      .map((a) => ({ tag: a.tag, name: attrName(a.tag) }))
      .filter((c) => c.name.toLowerCase().includes(q))
      .sort((a, b) => a.name.localeCompare(b.name))
      .slice(0, 30)
  })

  /** The objective's best rollable stats (the planner's keys). */
  function usePriorities() {
    const tags = [...new Set((planner.catalog?.affixes ?? []).filter((a) => a.can_roll).map((a) => a.tag))]
      .map((tag) => [tag, planner.impact(tag) ?? 0] as const)
      .filter(([tag, g]) => g > 1e-7 && planner.sourceTags.has(tag))
      .sort((a, b) => b[1] - a[1])
      .slice(0, 5)
      .map(([tag]) => tag)
    wanted = [...new Set([...wanted, ...tags])]
  }

  const matches = (it: InvItem) => (wanted.length ? it.pools.filter((p) => p.some((r) => wantedSet.has(tagOf(r)))).length : 0)
  const rolled = (it: InvItem) => it.attributes.filter((a) => wantedSet.has(a.tag)).length
  const key = (it: InvItem) => `${current?.id}#${it.index}`
  const gainOf = (it: InvItem) => {
    const g = planner.itemGains[key(it)]
    return g ? planner.gainOf(g.impact) : undefined
  }
  const stale = $derived(planner.itemGainsFor !== '' && planner.itemGainsFor !== planner.buildKey())
  const evaluated = $derived(Object.keys(planner.itemGains).length > 0)
  const isEquipment = (it: InvItem) => it.weapon || it.slots.length > 0

  const shown = $derived.by(() => {
    if (!current) return []
    let items = current.items.filter((it) => !minMatch || matches(it) >= minMatch)
    if (sortByGain) items = [...items].filter((it) => gainOf(it) !== undefined).sort((a, b) => (gainOf(b) ?? 0) - (gainOf(a) ?? 0))
    return items
  })
  /** The grid keeps the game's layout (empty slots included) unless it is filtered or sorted. */
  const grid = $derived.by(() => {
    if (!current || minMatch || sortByGain) return shown.map((it) => ({ i: it.index, it }))
    const byIndex = new Map(current.items.map((it) => [it.index, it]))
    return Array.from({ length: current.size }, (_, i) => ({ i, it: byIndex.get(i) ?? null }))
  })
  const sel = $derived(current?.items.find((it) => key(it) === picked) ?? null)
  const selGain = $derived(sel ? planner.itemGains[key(sel)] : undefined)

  const slotLabel = (it: InvItem) =>
    it.weapon ? t('slot.weapon') : it.slots.map((s) => slotName(planner.catalog?.slots.find((x) => x.value === s)?.name ?? String(s))).join(' / ') || t('inv.notEquipment')
  const tabLabel = (c: { kind: string; number: number }) => (c.kind === 'inventory' ? t('inv.inventory') : t('inv.stash', { n: c.number }))
  const matchCount = (c: { items: InvItem[] }) => (wanted.length ? c.items.filter((it) => matches(it) > 0).length : 0)
</script>

<div class="overlay">
  <div class="panel">
    <header>
      <span class="title">{t('inv.title')}<span class="who">{planner.character?.name}</span></span>
      <span class="actions">
        <button class="btn ghost" disabled={planner.inventoryLoading} onclick={() => planner.loadInventory()} title={t('inv.reloadTitle')}>{t('inv.reload')}</button>
        <button class="btn" onclick={() => (planner.inventoryOpen = false)}>{t('common.close')}</button>
      </span>
    </header>

    <div class="tabs">
      {#each tabs as c (c.id)}
        <button class="tab" class:on={c.id === current?.id} onclick={() => ((tab = c.id), (picked = null))}>
          {tabLabel(c)} <small>{c.items.length}/{c.size}</small>
          {#if matchCount(c)}<span class="hits">{matchCount(c)}</span>{/if}
        </button>
      {/each}
      {#if planner.inventoryLoading}<span class="muted">{t('app.loading')}</span>{/if}
      {#if current?.saved}
        <span class="saved" class:old={now - current.saved > 120} title={t('inv.savedTitle')}>{t('inv.savedAgo', { age: age(current.saved) })}</span>
      {/if}
    </div>
    {#if current && !current.items.length && !planner.inventoryLoading}
      <p class="empty-note">{t(current.kind === 'inventory' ? 'inv.emptyBag' : 'inv.emptyStash')}</p>
    {/if}

    <div class="body">
      <div class="grid-wrap scroll">
        <div class="grid">
          {#each grid as { i, it } (i)}
            {#if it}
              {@const m = matches(it)}
              {@const look = RARITY[it.rarity] ?? RARITY[0]}
              <button
                class="slot m{m}"
                class:dim={wanted.length > 0 && m === 0}
                class:picked={picked === key(it)}
                style:--frame={look.frame}
                title={it.name}
                onclick={() => (picked = picked === key(it) ? null : key(it))}
              >
                <img class="bg" src={asset(look.bg)} alt="" />
                {#if it.icon}<img class="icon" src={asset(it.icon)} alt="" />{/if}
                {#if it.tier > 0}<span class="lv">{it.tier}</span>{/if}
                {#if it.amount > 1}<span class="amount">{it.amount}</span>{/if}
                {#if it.hearts > 0}<span class="hearts">♥{it.hearts}</span>{/if}
                {#if m > 0}<span class="match">{m}</span>{/if}
                {#if isEquipment(it) && gainOf(it) !== undefined}<span class="gain"><ImpactBadge value={gainOf(it)} /></span>{/if}
              </button>
            {:else}
              <span class="slot empty"></span>
            {/if}
          {:else}
            <p class="muted">{t('inv.none')}</p>
          {/each}
        </div>
      </div>

      <aside class="side scroll">
        <section>
          <div class="label">{t('inv.wanted')}</div>
          <p class="hint">{t('inv.wantedHint')}</p>
          <div class="chips">
            {#each wanted as tag (tag)}
              <button class="chip" onclick={() => (wanted = wanted.filter((x) => x !== tag))} title={t('common.remove')}>{attrName(tag)} ✕</button>
            {/each}
          </div>
          <input class="field" placeholder={t('inv.search')} bind:value={query} />
          {#if choices.length}
            <div class="choices">
              {#each choices as c (c.tag)}
                <button class="choice" onclick={() => ((wanted = [...wanted, c.tag]), (query = ''))}>{c.name}</button>
              {/each}
            </div>
          {/if}
          <div class="row">
            <button class="btn ghost" disabled={!Object.keys(planner.impacts).length} onclick={usePriorities} title={Object.keys(planner.impacts).length ? t('inv.prioritiesTitle') : t('common.analyzing')}>{t('inv.priorities')}</button>
            {#if wanted.length}<button class="btn ghost" onclick={() => (wanted = [])}>{t('inv.clear')}</button>{/if}
          </div>
          <label class="row">
            {t('inv.show')}
            <select class="field" bind:value={minMatch}>
              <option value={0}>{t('inv.showAll')}</option>
              <option value={1}>{t('inv.showMin', { n: 1 })}</option>
              <option value={2}>{t('inv.showMin', { n: 2 })}</option>
              <option value={3}>{t('inv.showMin', { n: 3 })}</option>
            </select>
          </label>
          <div class="legend"><span class="m1">1</span><span class="m2">2</span><span class="m3">3</span> {t('inv.legend')}</div>
        </section>

        <section>
          <div class="label">{t('inv.build')}</div>
          <ObjectiveSelect />
          <div class="row">
            <button class="btn" disabled={planner.evaluating || !planner.inventory.length} onclick={() => planner.evaluateInventory()}>
              {planner.evaluating ? t('common.analyzing') : evaluated ? t('inv.reevaluate') : t('inv.evaluate')}
            </button>
            <label><input type="checkbox" bind:checked={sortByGain} disabled={!evaluated} /> {t('inv.sortGain')}</label>
          </div>
          <p class="hint">{stale ? t('inv.stale') : t('inv.evaluateHint')}</p>
        </section>

        {#if sel}
          <section class="detail">
            <div class="dname">{sel.name}</div>
            <div class="muted">{slotLabel(sel)} · {rarityName(sel.rarity)} · {t('inv.tier', { n: sel.tier })}{sel.hearts ? ` · ♥${sel.hearts}` : ''}{sel.locked ? ` · ${t('inv.locked')}` : ''}</div>
            {#if sel.attributes.length}
              <ul class="attrs">
                {#each sel.attributes as a, k (k)}
                  <li class:want={wantedSet.has(a.tag)}><span>{attrName(a.tag)}</span><span class="v">{formatValue(a.value, info(a.tag))}</span></li>
                {/each}
              </ul>
            {/if}
            {#if sel.pools.some((p) => p.length)}
              <div class="label">{t('inv.pools')}</div>
              {#each sel.pools as pool, k (k)}
                {#if pool.length}
                  <div class="pool">
                    <span class="pn">{k + 1}</span>
                    {#each pool as r (r)}<span class="pa" class:want={wantedSet.has(tagOf(r))}>{attrName(tagOf(r))}</span>{/each}
                  </div>
                {/if}
              {/each}
              <p class="hint">{t('inv.rolled', { n: rolled(sel), m: matches(sel) })}</p>
            {/if}
            {#if isEquipment(sel)}
              <div class="row">
                {#if selGain}<span>{t('inv.gain')} <ImpactBadge value={planner.gainOf(selGain.impact)} /></span>{/if}
                <button class="btn" onclick={() => planner.tryItem(sel, sel.weapon ? null : (selGain?.slot ?? sel.slots[0]))}>{t('inv.try')}</button>
              </div>
            {/if}
          </section>
        {/if}
      </aside>
    </div>
  </div>
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
    min-width: 0;
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
  }
  .title {
    font-family: var(--font-title);
    font-size: 20px;
    color: var(--gold-bright);
  }
  .who {
    margin-left: 12px;
    color: var(--text-dim);
    font-size: 16px;
  }
  .actions,
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    font-family: var(--font-ui);
    font-size: 13px;
  }
  .tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
  }
  .tab {
    font-family: var(--font-ui);
    font-size: 13px;
    color: var(--text-dim);
    background: var(--panel-2);
    border: 1px solid var(--line);
    padding: 4px 10px;
    cursor: pointer;
  }
  .tab.on {
    color: var(--gold-bright);
    border-color: var(--gold);
  }
  .tab small {
    color: var(--text-faint);
  }
  .hits {
    margin-left: 4px;
    padding: 0 5px;
    border-radius: 8px;
    background: var(--gold-dim);
    color: var(--text);
    font-size: 11px;
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 14px;
  }
  .grid-wrap {
    flex: 1;
    min-width: 0;
    overflow: auto;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, 60px);
    /* Rows leave room for the gain badge under each item. */
    gap: 18px 8px;
    padding: 4px 4px 14px;
  }
  .slot {
    position: relative;
    width: 60px;
    height: 60px;
    padding: 0;
    border: 2px solid var(--frame, var(--line));
    background: #0e0d0a;
    cursor: pointer;
  }
  .slot.empty {
    border-color: var(--line);
    background: radial-gradient(circle, #23201a, #12100c);
    cursor: default;
  }
  .slot.dim {
    opacity: 0.35;
  }
  .slot.picked {
    outline: 2px solid var(--gold-bright);
    outline-offset: 2px;
  }
  /* Pools that can roll a wanted stat: blue, yellow, red (the game's magic, unique, flawless tints). */
  .slot.m1 {
    box-shadow: 0 0 0 2px #3b6fe0;
  }
  .slot.m2 {
    box-shadow: 0 0 0 2px #e8d55a;
  }
  .slot.m3 {
    box-shadow: 0 0 0 2px #e0472c;
  }
  .slot img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    pointer-events: none;
  }
  .slot .icon {
    object-fit: contain;
    padding: 4px;
  }
  .lv,
  .amount,
  .hearts,
  .match {
    position: absolute;
    font-family: var(--font-ui);
    font-size: 10px;
    line-height: 1;
    padding: 1px 2px;
    background: rgba(0, 0, 0, 0.7);
    color: var(--text);
  }
  .lv {
    top: 1px;
    left: 1px;
  }
  .amount {
    bottom: 1px;
    right: 1px;
  }
  .hearts {
    bottom: 1px;
    left: 1px;
    color: #f08ab0;
  }
  .match {
    top: 1px;
    right: 1px;
    font-weight: 700;
  }
  .m1 .match {
    color: #8fb3ff;
  }
  .m2 .match {
    color: #f3e27a;
  }
  .m3 .match {
    color: #ff8a70;
  }
  .gain {
    position: absolute;
    bottom: -15px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 2;
    white-space: nowrap;
  }
  .side {
    width: 340px;
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 10px;
    overflow-y: auto;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px 12px;
    background: var(--panel-2);
  }
  .label {
    font-family: var(--font-label);
    font-size: 12px;
    color: var(--gold);
  }
  .hint,
  .muted {
    margin: 0;
    font-family: var(--font-ui);
    font-size: 12px;
    color: var(--text-dim);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .chip,
  .choice {
    font-family: var(--font-ui);
    font-size: 12px;
    color: var(--text);
    background: var(--panel-3);
    border: 1px solid var(--line);
    padding: 2px 8px;
    cursor: pointer;
    text-align: left;
  }
  .chip {
    border-color: var(--gold-dim);
  }
  .choices {
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-height: 220px;
    overflow-y: auto;
  }
  .legend {
    display: flex;
    gap: 6px;
    align-items: center;
    font-family: var(--font-ui);
    font-size: 12px;
    color: var(--text-dim);
  }
  .legend span {
    width: 16px;
    text-align: center;
    font-weight: 700;
    border-radius: 2px;
  }
  .legend .m1 {
    box-shadow: 0 0 0 2px #3b6fe0;
  }
  .legend .m2 {
    box-shadow: 0 0 0 2px #e8d55a;
  }
  .legend .m3 {
    box-shadow: 0 0 0 2px #e0472c;
  }
  .dname {
    font-family: var(--font-title);
    font-size: 16px;
    color: var(--gold-bright);
  }
  .attrs {
    margin: 0;
    padding: 0;
    list-style: none;
    font-family: var(--font-ui);
    font-size: 13px;
  }
  .attrs li {
    display: flex;
    justify-content: space-between;
    gap: 8px;
  }
  .attrs .v {
    font-variant-numeric: tabular-nums;
  }
  .want {
    color: var(--good);
  }
  .pool {
    display: flex;
    flex-wrap: wrap;
    gap: 3px 8px;
    font-family: var(--font-ui);
    font-size: 12px;
    color: var(--text-dim);
  }
  .pn {
    color: var(--gold);
    font-weight: 700;
  }
  .pa.want {
    color: var(--good);
    font-weight: 700;
  }
  .saved {
    margin-left: auto;
    font-family: var(--font-ui);
    font-size: 12.5px;
    color: var(--text-dim);
  }
  .saved.old {
    color: var(--gold-bright);
  }
  .empty-note {
    margin: 0;
    font-family: var(--font-ui);
    font-size: 13px;
    color: var(--text-dim);
  }
</style>
