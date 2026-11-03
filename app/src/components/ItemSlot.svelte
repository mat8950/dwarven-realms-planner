<script lang="ts">
  import { formatValue } from '../lib/format'
  import { PET, PET_SLOT, RARITY, asset, isMod } from '../lib/game'
  import { rarityName, t } from '../lib/i18n.svelte'
  import { planner, type Selection } from '../lib/store.svelte'
  import type { ItemView, SlotInfo } from '../lib/types'

  let { sel, item, slot, size = 84, tipLeft = false }: { sel: Selection; item: ItemView | null; slot?: SlotInfo; size?: number; tipLeft?: boolean } = $props()

  const isPet = $derived(sel === PET_SLOT)
  const look = $derived(item?.row ? (isPet ? PET : (RARITY[item.rarity] ?? RARITY[0])) : null)
  const selected = $derived(planner.selected === sel)
  const edited = $derived(planner.isEdited(sel))
  let hover = $state(false)

  const attrs = $derived(planner.effAttrs(sel))
  const hearts = $derived(planner.heartsOf(sel))
  const swapped = $derived(sel === 'weapon' ? planner.weaponInfo() : null)
  const icon = $derived(swapped?.icon ?? item?.icon)
  const name = $derived(swapped?.name ?? item?.name ?? '')
  const info = (tag: string) => planner.catalog?.attributes[tag]
</script>

<button
  class="slot"
  class:selected
  class:empty={!item?.row}
  style:--size="{size}px"
  style:--frame={look?.frame ?? 'var(--line)'}
  onclick={() => (planner.selected = selected ? null : sel)}
  onmouseenter={() => (hover = true)}
  onmouseleave={() => (hover = false)}
>
  {#if item?.row && look}
    <img class="bg" src={asset(look.bg)} alt="" />
    {#if icon}<img class="icon" src={asset(icon)} alt={name} />{/if}
    {#if item.tier > 0}<span class="lv">LV.{item.tier}</span>{/if}
    {#if edited}<span class="edited" title={t('item.edited')}>✦</span>{/if}
    {#if hearts > 0}<span class="hearts" title={t('item.hearts', { n: hearts })}>♥{hearts}</span>{/if}
  {:else if slot?.silhouette}
    <img class="sil" src={asset(slot.silhouette)} alt={slot.name} />
  {/if}

  {#if hover && item?.row}
    <div class="tip parchment" class:left={tipLeft}>
      <div class="tip-name" style:color={look?.frame}>{name}</div>
      <div class="tip-sub">{isPet ? t('item.pet') : rarityName(item.rarity)}{item.tier ? ` · ${t('item.level', { n: item.tier })}` : ''}{hearts ? ` · ♥${hearts}` : ''}</div>
      <hr />
      {#each attrs.filter((a) => !isMod(a.tag)) as a, i (i)}
        <div class="tip-row">
          <span>{info(a.tag)?.name || a.tag.split('.').pop()}</span>
          <span class="v">{formatValue(a.value, info(a.tag))}</span>
        </div>
      {/each}
      {#each attrs.filter((a) => isMod(a.tag)) as a, i (i)}
        <div class="tip-mod">{info(a.tag)?.description || info(a.tag)?.name || a.tag.split('.').pop()}{a.value > 1 ? ` (×${a.value})` : ''}</div>
      {/each}
    </div>
  {/if}
</button>

<style>
  .slot {
    position: relative;
    width: var(--size);
    height: var(--size);
    padding: 0;
    border: 3px solid var(--frame);
    background: #0e0d0a;
    cursor: pointer;
    box-shadow: inset 0 0 12px rgba(0, 0, 0, 0.8), 0 0 0 1px #000;
    transition: transform 0.08s ease, box-shadow 0.15s ease;
  }
  .slot:hover {
    transform: translateY(-1px);
    box-shadow: inset 0 0 12px rgba(0, 0, 0, 0.6), 0 0 10px color-mix(in srgb, var(--frame) 60%, transparent);
    z-index: 5;
  }
  .slot.selected {
    outline: 2px solid var(--gold-bright);
    outline-offset: 2px;
    box-shadow: 0 0 18px rgba(236, 198, 107, 0.55);
  }
  .slot.empty {
    border-color: var(--line);
    background: radial-gradient(circle, #23201a, #12100c);
  }
  img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    pointer-events: none;
  }
  .icon {
    object-fit: contain;
    padding: 4%;
  }
  .sil {
    opacity: 0.22;
    padding: 16%;
    object-fit: contain;
  }
  .lv {
    position: absolute;
    top: 2px;
    left: 4px;
    font-family: var(--font-label);
    font-size: calc(var(--size) * 0.17);
    color: #f3e3b3;
    text-shadow: 0 0 3px #000, 0 1px 2px #000, 1px 0 2px #000;
    pointer-events: none;
  }
  .edited {
    position: absolute;
    right: 3px;
    bottom: 1px;
    color: var(--gold-bright);
    text-shadow: 0 0 4px #000;
    font-size: 15px;
  }
  .hearts {
    position: absolute;
    left: 3px;
    bottom: 1px;
    font-family: var(--font-ui);
    font-size: calc(var(--size) * 0.16);
    color: #ff8a8a;
    text-shadow: 0 0 3px #000, 0 1px 2px #000;
    pointer-events: none;
  }
  .tip {
    position: absolute;
    left: calc(100% + 10px);
    top: 0;
    width: 290px;
    padding: 10px 12px;
    text-align: left;
    z-index: 50;
    pointer-events: none;
  }
  .tip.left {
    left: auto;
    right: calc(100% + 10px);
  }
  .tip-name {
    font-family: var(--font-title);
    font-size: 18px;
    font-weight: 700;
    filter: brightness(0.6) saturate(1.4);
  }
  .tip-sub {
    font-size: 13px;
    opacity: 0.75;
  }
  hr {
    border: none;
    border-top: 1px solid rgba(90, 60, 30, 0.35);
    margin: 6px 0;
  }
  .tip-row {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    font-size: 14px;
  }
  .tip-row .v {
    font-variant-numeric: tabular-nums;
    font-weight: 600;
  }
  .tip-mod {
    margin-top: 5px;
    font-size: 13px;
    font-style: italic;
    color: #6b2a7a;
  }
</style>
