<script lang="ts">
  import { onMount } from 'svelte'
  import { formatValue } from '../lib/format'
  import { asset } from '../lib/game'
  import { planner } from '../lib/store.svelte'
  import { availableNodes, canRefund, optionLabel, ownedNodes, pathTo } from '../lib/tree'
  import type { TreeOption } from '../lib/types'
  import { t } from '../lib/i18n.svelte'
  import ImpactBadge from './ImpactBadge.svelte'
  import ObjectiveSelect from './ObjectiveSelect.svelte'
  import SkillSets from './SkillSets.svelte'

  const tab = $derived(planner.skillsTab)

  const tree = $derived(planner.catalog!.skill_tree)
  const nodes = $derived(tree.nodes)
  const edges = $derived(nodes.flatMap((n, i) => n.links.filter((j) => j > i).map((j) => [i, j] as const)))
  const rows = $derived(planner.treeRows())
  const owned = $derived(ownedNodes(tree, rows))
  const avail = $derived(availableNodes(tree, owned))
  const budget = $derived(planner.treeBudget())
  const dps = $derived(planner.stats?.dps)
  const left = $derived(budget - rows.length)

  const info = (tag: string) => planner.catalog?.attributes[tag]
  const bonusText = (o: TreeOption) => o.attributes.map(([t, v]) => `${formatValue(v, info(t))} ${info(t)?.name || t.split('.').pop()}`).join(' · ')
  const optionOf = (i: number) => nodes[i].options.find((o) => o.row === owned.get(i))
  const gainOf = (o: TreeOption | undefined, owned = false) => (o ? planner.optionImpact(o, owned) : undefined)
  /** Best option of a node for the objective (the first one before the analysis). */
  function bestOption(i: number): TreeOption | undefined {
    let best: TreeOption | undefined
    let g = -Infinity
    for (const o of nodes[i].options) {
      const g0 = gainOf(o)
      const v = Number.isFinite(g0) ? g0! : -1
      if (!best || v > g) [best, g] = [o, v]
    }
    return best
  }
  /** Option shown on a node: the owned one, else the best one. */
  const shown = (i: number) => optionOf(i) ?? bestOption(i)
  /** Gain of buying a node, or for an owned one what it brings (loss when refunded). */
  const gains = $derived(nodes.map((_, i) => (owned.has(i) ? gainOf(optionOf(i), true) : gainOf(bestOption(i)))))
  /** Gain with a number (not analysed and not quantified both give undefined). */
  const num = (i: number) => (Number.isFinite(gains[i]) ? gains[i] : undefined)
  const maxGain = $derived(Math.max(1e-9, ...gains.map((g) => (Number.isFinite(g) ? g! : 0))))

  let query = $state('')
  const norm = (s: string) => s.normalize('NFD').replace(/\p{Diacritic}/gu, '').toLowerCase()
  const matches = $derived.by(() => {
    const q = norm(query.trim())
    if (!q) return null
    const out = new Set<number>()
    nodes.forEach((n, i) => {
      if (n.options.some((o) => norm(`${o.name} ${o.description} ${bonusText(o)}`).includes(q))) out.add(i)
    })
    return out
  })

  let hovered = $state<number | null>(null)
  let choosing = $state<number | null>(null)
  let message = $state('')
  const focus = $derived(choosing ?? hovered)
  const hoverPath = $derived(hovered !== null && !owned.has(hovered) && !avail.has(hovered) ? pathTo(tree, owned, hovered) : null)
  const pathSet = $derived(new Set(hoverPath ?? []))

  const setRows = (next: string[]) => {
    planner.tree = next
    message = ''
  }
  function buy(i: number, row?: string) {
    const r = row ?? (nodes[i].options.length === 1 ? nodes[i].options[0].row : undefined)
    if (!r) return void (choosing = i)
    const cur = owned.get(i)
    if (cur) return setRows(rows.map((x) => (x === cur ? r : x)))
    if (avail.has(i)) return setRows([...rows, r])
    const path = pathTo(tree, owned, i)
    if (!path) return
    // Nodes on the way take their best option for the objective.
    setRows([...rows, ...path.slice(0, -1).map((j) => bestOption(j)!.row), r])
  }
  function refund(i: number) {
    if (!canRefund(tree, owned, i)) return void (message = t('tree.cutWarning'))
    setRows(rows.filter((x) => x !== owned.get(i)))
    if (choosing === i) choosing = null
  }
  function click(i: number) {
    if (moved || nodes[i].root) return
    if (nodes[i].options.length > 1) return void (choosing = choosing === i ? null : i)
    if (owned.has(i)) refund(i)
    else buy(i)
  }

  /** Nodes worth buying next, and owned nodes that give the least (refund candidates). */
  const bestNext = $derived(
    [...avail]
      .map((i) => ({ i, g: num(i) ?? 0 }))
      .filter((x) => x.g > 1e-7)
      .sort((a, b) => b.g - a.g)
      .slice(0, 10),
  )
  const weakest = $derived(
    [...owned.keys()]
      .filter((i) => !nodes[i].root && canRefund(tree, owned, i) && num(i) !== undefined)
      .map((i) => ({ i, g: num(i)! }))
      .sort((a, b) => a.g - b.g)
      .slice(0, 8),
  )

  // View: pan with drag, zoom with the wheel (around the cursor).
  let vw = $state(1000)
  let vh = $state(800)
  let scale = $state(0.15)
  let tx = $state(0)
  let ty = $state(0)
  let moved = false
  let drag: { x: number; y: number; tx: number; ty: number } | null = null
  function fit() {
    const xs = nodes.flatMap((n) => [n.x, n.x + n.w])
    const ys = nodes.flatMap((n) => [n.y, n.y + n.h])
    const [x0, x1, y0, y1] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)]
    scale = Math.min(vw / (x1 - x0), vh / (y1 - y0)) * 0.95
    tx = vw / 2 - ((x0 + x1) / 2) * scale
    ty = vh / 2 - ((y0 + y1) / 2) * scale
  }
  function wheel(e: WheelEvent) {
    e.preventDefault()
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
    const [cx, cy] = [e.clientX - r.left, e.clientY - r.top]
    const next = Math.min(3, Math.max(0.05, scale * Math.exp(-e.deltaY * 0.0015)))
    tx = cx - ((cx - tx) * next) / scale
    ty = cy - ((cy - ty) * next) / scale
    scale = next
  }
  function down(e: PointerEvent) {
    drag = { x: e.clientX, y: e.clientY, tx, ty }
    moved = false
  }
  function move(e: PointerEvent) {
    if (!drag) return
    const [dx, dy] = [e.clientX - drag.x, e.clientY - drag.y]
    if (Math.abs(dx) + Math.abs(dy) > 4) moved = true
    tx = drag.tx + dx
    ty = drag.ty + dy
  }
  const up = () => (drag = null)
  onMount(() => requestAnimationFrame(fit))

  const heat = (i: number) => {
    const g = num(i)
    if (g === undefined || g <= 1e-7) return null
    return `rgba(127, 230, 100, ${(0.25 + 0.75 * Math.sqrt(g / maxGain)).toFixed(2)})`
  }
  const dim = (i: number) => (planner.hideUseless && !owned.has(i) && num(i) !== undefined && Math.abs(num(i)!) < 1e-7) || (matches !== null && !matches.has(i))
  const pct = (g: number | undefined) => (g === undefined ? '' : `${g > 0 ? '+' : ''}${(g * 100).toFixed(Math.abs(g) >= 0.001 ? 2 : 3)}%`)

</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && (choosing !== null ? (choosing = null) : (planner.treeOpen = false))} onpointerup={up} />

<div class="overlay">
  {#if tab === 'skills'}<SkillSets />{:else}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="canvas" bind:clientWidth={vw} bind:clientHeight={vh} onwheel={wheel} onpointerdown={down} onpointermove={move}>
    <svg width={vw} height={vh}>
      <defs>
        <clipPath id="node-clip" clipPathUnits="objectBoundingBox"><circle cx="0.5" cy="0.5" r="0.5" /></clipPath>
      </defs>
      <g transform="translate({tx} {ty}) scale({scale})">
        {#each edges as [a, b] (a * 100000 + b)}
          {@const na = nodes[a]}
          {@const nb = nodes[b]}
          <line
            x1={na.x + na.w / 2}
            y1={na.y + na.h / 2}
            x2={nb.x + nb.w / 2}
            y2={nb.y + nb.h / 2}
            class:on={owned.has(a) && owned.has(b)}
            class:path={(pathSet.has(a) || owned.has(a)) && (pathSet.has(b) || owned.has(b)) && (pathSet.has(a) || pathSet.has(b))}
          />
        {/each}
        {#each nodes as n, i (n.id)}
          {@const o = shown(i)}
          {@const r = n.w / 2}
          {@const h = heat(i)}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <g
            class="node"
            class:owned={owned.has(i)}
            class:avail={avail.has(i)}
            class:path={pathSet.has(i)}
            class:match={matches?.has(i)}
            class:dim={dim(i)}
            class:sel={choosing === i}
            transform="translate({n.x + r} {n.y + n.h / 2})"
            role="button"
            tabindex="-1"
            onclick={() => click(i)}
            onmouseenter={() => (hovered = i)}
            onmouseleave={() => hovered === i && (hovered = null)}
          >
            {#if h && !owned.has(i)}<circle class="glow" r={r + 6} style:stroke={h} />{/if}
            <circle class="disc" {r} />
            {#if o?.icon && n.w * scale >= 10}
              <image href={asset(o.icon)} x={-r * 0.8} y={-r * 0.8} width={r * 1.6} height={r * 1.6} clip-path="url(#node-clip)" />
            {/if}
            <circle class="ring" {r} />
            {#if n.options.length > 1 && n.w * scale >= 8}<circle class="multi" r={r * 0.22} cx={r * 0.72} cy={-r * 0.72} />{/if}
          </g>
        {/each}
      </g>
    </svg>
    <div class="hint">{t('tree.hint')}</div>
  </div>
  {/if}

  <aside class="side frame">
    <header>
      <span class="title">{t('tree.title')}</span>
      <button class="btn ghost" title="Fermer (Échap)" onclick={() => (planner.treeOpen = false)}>✕</button>
    </header>
    <div class="tabs">
      <button class="btn" class:on={tab === 'tree'} onclick={() => (planner.skillsTab = 'tree')}>{t('tree.tabTree')}</button>
      <button class="btn" class:on={tab === 'skills'} onclick={() => (planner.skillsTab = 'skills')}>{t('tree.tabSkills')}</button>
    </div>
    {#if tab === 'tree'}
      <div class="points" class:over={left < 0}>
        <span class="big">{rows.length}</span> {t('tree.budget', { max: budget })}
        <span class="rest">{left >= 0 ? t('tree.left', { n: left }) : t('tree.over', { n: -left })}</span>
      </div>
    {/if}
    <label class="obj">
      {t('tree.gainOn')}
      <ObjectiveSelect />
      {#if planner.treeAnalyzing}<span class="spin" title={t('tree.analyzing')}>◌</span>{/if}
    </label>
    {#if tab === 'tree'}
    <div class="actions">
      <button class="btn" disabled={planner.tree === null} onclick={() => setRows([...(planner.character?.tree ?? [])])}>{t('tree.saveTree')}</button>
      <button class="btn" onclick={() => setRows([])}>{t('tree.removeAll')}</button>
      <button class="btn ghost" onclick={fit}>{t('tree.recenter')}</button>
    </div>
    <input class="field" placeholder={t('tree.search')} bind:value={query} />
    <label class="check"><input type="checkbox" bind:checked={planner.hideUseless} /> {t('tree.dim')}</label>
    {#if message}<p class="msg">{message}</p>{/if}
    {/if}
    {#if dps && dps.offhand_crit_chance > 0}
      <p class="crit" class:low={dps.crit_total < 10}>
        {t('tree.critNote', { t: (dps.crit_total * 100).toFixed(1), o: (dps.offhand_crit_chance * 100).toFixed(1) })}
        {dps.crit_total >= 10 ? t('tree.critMargin', { m: ((dps.crit_total - 10) * 100).toFixed(1) }) : t('tree.critMissing', { m: ((10 - dps.crit_total) * 100).toFixed(1) })}
      </p>
    {/if}

    {#if tab === 'tree'}
    <div class="scroll body">
      {#if focus !== null}
        {@const n = nodes[focus]}
        <div class="detail">
          {#if choosing === focus}
            <div class="label">{t('tree.chooseOption')}</div>
            {#each n.options as opt (opt.row)}
              <button class="choice" class:cur={owned.get(focus) === opt.row} onclick={() => (buy(focus, opt.row), (choosing = null))}>
                {#if opt.icon}<img src={asset(opt.icon)} alt="" />{/if}
                <span class="cname">{optionLabel(opt)}<small>{bonusText(opt)}</small></span>
                <ImpactBadge value={gainOf(opt, owned.get(focus) === opt.row)} title={t('tree.optionGainTitle')} />
              </button>
            {/each}
            {#if owned.has(focus)}<button class="btn ghost wide" onclick={() => refund(focus)}>{t('tree.removeNode')}</button>{/if}
          {:else}
            {@const o = shown(focus)}
            <div class="dname">{n.root ? t('tree.start') : o ? optionLabel(o) : n.id}</div>
            {#if o}<div class="bonus">{bonusText(o)}</div>{/if}
            {#if o?.description}<div class="desc">{o.description}</div>{/if}
            <div class="state">
              {#if n.root}{t('tree.alwaysOwned')}
              {:else if owned.has(focus)}{t('tree.owned')}{#if !canRefund(tree, owned, focus)}{t('tree.cutsTree')}{/if}
              {:else if avail.has(focus)}{t('tree.available')}
              {:else if hoverPath}{t('tree.path', { n: hoverPath.length })}
              {/if}
              {#if n.options.length > 1}{t('tree.options', { n: n.options.length })}{/if}
              <ImpactBadge value={gains[focus]} title={t('tree.nodeGainTitle')} />
            </div>
          {/if}
        </div>
      {/if}

      <div class="label">{t('tree.bestNext')}</div>
      {#each bestNext as { i, g } (i)}
        {@const o = shown(i)}
        <button class="choice" onmouseenter={() => (hovered = i)} onclick={() => (nodes[i].options.length > 1 ? (choosing = i) : buy(i))}>
          {#if o?.icon}<img src={asset(o.icon)} alt="" />{/if}
          <span class="cname">{o ? optionLabel(o) : nodes[i].id}<small>{o ? bonusText(o) : ''}</small></span>
          <span class="g good">{pct(g)}</span>
        </button>
      {:else}
        <p class="none">{planner.treeAnalyzing || !Object.keys(planner.treeImpacts).length ? t('common.analyzing') : t('tree.noneNext')}</p>
      {/each}

      <div class="label">{t('tree.weakest')}</div>
      {#each weakest as { i, g } (i)}
        {@const o = shown(i)}
        <button class="choice" onmouseenter={() => (hovered = i)} onclick={() => refund(i)} title={t('tree.clickRemove')}>
          {#if o?.icon}<img src={asset(o.icon)} alt="" />{/if}
          <span class="cname">{o ? optionLabel(o) : nodes[i].id}<small>{o ? bonusText(o) : ''}</small></span>
          <span class="g">{pct(g)}</span>
        </button>
      {/each}
      <p class="fine">{t('tree.fine')}</p>
    </div>
    {/if}
  </aside>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 600;
    display: grid;
    grid-template-columns: minmax(0, 1fr) 340px;
    background: radial-gradient(ellipse at center, #1a1712 0%, #0a0907 75%);
  }
  .canvas {
    position: relative;
    overflow: hidden;
    cursor: grab;
    touch-action: none;
  }
  .canvas:active {
    cursor: grabbing;
  }
  svg {
    display: block;
  }
  line {
    stroke: #3b342a;
    stroke-width: 4;
  }
  line.on {
    stroke: var(--gold);
    stroke-width: 6;
  }
  line.path {
    stroke: #9be07f;
    stroke-dasharray: 10 8;
    stroke-width: 6;
  }
  .node {
    cursor: pointer;
  }
  .disc {
    fill: #15130f;
  }
  .ring {
    fill: none;
    stroke: #4a4134;
    stroke-width: 3;
  }
  .glow {
    fill: none;
    stroke-width: 5;
  }
  .node image {
    opacity: 0.35;
  }
  .node.avail .ring {
    stroke: #a08a55;
  }
  .node.avail image {
    opacity: 0.75;
  }
  .node.owned .ring {
    stroke: var(--gold-bright);
    stroke-width: 5;
  }
  .node.owned .disc {
    fill: #3a2f16;
  }
  .node.owned image {
    opacity: 1;
  }
  .node.path .ring {
    stroke: #9be07f;
    stroke-width: 5;
  }
  .node.match .ring {
    stroke: #7fc8ff;
    stroke-width: 6;
  }
  .node.sel .ring {
    stroke: #fff;
  }
  .node.dim {
    opacity: 0.25;
  }
  .node:hover .ring {
    stroke: #fff;
  }
  .multi {
    fill: #7fc8ff;
  }
  .hint {
    position: absolute;
    left: 12px;
    bottom: 10px;
    font-family: var(--font-ui);
    font-size: 12px;
    color: var(--text-faint);
    pointer-events: none;
  }
  .side {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
    min-height: 0;
    background: var(--panel);
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  header .title {
    font-size: 19px;
  }
  .tabs {
    display: flex;
    gap: 6px;
  }
  .tabs .btn {
    flex: 1;
    opacity: 0.6;
  }
  .tabs .btn.on {
    opacity: 1;
    border-color: var(--gold-bright);
  }
  .points {
    font-family: var(--font-ui);
    color: var(--text-dim);
  }
  .points .big {
    font-family: var(--font-title);
    font-size: 26px;
    color: var(--gold-bright);
  }
  .points .rest {
    margin-left: 8px;
    font-size: 13px;
  }
  .points.over .big,
  .points.over .rest {
    color: var(--bad);
  }
  .obj {
    display: flex;
    align-items: center;
    gap: 8px;
    font-family: var(--font-ui);
    font-size: 13px;
    color: var(--text-dim);
  }
  .obj :global(select) {
    flex: 1;
    min-width: 0;
  }
  .spin {
    margin-left: 6px;
    display: inline-block;
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .actions {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 6px;
    font-family: var(--font-ui);
    font-size: 13px;
    color: var(--text-dim);
  }
  .crit {
    margin: 0;
    font-family: var(--font-ui);
    font-size: 12.5px;
    color: #f0c46b;
  }
  .crit.low {
    color: var(--bad);
  }
  .msg {
    margin: 0;
    font-family: var(--font-ui);
    font-size: 13px;
    color: var(--bad);
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding-right: 4px;
  }
  .label {
    font-size: 11px;
    margin: 10px 0 2px;
  }
  .detail {
    border: 1px solid var(--gold-dim);
    background: rgba(201, 162, 74, 0.06);
    padding: 8px 10px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .detail .label {
    margin-top: 0;
  }
  .dname {
    font-family: var(--font-title);
    font-size: 17px;
    color: var(--gold-bright);
  }
  .bonus {
    font-family: var(--font-ui);
    color: var(--good);
  }
  .desc {
    font-size: 13.5px;
    color: var(--text-dim);
  }
  .state {
    font-family: var(--font-ui);
    font-size: 12.5px;
    color: var(--text-dim);
    display: flex;
    gap: 6px;
    align-items: center;
    flex-wrap: wrap;
  }
  .choice {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 4px 6px;
    background: none;
    border: 1px solid transparent;
    color: var(--text);
    text-align: left;
    cursor: pointer;
  }
  .choice:hover {
    background: rgba(201, 162, 74, 0.08);
  }
  .choice.cur {
    border-color: var(--gold);
  }
  .choice img {
    width: 26px;
    height: 26px;
    border-radius: 50%;
  }
  .cname {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    font-size: 14px;
  }
  .cname small {
    font-family: var(--font-ui);
    font-size: 11.5px;
    color: var(--text-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .g {
    font-family: var(--font-ui);
    font-size: 12.5px;
    font-variant-numeric: tabular-nums;
    color: var(--text-dim);
  }
  .g.good {
    color: var(--good);
  }
  .wide {
    width: 100%;
  }
  .none,
  .fine {
    font-family: var(--font-ui);
    font-size: 12px;
    color: var(--text-faint);
    margin: 4px 0;
  }
</style>
