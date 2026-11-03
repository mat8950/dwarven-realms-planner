<script lang="ts">
  import { fmtNumber, t } from '../lib/i18n.svelte'
  import { debug } from '../lib/debug.svelte'
  import { planner } from '../lib/store.svelte'

  let tab = $state<'ui' | 'backend'>('ui')
  let copied = $state(false)

  $effect(() => {
    if (debug.open) debug.refreshBackend()
  })

  const s = $derived(planner.stats)
  const stepPct = $derived(s && s.max_steps ? (s.steps / s.max_steps) * 100 : 0)

  /** Everything needed to diagnose an issue, as plain text. */
  function report(): string {
    const b = debug.backend
    const lines = [
      `# Dwarven Realms Planner — debug report ${new Date().toISOString()}`,
      `version ${b?.version ?? '?'} · data ${planner.catalog?.build ?? '?'} · ${planner.catalog?.lang ?? '?'} · ${'__TAURI_INTERNALS__' in window ? 'Tauri' : 'browser'}`,
      `log ${b?.log_path ?? '?'}`,
      `root ${b?.data_root ?? '?'} · saves ${b?.save_dir ?? '?'}`,
      `save ${planner.character?.save ?? '-'} (${planner.character?.name ?? '-'} lvl ${planner.character?.level ?? '-'})`,
      `objective ${JSON.stringify(planner.objective)} · forge ${planner.forgeMode} · stance ${s?.stance ?? '-'} · potions ${planner.potions ?? 'full'}/${s?.max_potions ?? '?'}`,
      `edited build: items ${Object.keys(planner.edits).join(',') || '-'} · weapon ${planner.weaponRow ?? (planner.weaponEdit ? 'edited' : '-')} · hearts ${JSON.stringify(planner.hearts)} · effects ${planner.effects.length} · status ${JSON.stringify(planner.statusEffects)} · tree ${planner.tree ? planner.tree.length : 'save'} · skills ${Object.keys(planner.skillLevels).length}`,
      s ? `engine: ${s.elapsed_ms.toFixed(1)} ms · steps ${s.steps}/${s.max_steps} (${stepPct.toFixed(2)}%) · CR ${s.combat_rating.toFixed(0)}` : 'engine: no compute',
      `not simulated (${s?.missing.length ?? 0}): ${s?.missing.join(' | ') || '-'}`,
      '',
      '## appels',
      ...Object.entries(debug.calls).map(([k, c]) => `${k}: ${c.count}× · ${c.errors} error(s) · last ${c.lastMs.toFixed(0)} ms · max ${c.maxMs.toFixed(0)} ms`),
      '',
      '## journal UI',
      ...debug.entries.map((e) => `${e.time} ${e.level.toUpperCase()} [${e.source}] ${e.message}`),
      '',
      '## journal backend (fin)',
      ...(b?.recent ?? []),
    ]
    return lines.join('\n')
  }

  async function copy() {
    try {
      await navigator.clipboard.writeText(report())
      copied = true
      setTimeout(() => (copied = false), 1500)
    } catch (e) {
      debug.log('warn', 'debug', `copy failed: ${e}`)
    }
  }
</script>

{#if debug.open}
  <div class="backdrop" role="presentation" onclick={() => (debug.open = false)}></div>
  <aside class="debug frame">
    <header>
      <span class="title">{t('debug.title')}</span>
      <div class="tabs">
        <button class="btn" class:on={tab === 'ui'} onclick={() => (tab = 'ui')}>{t('debug.app')}</button>
        <button class="btn" class:on={tab === 'backend'} onclick={() => ((tab = 'backend'), debug.refreshBackend())}>{t('debug.disk')}</button>
      </div>
      <span class="spacer"></span>
      <button class="btn" onclick={copy}>{copied ? t('debug.copied') : t('debug.copy')}</button>
      <button class="btn ghost" onclick={() => debug.clear()}>{t('debug.clear')}</button>
      <button class="btn ghost" title={t('debug.closeTitle')} onclick={() => (debug.open = false)}>✕</button>
    </header>

    <div class="grid">
      <div class="card">
        <div class="label">{t('debug.engine')}</div>
        {#if s}
          <div class="kv"><span>{t('debug.engineTime')}</span><b>{s.elapsed_ms.toFixed(1)} ms</b></div>
          <div class="kv"><span>{t('debug.steps')}</span><b>{fmtNumber(s.steps)} / {fmtNumber(s.max_steps)}</b></div>
          <div class="bar"><div style:width="{Math.min(100, stepPct)}%" class:warn={stepPct > 50}></div></div>
          <div class="kv"><span>{t('debug.stancePotions')}</span><b>{s.stance} · {planner.potions ?? s.max_potions}/{s.max_potions}</b></div>
        {:else}
          <p class="dim">{t('debug.noCompute')}</p>
        {/if}
      </div>
      <div class="card">
        <div class="label">{t('debug.calls')}</div>
        {#each Object.entries(debug.calls) as [k, c] (k)}
          <div class="kv"><span>{k}</span><b class:bad={c.errors > 0}>{c.count}× · {c.lastMs.toFixed(0)} ms{c.errors ? ` · ${c.errors} err.` : ''}</b></div>
        {/each}
      </div>
      <div class="card">
        <div class="label">{t('debug.data')}</div>
        <div class="kv"><span>{t('debug.gameBuild')}</span><b>{planner.catalog?.build ?? '?'}</b></div>
        <div class="kv"><span>{t('debug.version')}</span><b>{debug.backend?.version ?? '?'}</b></div>
        <div class="path" title={t('debug.logTitle')}>{debug.backend?.log_path ?? ''}</div>
        <div class="path" title={t('debug.saveTitle')}>{planner.character?.save ?? ''}</div>
      </div>
      <div class="card">
        <div class="label">{t('debug.missing', { n: s?.missing.length ?? 0 })}</div>
        <div class="missing scroll">
          {#each s?.missing ?? [] as m (m)}<div>{m}</div>{:else}<div class="dim">{t('debug.none')}</div>{/each}
        </div>
      </div>
    </div>

    <div class="log scroll">
      {#if tab === 'ui'}
        {#each [...debug.entries].reverse() as e, i (i)}
          <div class="line {e.level}"><span class="t">{e.time}</span><span class="src">{e.source}</span><span class="msg">{e.message}</span></div>
        {:else}
          <div class="dim">{t('debug.emptyLog')}</div>
        {/each}
      {:else}
        {#each [...(debug.backend?.recent ?? [])].reverse() as l, i (i)}
          <div class="line raw" class:error={/ (ERROR|PANIC) /.test(l)}>{l}</div>
        {:else}
          <div class="dim">{t('debug.emptyDisk')}</div>
        {/each}
      {/if}
    </div>
  </aside>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    z-index: 900;
  }
  .debug {
    position: fixed;
    left: 4%;
    right: 4%;
    bottom: 3%;
    height: 62%;
    z-index: 901;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px 14px;
    font-family: var(--font-ui);
  }
  header {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  header .title {
    font-size: 20px;
  }
  .tabs {
    display: flex;
    gap: 4px;
  }
  .tabs .on {
    border-color: var(--gold);
    background: linear-gradient(180deg, #4a4130, #2c261a);
  }
  .spacer {
    flex: 1;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 10px;
  }
  .card {
    background: rgba(0, 0, 0, 0.25);
    border: 1px solid var(--line);
    padding: 8px 10px;
    min-height: 0;
  }
  .card .label {
    font-size: 11px;
    margin-bottom: 4px;
  }
  .kv {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    font-size: 13px;
  }
  .kv span {
    color: var(--text-dim);
  }
  .kv b {
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .bar {
    height: 5px;
    background: #12100c;
    border: 1px solid var(--line);
    margin: 3px 0 5px;
  }
  .bar div {
    height: 100%;
    background: var(--good);
  }
  .bar div.warn {
    background: var(--bad);
  }
  .path {
    font-size: 11px;
    color: var(--text-faint);
    word-break: break-all;
    margin-top: 3px;
  }
  .missing {
    max-height: 90px;
    font-size: 12px;
    color: var(--text-dim);
  }
  .log {
    flex: 1;
    min-height: 0;
    background: #0c0b08;
    border: 1px solid var(--line);
    padding: 6px 8px;
    font-family: ui-monospace, 'JetBrains Mono', monospace;
    font-size: 12px;
  }
  .line {
    display: flex;
    gap: 10px;
    white-space: pre-wrap;
    padding: 1px 0;
    border-bottom: 1px solid #16140f;
  }
  .line .t {
    color: var(--text-faint);
    flex: none;
  }
  .line .src {
    color: var(--gold);
    flex: none;
    min-width: 70px;
  }
  .line.error,
  .line.error .msg {
    color: #ff9c8a;
  }
  .line.warn .msg {
    color: #f0c46b;
  }
  .line.raw {
    color: var(--text-dim);
  }
  .dim {
    color: var(--text-faint);
  }
  .bad {
    color: var(--bad);
  }
</style>
