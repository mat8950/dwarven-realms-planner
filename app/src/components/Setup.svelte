<script lang="ts">
  // Game data setup: the planner ships no game data. It finds the player's installation, takes
  // the mappings file (.usmap) the UE4SS dumper writes, and extracts the data with dr-extract.
  import { open as pick } from '@tauri-apps/plugin-dialog'
  import { onDestroy } from 'svelte'
  import { api } from '../lib/api'
  import { fmtNumber, t } from '../lib/i18n.svelte'
  import { planner } from '../lib/store.svelte'
  import type { ExtractProgress } from '../lib/types'

  const s = $derived(planner.setup)
  let busy = $state(false)
  let progress = $state<ExtractProgress | null>(null)
  let warnings = $state<string[]>([])
  let error = $state<string | null>(null)
  let done = $state(false)

  let unlisten: (() => void) | undefined
  void api.onExtractProgress((p) => {
    if (p.warn) warnings = [...warnings.slice(-4), p.warn]
    else if (p.error) error = p.error
    else progress = p
  }).then((u) => (unlisten = u))
  onDestroy(() => unlisten?.())

  const canExtract = $derived(!!s?.install && !!s?.mappings && !!s?.extractor && !busy)
  const pct = $derived(progress?.total ? Math.round((100 * (progress.done ?? 0)) / progress.total) : null)
  const stepLabel = (step: string | undefined) => (step ? t(`setup.step.${step}`, {}, step) : '')

  async function act(f: () => Promise<unknown>) {
    error = null
    try {
      await f()
    } catch (e) {
      error = String(e)
    }
    await planner.refreshSetup()
  }

  const chooseGame = () =>
    act(async () => {
      const dir = await pick({ directory: true, title: t('setup.pickGame') })
      if (typeof dir === 'string') await api.setGameDir(dir)
    })
  const chooseSaves = () =>
    act(async () => {
      const dir = await pick({ directory: true, title: t('setup.pickSaves') })
      if (typeof dir === 'string') await api.setSaveDir(dir)
    })
  const chooseMappings = () =>
    act(async () => {
      const file = await pick({ title: t('setup.pickMappings'), filters: [{ name: 'Mappings', extensions: ['usmap'] }] })
      if (typeof file === 'string') await api.importMappings(file)
    })
  const importDumped = () => act(() => api.importMappings(s!.dumped[s!.dumped.length - 1]))

  // Mappings dumper: install it, wait for the file (imported as soon as it appears), remove it.
  const LAUNCH_OPTION = 'WINEDLLOVERRIDES="dwmapi=n,b" %command%'
  let dumperBusy = $state(false)
  let copied = $state(false)
  let imported = $state<string | null>(null)
  async function dumperAction(f: () => Promise<void>) {
    dumperBusy = true
    await act(f)
    dumperBusy = false
  }
  async function copyOption() {
    try {
      await navigator.clipboard.writeText(LAUNCH_OPTION)
      copied = true
      setTimeout(() => (copied = false), 1500)
    } catch {
      // no clipboard: the option stays selectable on screen
    }
  }
  // While the dumper is in the game, look for its file every 3 s and import it once.
  $effect(() => {
    if (!s?.dumper_installed) return
    const id = setInterval(() => void planner.refreshSetup(), 3000)
    return () => clearInterval(id)
  })
  $effect(() => {
    const file = s?.dumper_installed ? s.dumped.at(-1) : undefined
    if (file && file !== imported) {
      imported = file
      void act(() => api.importMappings(file))
    }
  })

  async function extract() {
    busy = true
    done = false
    progress = null
    warnings = []
    error = null
    try {
      await api.extract()
      done = true
      await planner.reloadData()
    } catch (e) {
      error = String(e)
    } finally {
      busy = false
      await planner.refreshSetup()
    }
  }

  const close = () => (planner.setupOpen = false)
</script>

<div class="overlay">
  <div class="panel">
    <header>
      <span class="title">{t('setup.title')}</span>
      {#if s?.data_root && !busy}<button class="btn" onclick={close}>{t('common.close')}</button>{/if}
    </header>
    <p class="intro">{t('setup.intro')}</p>

    {#if s}
      <section>
        <h3>1. {t('setup.game')}</h3>
        {#if s.install}
          <div class="row ok">
            <span class="path">{s.install.dir}</span>
            <span class="tag">{t(`setup.store.${s.install.store}`, {}, s.install.store)} · {t('setup.build', { build: s.install.build })}</span>
          </div>
        {:else}
          <div class="row bad">{t('setup.gameMissing')}</div>
        {/if}
        <div class="actions">
          <button class="btn" onclick={chooseGame}>{t('setup.chooseGame')}</button>
          {#if s.manual_dir}<button class="btn ghost" onclick={() => act(() => api.setGameDir(null))}>{t('setup.autoDetect')}</button>{/if}
        </div>
        <div class="row" class:ok={!!s.save_dir} class:bad={!s.save_dir}>
          <span class="label">{t('setup.saves')}</span>
          <span class="path">{s.save_dir ?? t('setup.savesMissing')}</span>
          <button class="btn ghost" onclick={chooseSaves}>{t('setup.change')}</button>
        </div>
      </section>

      <section>
        <h3>2. {t('setup.mappings')}</h3>
        <p class="help">{t('setup.mappingsWhy')}</p>
        {#if s.mappings}
          <div class="row ok"><span class="path">{s.mappings}</span></div>
        {:else}
          <div class="row bad">{t('setup.mappingsMissing')}</div>
        {/if}
        {#if !s.dumper_installed}
          <div class="actions">
            <button class="btn" disabled={!s.install || !s.extractor || dumperBusy} onclick={() => dumperAction(api.dumperInstall)}>{t('setup.dumperInstall')}</button>
            {#if dumperBusy}<span class="help">{t('setup.dumperDownloading')}</span>{/if}
          </div>
          <p class="help">{t('setup.dumperInstallHelp')}</p>
        {:else}
          <ol class="steps">
            {#if s.os === 'macos'}
              <li>{t('setup.dumperOptionMac')}</li>
            {:else if s.os !== 'windows'}
              <li>
                {t('setup.dumperOption')} <code>{LAUNCH_OPTION}</code>
                <button class="btn ghost" onclick={copyOption}>{copied ? t('setup.copied') : t('setup.copy')}</button>
              </li>
            {/if}
            <li>{t('setup.dumperRun')}</li>
            <li class:found={!!imported}>{imported ? t('setup.dumperFound') : t('setup.dumperWaiting')}</li>
            <li>
              {t('setup.dumperRemoveHelp')}
              <button class="btn" disabled={dumperBusy} onclick={() => dumperAction(api.dumperRemove)}>{t('setup.dumperRemove')}</button>
            </li>
          </ol>
        {/if}
        <div class="actions">
          {#if s.dumped.length && !s.dumper_installed}<button class="btn ghost" onclick={importDumped}>{t('setup.importDumped')}</button>{/if}
          <button class="btn ghost" onclick={chooseMappings}>{t('setup.importFile')}</button>
        </div>
        <details>
          <summary>{t('setup.howTo')}</summary>
          <ol>
            <li>{t('setup.how1')}</li>
            <li>{t('setup.how2')}</li>
            <li>{t('setup.how3')}</li>
            <li>{t('setup.how4')}</li>
            <li>{t('setup.how5')}</li>
          </ol>
        </details>
      </section>

      <section>
        <h3>3. {t('setup.extract')}</h3>
        {#if s.data_root}
          <div class="row" class:ok={s.up_to_date} class:warn={!s.up_to_date}>
            {s.up_to_date ? t('setup.dataOk', { build: s.data_build ?? '?' }) : t('setup.dataOld', { build: s.data_build ?? '?' })}
          </div>
        {/if}
        {#if !s.extractor}<div class="row bad">{t('setup.noExtractor')}</div>{/if}
        <div class="actions">
          <button class="btn primary" disabled={!canExtract} onclick={extract}>{s.data_root ? t('setup.reextract') : t('setup.extractNow')}</button>
          <span class="help">{t('setup.extractTime')}</span>
        </div>
        {#if busy || progress}
          <div class="progress">
            <div class="bar"><div class="fill" style:width={`${pct ?? (busy ? 5 : 100)}%`}></div></div>
            <span>{stepLabel(progress?.step)}{progress?.total ? ` ${fmtNumber(progress.done ?? 0)} / ${fmtNumber(progress.total)}` : ''}</span>
          </div>
        {/if}
        {#each warnings as w (w)}<div class="warnline">{w}</div>{/each}
        {#if done}<div class="row ok">{t('setup.done')}</div>{/if}
      </section>
    {/if}
    {#if error}<pre class="error">{error}</pre>{/if}
    <p class="legal">{t('setup.legal')}</p>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 700;
    display: flex;
    justify-content: center;
    padding: 24px;
    overflow: auto;
    background: rgba(8, 7, 5, 0.9);
  }
  .panel {
    width: min(860px, 100%);
    height: fit-content;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px 20px;
    background: var(--panel);
    border: 1px solid var(--line);
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .title {
    font-family: var(--font-title);
    font-size: 22px;
    color: var(--gold-bright);
  }
  h3 {
    margin: 0 0 6px;
    font-family: var(--font-label);
    font-size: 15px;
    color: var(--gold);
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px 12px;
    background: var(--panel-2);
  }
  .intro,
  .help,
  .legal {
    margin: 0;
    font-family: var(--font-ui);
    font-size: 13px;
    color: var(--text-dim);
  }
  .legal {
    color: var(--text-faint);
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    font-family: var(--font-ui);
    font-size: 13px;
  }
  .row.ok::before,
  .row.bad::before,
  .row.warn::before {
    font-weight: 700;
  }
  .row.ok::before {
    content: '✓';
    color: var(--good);
  }
  .row.bad::before {
    content: '✕';
    color: var(--bad);
  }
  .row.warn::before {
    content: '!';
    color: var(--gold-bright);
  }
  .label {
    color: var(--text-dim);
  }
  .path {
    font-family: ui-monospace, monospace;
    font-size: 12px;
    word-break: break-all;
  }
  .tag {
    color: var(--text-dim);
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  details {
    font-family: var(--font-ui);
    font-size: 13px;
    color: var(--text-dim);
  }
  summary {
    cursor: pointer;
    color: var(--text);
  }
  ol {
    margin: 6px 0 0;
    padding-left: 20px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .btn.primary {
    border-color: var(--gold);
    padding: 6px 18px;
  }
  .steps {
    margin: 0;
    padding-left: 20px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-family: var(--font-ui);
    font-size: 13px;
  }
  .steps .found {
    color: var(--good);
  }
  code {
    padding: 1px 5px;
    font-family: ui-monospace, monospace;
    font-size: 12px;
    background: var(--bg-2);
    user-select: all;
  }
  .progress {
    display: flex;
    align-items: center;
    gap: 10px;
    font-family: var(--font-ui);
    font-size: 13px;
  }
  .bar {
    flex: 1;
    height: 8px;
    background: var(--bg-2);
    border: 1px solid var(--line);
  }
  .fill {
    height: 100%;
    background: var(--gold);
    transition: width 0.2s;
  }
  .warnline {
    font-family: ui-monospace, monospace;
    font-size: 11px;
    color: var(--text-faint);
  }
  .error {
    margin: 0;
    padding: 8px;
    white-space: pre-wrap;
    color: var(--bad);
    background: var(--bg-2);
  }
</style>
