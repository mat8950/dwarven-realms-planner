<script lang="ts">
  import { t } from '../lib/i18n.svelte'
  import { planner } from '../lib/store.svelte'

  /** Objective picker: DPS total, combat rating, one weapon attack (current stance) or one offhand. */
  const o = $derived(planner.objective)
  const value = $derived(o.kind === 'dps' ? (o.boss ? 'dps:boss' : 'dps') : o.kind === 'cr' ? 'cr' : `${o.kind}:${o.index}`)
  function set(v: string) {
    const [kind, i] = v.split(':')
    if (kind === 'dps') planner.objective = { kind: 'dps', boss: i === 'boss' }
    else if (kind === 'cr') planner.objective = { kind: 'cr' }
    else planner.objective = { kind: kind as 'attack' | 'ability', index: +i }
  }
  const attacks = $derived((planner.catalog?.attacks ?? []).map((a, index) => ({ a, index })).filter(({ a }) => a.stance === planner.stats?.stance))
</script>

<select class="field" {value} onchange={(e) => set(e.currentTarget.value)} title={t('objective.title')}>
  <option value="dps">{t('objective.dps')}</option>
  <option value="dps:boss">{t('objective.dpsBoss')}</option>
  <option value="cr">{t('objective.cr')}</option>
  <optgroup label={t('objective.attacks')}>
    {#each attacks as { a, index } (a.row)}<option value="attack:{index}">{a.name}</option>{/each}
  </optgroup>
  <optgroup label={t('objective.offhands')}>
    {#each planner.catalog?.abilities ?? [] as a, index (a.row)}<option value="ability:{index}">{a.name}</option>{/each}
  </optgroup>
</select>
