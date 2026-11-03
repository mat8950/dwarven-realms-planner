<script lang="ts">
  import { LEFT_SLOTS, RIGHT_SLOTS, TOP_SLOT, WEAPON_CHILDREN } from '../lib/game'
  import { t } from '../lib/i18n.svelte'
  import { planner } from '../lib/store.svelte'
  import ItemSlot from './ItemSlot.svelte'

  const slotInfo = (v: number) => planner.catalog?.slots.find((s) => s.value === v)
  const item = (v: number) => planner.character?.items[v] ?? null
  const cr = $derived(planner.stats?.combat_rating ?? 0)
  const baseCr = $derived(planner.baseline?.combat_rating ?? 0)
  const crDelta = $derived(baseCr ? (cr - baseCr) / baseCr : 0)
  const crText = (n: number) => new Intl.NumberFormat('en-US', { maximumFractionDigits: 0 }).format(n)
</script>

<section class="doll frame">
  <div class="col">
    {#each LEFT_SLOTS as v (v)}<ItemSlot sel={v} item={item(v)} slot={slotInfo(v)} />{/each}
  </div>

  <div class="center">
    <div class="top"><ItemSlot sel={TOP_SLOT} item={item(TOP_SLOT)} slot={slotInfo(TOP_SLOT)} size={80} /></div>

    <div class="hero">
      {#if planner.character}
        <div class="emblem">
          <div class="ring"></div>
          <div class="cr-label label">{t('paperdoll.cr')}</div>
          <div class="cr">{crText(cr)}</div>
          {#if Math.abs(crDelta) > 1e-6}
            <div class="cr-delta" class:good={crDelta > 0} class:bad={crDelta < 0}>{crDelta > 0 ? '+' : ''}{(crDelta * 100).toFixed(2)}%</div>
          {/if}
        </div>
      {/if}
    </div>

    <div class="weapon">
      <ItemSlot sel="weapon" item={planner.character?.weapon ?? null} size={84} />
      <svg class="links" viewBox="0 0 300 40" preserveAspectRatio="none">
        <path d="M150 0 V18 M50 18 H250 M50 18 V40 M150 18 V40 M250 18 V40" stroke="var(--gold-dim)" stroke-width="2" fill="none" />
      </svg>
      <div class="children">
        {#each WEAPON_CHILDREN as v (v)}<ItemSlot sel={v} item={item(v)} slot={slotInfo(v)} size={76} />{/each}
      </div>
    </div>
  </div>

  <div class="col">
    {#each RIGHT_SLOTS as v (v)}<ItemSlot sel={v} item={item(v)} slot={slotInfo(v)} tipLeft />{/each}
  </div>
</section>

<style>
  .doll {
    display: grid;
    grid-template-columns: auto 1fr auto;
    gap: 18px;
    padding: 22px 26px;
    min-width: 560px;
  }
  .col {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    gap: 12px;
  }
  .center {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: space-between;
    min-width: 300px;
  }
  .hero {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
  }
  .emblem {
    position: relative;
    width: 230px;
    height: 230px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
  }
  .ring {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    border: 2px solid var(--gold-dim);
    background:
      radial-gradient(circle, rgba(201, 162, 74, 0.12), transparent 62%),
      repeating-conic-gradient(from 0deg, rgba(201, 162, 74, 0.18) 0 2deg, transparent 2deg 15deg);
    box-shadow: inset 0 0 40px rgba(0, 0, 0, 0.8), 0 0 30px rgba(201, 162, 74, 0.08);
  }
  .cr-label {
    position: relative;
    font-size: 12px;
  }
  .cr {
    position: relative;
    font-family: var(--font-title);
    font-size: 34px;
    color: var(--gold-bright);
    text-shadow: 0 0 14px rgba(236, 198, 107, 0.35);
    font-variant-numeric: tabular-nums;
  }
  .cr-delta {
    position: relative;
    font-family: var(--font-ui);
    font-size: 16px;
    font-variant-numeric: tabular-nums;
  }
  .weapon {
    display: flex;
    flex-direction: column;
    align-items: center;
  }
  .links {
    width: 268px;
    height: 26px;
  }
  .children {
    display: flex;
    gap: 22px;
  }
</style>
