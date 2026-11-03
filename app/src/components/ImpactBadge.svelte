<script lang="ts">
  import { t } from '../lib/i18n.svelte'
  import { planner } from '../lib/store.svelte'

  /** Gain of the current objective for one typical roll of `tag`, or a given gain (`value`).
   *  `cond`: the game state the gain was measured in; `unq`: why a NaN value has no number. */
  let { tag, value, title, cond, unq = 'combat' }: { tag?: string; value?: number; title?: string; cond?: string; unq?: 'loot' | 'combat' } = $props()
  const v = $derived(tag !== undefined ? planner.impact(tag) : value)
  // NaN: an effect the stats cannot show (proc, ability behaviour).
  const level = $derived(v === undefined ? '' : Number.isNaN(v) ? 'none' : Math.abs(v) < 1e-7 ? 'none' : v < 0 ? 'neg' : v > 0.005 ? 'high' : v > 0.001 ? 'mid' : 'low')
  const digits = (p: number) => (Math.abs(p) >= 10 ? 1 : Math.abs(p) >= 0.1 ? 2 : 3)
  const text = $derived(v === undefined ? '' : Number.isNaN(v) ? t(unq === 'loot' ? 'impact.loot' : 'impact.unquantified') : Math.abs(v) < 1e-7 ? '—' : `${v > 0 ? '+' : ''}${(v * 100).toFixed(digits(v * 100))}%${cond ? '*' : ''}`)
  const tip = $derived(
    v !== undefined && Number.isNaN(v)
      ? t(unq === 'loot' ? 'impact.lootTitle' : 'impact.unquantifiedTitle')
      : `${title ?? t('impact.gainTitle', { obj: planner.objectiveLabel() })}${cond ? ` — ${t('impact.condTitle', { state: cond })}` : ''}`,
  )
</script>

{#if v !== undefined}
  <span class="badge {level}" class:cond={!!cond} title={tip}>{text}</span>
{/if}

<style>
  .badge {
    font-family: var(--font-ui);
    font-size: 12px;
    padding: 1px 6px;
    border-radius: 9px;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    border: 1px solid transparent;
  }
  .none {
    color: var(--text-faint);
  }
  .low {
    color: #b9c99a;
    border-color: rgba(185, 201, 154, 0.25);
  }
  .mid {
    color: #9be07f;
    border-color: rgba(127, 211, 107, 0.4);
    background: rgba(127, 211, 107, 0.08);
  }
  .high {
    color: #c9ff9e;
    border-color: rgba(160, 240, 120, 0.6);
    background: rgba(127, 211, 107, 0.18);
  }
  .neg {
    color: var(--bad);
    border-color: rgba(229, 115, 95, 0.4);
  }
  .cond {
    border-style: dashed;
  }
</style>
