<script lang="ts">
  import { untrack } from 'svelte'
  import DebugPanel from './components/DebugPanel.svelte'
  import Setup from './components/Setup.svelte'
  import Inventory from './components/Inventory.svelte'
  import HistoryView from './components/HistoryView.svelte'
  import SessionView from './components/SessionView.svelte'
  import ItemEditor from './components/ItemEditor.svelte'
  import SkillTree from './components/SkillTree.svelte'
  import Paperdoll from './components/Paperdoll.svelte'
  import StatsPanel from './components/StatsPanel.svelte'
  import TopBar from './components/TopBar.svelte'
  import { debug } from './lib/debug.svelte'
  import { t } from './lib/i18n.svelte'
  import { planner } from './lib/store.svelte'

  planner.init()

  // The objective is kept between sessions.
  $effect(() => {
    try {
      localStorage.setItem('dr-planner.objective', JSON.stringify(planner.objective))
    } catch {
      // no storage: the choice lasts for this session
    }
  })

  // Recompute whenever the build or the situation changes (debounced).
  let timer: ReturnType<typeof setTimeout> | undefined
  let atimer: ReturnType<typeof setTimeout> | undefined
  $effect(() => {
    void [planner.character, planner.edits, planner.weaponEdit, planner.weaponRow, planner.hearts, planner.forgeMode, planner.tree, planner.skillLevels, planner.stance, planner.potions, planner.effects, planner.statusEffects, planner.ruptureOrbs, planner.colossus, planner.unholyVoid]
    clearTimeout(timer)
    timer = setTimeout(() => planner.recompute(), 40)
    // The impact analysis (~1 s) follows once the build stops changing.
    clearTimeout(atimer)
    atimer = setTimeout(() => planner.analyze().then(() => planner.analyzeTree()), 700)
  })
  // Opening the tree view analyses its nodes for the current build.
  $effect(() => {
    void planner.skillsTab
    if (planner.treeOpen) untrack(() => void planner.analyzeTree())
  })
</script>

<main>
  <TopBar />
  <DebugPanel />
  {#if planner.treeOpen && planner.character}<SkillTree />{/if}
  {#if planner.historyOpen}<HistoryView />{/if}
  {#if planner.inventoryOpen && planner.character}<Inventory />{/if}
  {#if planner.sessionOpen && planner.character}<SessionView />{/if}
  {#if planner.setupOpen}<Setup />{/if}
  {#if planner.loading}
    <div class="splash title">{t('app.loading')}</div>
  {:else if !planner.character && planner.error}
    <div class="splash"><div class="title">{t('app.error')}</div><pre>{planner.error}</pre></div>
  {:else}
    <div class="grid">
      <Paperdoll />
      <ItemEditor />
      <StatsPanel />
    </div>
    {#if planner.error}<button class="err" title={t('app.openDebug')} onclick={() => (debug.open = true)}>{planner.error}</button>{/if}
  {/if}
</main>

<style>
  main {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 14px;
  }
  .grid {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: auto minmax(340px, 1fr) minmax(400px, 460px);
    gap: 14px;
  }
  .splash {
    margin: auto;
    text-align: center;
    font-size: 20px;
  }
  .splash pre {
    font-size: 13px;
    color: var(--bad);
    white-space: pre-wrap;
    max-width: 800px;
  }
  .err {
    position: fixed;
    bottom: 12px;
    left: 50%;
    transform: translateX(-50%);
    background: #3a1712;
    border: 1px solid var(--bad);
    color: #ffd8cf;
    padding: 8px 14px;
    font-family: var(--font-ui);
    max-width: 80%;
    cursor: pointer;
    z-index: 800;
  }
</style>
