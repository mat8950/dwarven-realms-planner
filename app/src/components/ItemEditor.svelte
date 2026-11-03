<script lang="ts">
  import { MAX_HEARTS, heartFactor, successChance } from '../lib/forge'
  import { formatValue, fromDisplay, toDisplay } from '../lib/format'
  import { PET, PET_SLOT, RARITY, abilityOfMod, asset, isMod } from '../lib/game'
  import { rarityName, slotName as slotLabel, stanceName as stanceLabel, t } from '../lib/i18n.svelte'
  import { planner } from '../lib/store.svelte'
  import ImpactBadge from './ImpactBadge.svelte'

  const sel = $derived(planner.selected)
  const item = $derived(planner.item(sel))
  const swapped = $derived(sel === 'weapon' ? planner.weaponInfo() : null)
  const attrs = $derived(planner.effAttrs(sel))
  const indexed = $derived(attrs.map((a, i) => ({ ...a, i })))
  const affixes = $derived(indexed.filter((a) => !isMod(a.tag)))
  const mods = $derived(indexed.filter((a) => isMod(a.tag)))
  const info = (tag: string) => planner.catalog?.attributes[tag]
  const look = $derived(item ? (sel === PET_SLOT ? PET : (RARITY[item.rarity] ?? RARITY[0])) : null)
  const slotName = $derived(
    sel === 'weapon' ? t('slot.weapon') : sel === null ? '' : slotLabel(planner.catalog?.slots.find((s) => s.value === sel)?.name ?? ''),
  )
  const h0 = $derived(planner.originalHearts(sel))
  const hearts = $derived(planner.heartsOf(sel))

  let picking = $state<'affix' | 'mod' | 'weapon' | null>(null)
  let search = $state('')
  // A picker belongs to the item it was opened on.
  $effect(() => {
    void sel
    picking = null
    search = ''
  })
  const reveal = (node: HTMLElement) => {
    node.scrollIntoView({ block: 'nearest', behavior: 'smooth' })
  }
  const norm = (s: string) => s.normalize('NFD').replace(/\p{Diacritic}/gu, '').toLowerCase()
  const open = (p: typeof picking) => {
    picking = picking === p ? null : p
    search = ''
    // Gains at this item's roll (diminishing returns: cooldown, caps...), not a typical one.
    if (picking === 'affix') void planner.analyzePicker(affixChoices.map((c) => [c.tag, c.value] as [string, number]))
    if (picking === 'mod') void planner.analyzePicker(modChoices.map((c) => [c.tag, 1] as [string, number]))
  }
  const score = (c: { tag: string; value: number }) => planner.addGain(c.tag, c.value) ?? -1

  /** Rollable affixes, most useful for the current objective first. */
  const affixChoices = $derived.by(() => {
    const seen = new Set<string>()
    const out: { tag: string; name: string; value: number }[] = []
    for (const a of planner.catalog?.affixes ?? []) {
      if (!a.can_roll || seen.has(a.tag) || isMod(a.tag) || attrs.some((x) => x.tag === a.tag)) continue
      seen.add(a.tag)
      out.push({ tag: a.tag, name: info(a.tag)?.name || a.tag.split('.').slice(-2).join(' '), value: a.value + a.per_level * Math.max(item?.tier ?? 0, 1) })
    }
    const q = norm(search)
    return out
      .filter((c) => !q || norm(c.name).includes(q) || norm(c.tag).includes(q))
      .filter((c) => !planner.hideUseless || !planner.useless(c.tag))
      .sort((a, b) => score(b) - score(a) || a.name.localeCompare(b.name))
  })

  /** The game picks an item's mods by the item's type (`Get ALL Mod Tags for Equipment Slot`), not by
   *  where it is worn: a ring takes the ring mods in either ring slot (they are listed under Ring1). */
  const RING_SLOTS = [6, 7]
  const modSlots = (s: number | null) => (s !== null && RING_SLOTS.includes(s) ? RING_SLOTS : [s])

  /** Equipment modifiers (monograms) allowed on this slot, plus modifiers of the offhand
   *  abilities this item already carries. */
  const modChoices = $derived.by(() => {
    const q = norm(search)
    const equipment = (planner.catalog?.modifiers ?? [])
      .filter((m) => m.in_use && (sel === 'weapon' ? m.weapon_affinity > 0 : modSlots(sel).includes(m.slot)))
      .map((m) => ({ key: m.row, tag: m.tag, text: m.description || m.name || m.tag }))
    const abilities = new Set(attrs.map((a) => abilityOfMod(a.tag) ?? (a.tag.startsWith('EasyRPG.Attributes.Abilities.') ? a.tag.split('.').slice(0, 4).join('.') : null)).filter(Boolean))
    const abilityMods = Object.entries(planner.catalog?.attributes ?? {})
      .filter(([t]) => abilities.has(abilityOfMod(t) ?? ''))
      .map(([t, a]) => ({ key: t, tag: t, text: a.description || a.name || t.split('.').pop() || t }))
    const gain = (tag: string) => {
      const g = planner.addGain(tag, 1)
      return g === undefined || Number.isNaN(g) ? -1 : g
    }
    // Several modifier rows can share a tag (and a text): one entry per tag.
    const seen = new Set<string>()
    return [...abilityMods, ...equipment]
      .filter((m) => !seen.has(m.tag) && seen.add(m.tag) && (!q || norm(m.text).includes(q) || norm(m.tag).includes(q)))
      .sort((a, b) => gain(b.tag) - gain(a.tag))
  })

  /** Weapons of the game, same stance first. */
  const weaponChoices = $derived.by(() => {
    const q = norm(search)
    const stance = planner.stats?.stance
    return (planner.catalog?.weapons ?? [])
      .filter((w) => w.name && (!q || norm(w.name).includes(q) || norm(w.row).includes(q)))
      .sort((a, b) => Number(b.stance === stance) - Number(a.stance === stance) || a.stance - b.stance || a.name.localeCompare(b.name))
      .slice(0, 200)
  })
  /** Applies a number field; an empty or invalid entry puts the shown value back. */
  function onNumber(e: Event & { currentTarget: HTMLInputElement }, shown: number, apply: (v: number) => void) {
    const v = parseFloat(e.currentTarget.value)
    if (Number.isFinite(v)) apply(v)
    else e.currentTarget.value = String(shown)
  }
  const stanceName = (v: number) => {
    const n = planner.catalog?.stances.find(([x]) => x === v)?.[1] ?? ''
    return stanceLabel(n)
  }
</script>

<section class="editor frame">
  {#if item && look}
    <header>
      <div class="ico" style:--frame={look.frame}>
        <img src={asset(look.bg)} alt="" />
        {#if swapped?.icon ?? item.icon}<img class="fg" src={asset(swapped?.icon ?? item.icon)} alt="" />{/if}
      </div>
      <div class="head">
        <div class="slotname label">{slotName}</div>
        <div class="name title" style:color={look.frame}>{swapped?.name ?? item.name}</div>
        <div class="sub">{sel === PET_SLOT ? t('item.pet') : rarityName(item.rarity)}{item.tier ? ` · ${t('item.level', { n: item.tier })}` : ''}{swapped ? ` · ${stanceName(swapped.stance)}` : ''}</div>
      </div>
      <button class="btn ghost close" title={t('common.close')} onclick={() => (planner.selected = null)}>✕</button>
    </header>

    <div class="body scroll">
      {#if sel === 'weapon'}
        <button class="btn add" onclick={() => open('weapon')}>{picking === 'weapon' ? t('editor.closeList') : t('editor.swapWeapon')}</button>
        {#if picking === 'weapon'}
          <div class="picker" use:reveal>
            <!-- svelte-ignore a11y_autofocus -->
            <input class="field" placeholder={t('editor.searchWeapon')} bind:value={search} autofocus />
            <div class="choices scroll">
              {#each weaponChoices as w (w.row)}
                <button class="choice" onclick={() => ((planner.weaponRow = w.row === planner.character?.weapon?.row ? null : w.row), (planner.stance = null), (picking = null))}>
                  {#if w.icon}<img class="wicon" src={asset(w.icon)} alt="" />{/if}
                  <span>{w.name}</span>
                  <span class="hint">{stanceName(w.stance)}</span>
                </button>
              {/each}
            </div>
            <p class="note">{t('editor.swapNote')}</p>
          </div>
        {/if}
      {/if}

      {#if sel !== PET_SLOT}
        <h3 class="label">{t('editor.forge')} <span class="count">{hearts}/{MAX_HEARTS}</span></h3>
        <div class="hearts">
          {#each Array(MAX_HEARTS) as _, i (i)}
            <button
              class="heart"
              class:on={i < hearts}
              class:orig={i < h0}
              title={t('editor.heart', { n: i + 1 })}
              onclick={() => planner.setHearts(sel, i + 1 === hearts ? i : i + 1)}
            >
              <img src={asset(i < hearts ? 'EasySurvivalRPG/Textures/New_Icons/dwarven-heart.webp' : 'EasySurvivalRPG/Textures/UI/heart-of-dwarf-slot-empty.webp')} alt="" />
            </button>
          {/each}
        </div>
        <div class="forge-info">
          <span>{t('editor.forgeFactor', { f: heartFactor(0, hearts).toFixed(3) })}</span>
          {#if hearts > h0}
            <span class="risk">{t('editor.success', { from: h0, to: hearts, p: (successChance(h0, hearts) * 100).toFixed(hearts - h0 > 3 ? 2 : 1) })}</span>
          {:else if hearts < h0}
            <span class="hint">{t('editor.without', { n: h0 - hearts })}</span>
          {/if}
        </div>
      {/if}

      <h3 class="label">{t('editor.affixes')} <span class="count">{affixes.length}</span></h3>
      {#each affixes as a (a.i)}
        {@const i = info(a.tag)}
        <div class="affix" class:dim={Math.abs(planner.contribution(a.tag, a.value) ?? 1) < 1e-7}>
          {#if i?.icon}<img src={asset(i.icon)} alt="" />{:else}<span class="noicon"></span>{/if}
          <span class="aname" title={a.tag}>{i?.name || a.tag.split('.').pop()}</span>
          <ImpactBadge value={planner.contribution(a.tag, a.value)} title={t('editor.affixContrib', { obj: planner.objectiveLabel() })} />
          <input
            class="field num"
            type="number"
            step="any"
            value={+toDisplay(a.value, i).toFixed(4)}
            onchange={(e) => onNumber(e, +toDisplay(a.value, i).toFixed(4), (v) => planner.setEffValue(sel, a.i, fromDisplay(v, i)))}
          />
          <span class="unit">{i?.value_type === 2 ? '%' : ''}</span>
          <button class="btn ghost x" title={t('common.remove')} onclick={() => planner.removeAttr(sel, a.i)}>✕</button>
        </div>
      {/each}

      {#if picking === 'affix'}
        <div class="picker" use:reveal>
          <!-- svelte-ignore a11y_autofocus -->
          <input class="field" placeholder={t('editor.searchAffix', { obj: planner.objectiveLabel() })} bind:value={search} autofocus />
          <div class="choices scroll">
            {#each affixChoices as c (c.tag)}
              <button class="choice" class:dim={planner.useless(c.tag)} onclick={() => (planner.addAttr(sel, c.tag, c.value), (picking = null))}>
                {#if info(c.tag)?.icon}<img src={asset(info(c.tag)?.icon)} alt="" />{/if}
                <span>{c.name}</span>
                <ImpactBadge value={planner.addGain(c.tag, c.value)} title={t('editor.rollGain', { v: formatValue(c.value, info(c.tag)), obj: planner.objectiveLabel() })} />
                <span class="hint">{formatValue(c.value, info(c.tag))}</span>
              </button>
            {/each}
          </div>
        </div>
      {:else}
        <button class="btn add" onclick={() => open('affix')}>{t('editor.addAffix')}</button>
      {/if}

      <h3 class="label">{t('editor.mods')} <span class="count">{mods.length}</span></h3>
      {#each mods as m (m.i)}
        <div class="mod">
          <div class="mtext">{info(m.tag)?.description || info(m.tag)?.name || m.tag.replace('EasyRPG.Items.Modifiers.', '')}</div>
          <div class="mctl">
            <ImpactBadge value={planner.contribution(m.tag, m.value)} title={t('editor.modContrib', { obj: planner.objectiveLabel() })} cond={planner.condition(m.tag)?.label} unq={planner.unquantifiedKind(m.tag)} />
            <span class="stack" title={t('editor.stack')}>×</span>
            <input class="field num small" type="number" min="1" step="1" value={m.value} onchange={(e) => onNumber(e, m.value, (v) => planner.setEffValue(sel, m.i, v))} />
            <button class="btn ghost x" title={t('common.remove')} onclick={() => planner.removeAttr(sel, m.i)}>✕</button>
          </div>
        </div>
      {/each}

      {#if picking === 'mod'}
        <div class="picker" use:reveal>
          <!-- svelte-ignore a11y_autofocus -->
          <input class="field" placeholder={t('editor.searchMod')} bind:value={search} autofocus />
          <div class="choices scroll">
            {#each modChoices as m (m.key)}
              <button class="choice mod-choice" onclick={() => (planner.addAttr(sel, m.tag, 1), (picking = null))}>
                <span class="mtxt">{m.text}</span>
                <ImpactBadge value={planner.addGain(m.tag, 1)} title={t('editor.modGain', { obj: planner.objectiveLabel() })} cond={planner.condition(m.tag)?.label} unq={planner.unquantifiedKind(m.tag)} />
              </button>
            {:else}
              <div class="none">{t('editor.noMod')}</div>
            {/each}
          </div>
        </div>
      {:else}
        <button class="btn add" onclick={() => open('mod')}>{t('editor.addMod')}</button>
      {/if}
    </div>

    <footer>
      <button class="btn" disabled={!planner.isEdited(sel)} onclick={() => planner.resetItem(sel)}>{t('editor.resetItem')}</button>
    </footer>
  {:else}
    <div class="empty">
      <div class="title">{t('editor.emptyTitle')}</div>
      <p>{t('editor.emptyText')}</p>
    </div>
  {/if}
</section>

<style>
  .editor {
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 14px 16px;
  }
  header {
    display: flex;
    gap: 12px;
    align-items: center;
    padding-bottom: 10px;
    border-bottom: 1px solid var(--line);
  }
  .ico {
    position: relative;
    width: 64px;
    height: 64px;
    flex: none;
    border: 3px solid var(--frame);
  }
  .ico img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .ico .fg {
    object-fit: contain;
    padding: 3px;
  }
  .head {
    flex: 1;
    min-width: 0;
  }
  .slotname {
    font-size: 11px;
  }
  .name {
    font-size: 20px;
    line-height: 1.15;
    text-transform: none;
    letter-spacing: 0.04em;
    filter: brightness(1.15);
  }
  .sub {
    color: var(--text-dim);
    font-size: 14px;
  }
  .close {
    align-self: flex-start;
  }
  .body {
    flex: 1;
    min-height: 0;
    padding: 4px 4px 4px 0;
  }
  h3 {
    font-size: 13px;
    margin: 14px 0 6px;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .count {
    color: var(--text-faint);
    font-family: var(--font-ui);
  }
  .hearts {
    display: flex;
    gap: 3px;
  }
  .heart {
    width: 30px;
    height: 30px;
    padding: 0;
    border: none;
    background: none;
    cursor: pointer;
    opacity: 0.55;
    transition: transform 0.08s ease, opacity 0.1s;
  }
  .heart.on {
    opacity: 1;
  }
  .heart.orig.on {
    filter: drop-shadow(0 0 4px rgba(236, 198, 107, 0.6));
  }
  .heart:hover {
    transform: scale(1.12);
    opacity: 1;
  }
  .heart img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
  .forge-info {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    font-family: var(--font-ui);
    font-size: 13px;
    color: var(--text-dim);
    margin-top: 4px;
  }
  .risk {
    color: #f0b46b;
  }
  .affix {
    display: grid;
    grid-template-columns: 22px minmax(0, 1fr) auto 104px 14px 26px;
    align-items: center;
    gap: 8px;
    padding: 3px 0;
  }
  .dim {
    opacity: 0.45;
  }
  .affix img,
  .noicon,
  .choice img {
    width: 22px;
    height: 22px;
    border-radius: 50%;
  }
  .choice img.wicon {
    width: 30px;
    height: 30px;
    border-radius: 2px;
  }
  .aname {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .num {
    width: 100%;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .num.small {
    width: 56px;
  }
  .unit {
    color: var(--text-dim);
    font-family: var(--font-ui);
  }
  .x {
    padding: 2px 6px;
  }
  .add {
    margin-top: 6px;
    width: 100%;
  }
  .mod {
    display: flex;
    gap: 10px;
    align-items: center;
    padding: 6px 8px;
    margin: 4px 0;
    background: linear-gradient(90deg, rgba(168, 60, 200, 0.12), transparent);
    border-left: 2px solid rgba(190, 90, 220, 0.6);
  }
  .mtext {
    flex: 1;
    font-size: 14px;
    font-style: italic;
    color: #dcc9ee;
  }
  .mctl {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .stack {
    color: var(--text-dim);
  }
  .picker {
    margin-top: 6px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .choices {
    max-height: 300px;
    border: 1px solid var(--line);
    background: #12100c;
  }
  .choice {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 5px 8px;
    background: none;
    border: none;
    border-bottom: 1px solid #1d1a14;
    text-align: left;
    cursor: pointer;
  }
  .choice:hover {
    background: rgba(201, 162, 74, 0.1);
  }
  .choice span:first-of-type {
    flex: 1;
  }
  .hint {
    color: var(--text-faint);
    font-size: 13px;
    font-family: var(--font-ui);
  }
  .note {
    margin: 0;
    font-size: 12px;
    color: var(--text-dim);
  }
  .mod-choice {
    display: flex;
    align-items: center;
    gap: 8px;
    font-style: italic;
    font-size: 14px;
  }
  .mod-choice .mtxt {
    flex: 1;
  }
  .none {
    padding: 10px;
    color: var(--text-dim);
  }
  footer {
    padding-top: 10px;
    border-top: 1px solid var(--line);
    display: flex;
    justify-content: flex-end;
  }
  .empty {
    margin: auto;
    text-align: center;
    max-width: 300px;
    color: var(--text-dim);
  }
  .empty .title {
    font-size: 20px;
    margin-bottom: 8px;
  }
</style>
