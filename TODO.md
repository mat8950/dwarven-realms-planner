# TODO

## Farming history (fortune rupture runs)

A player logs every rupture run by hand in a spreadsheet (screenshot received 2026-11-04). The app fills what the
save holds and asks only for the rest.

### Done

- dr-core `session`: each character save the game writes is read (resources, activity counters, weapon skills,
  level, XP, map) and appended to `history/<save id>.session.jsonl`. The desktop app checks the save folder every
  2 seconds while it runs (`spawn_session_watch`).
- "Session" view, "Session" tab: gains since the start of the play session, save by save.
- "Session" view, "Runs" tab (`app/src/lib/runs.ts`, `RunsView.svelte`):
  - runs found in the journal, with their duration, all their gains, elven fragments per run and per hour;
  - what the player adds by hand, kept in `history/<save id>.runs.json`: rupture level, boon, bane, essence, rerolls,
    elven containers, elites, monsters, notes;
  - the current farm level: entered once, every run after takes it until the next power-up;
  - Fortune keys spent at the launch give the rerolls (keys - 1) as a default;
  - averages by boon and by bane, filters by level and boon, copy of the table for a spreadsheet.
- Extraction: icons of the resources and of the rupture fortunes (`textures.py`, `Program.cs`).
- `cargo run --release -p dr-cli --example session`: the terminal version (the port of `drstats`), with `--keep`
  to copy each save and `import` to add such copies to a journal.

### Next

- Run the Runs tab in the desktop app for a few evenings of play, and compare with the player's spreadsheet.
- Optional: an essence shop helper. From the essence collected, show how many containers it buys and what a shop
  reroll costs, with the price rule below.
- How many fragments an ElvenFragments container gives (its container blueprint), to compare with the gains.

### How the mode works (read from the game's blueprints, build 24115719)

- **Fortunes.** In this rupture mode an NPC (`BP_FortuneRuptureInteractable`) stands at the start. Revealing its
  cards costs one Tomb of Fortune key (`Key_Casino`, resource `CasinoKey` in the save). There are 3 cards. The
  manager (`BP_RuptureFortune_Manager`) shuffles the boons and keeps 3, and shuffles the banes and keeps 3. Card i
  pairs boon i with bane i. The player takes one card. The cards are rolled once per rupture: to get other cards,
  players leave and enter a new rupture, which costs another key. The sheet's "Rerolls" are these new entries
  (the saves show one key spent per entry: 3 keys in a minute at 23:27-23:28 on 2026-11-04).
- **Boons** (`DT_RuptureFortunes`, tags `Fortunes.Positive.*`), with what the code does:
  - DoubleContainers ("2xc"): each container that spawns on a kill (skill `ChanceToSpawnContainer`) spawns a second
    one at the player.
  - DoubleBossChest: double chest from the rupture lord.
  - DoubleEssence ("2xe"): essence gains x2.
  - DoubleTears ("2xt"): each tear event spawns a second one nearby.
  - IncreasedRareSpawn: up to 6 more elites per spawn zone.
  - TenTimesGold: 40 % chance that a tear spawns the loot spider (golden tears).
  - DoubleXP, DoubleDestructableLoot (mine nodes), AlwaysElven (elven capital portal), ForceGreenTear (first tear is
    an Emerald one), ForcceWeaponMonogram (weapon monogram at the end).
  - ForceAllContainersHealth: every container is a health stone container, the essence shop's too. Bad for a
    fragment farm.
- **Banes** (`Fortunes.Negative.*`): EnemiesDealDoubleDamage ("2xdmg"), EnemiesDoubleHP ("2xh"),
  GlobalEarthQuakeChance ("eq"), NoHealthRegen ("no hr"), NoLifeSteal ("no ls"), NoPotions ("no pots"),
  NoPrimarySecondary.
- **Essence** (`BP_RuptureOrbsComponent`, "orbs" in the code): enemies drop it. DoubleEssence doubles each gain,
  and the skill `GainExtraEssence` doubles it with a 33 % chance. The boss opens at 100 essence.
- **Essence shop** at the end (`UI_EssenceTurnIn`): it offers 5 container types drawn at random from
  `DT_ContainerSpawners` (25 types, each with a minimum level; "ElvenFragments" needs level 50).
  - Price of a container: random 70 to 145, times trunc(1 + 0.3 x n). n counts the purchases and shop rerolls
    done in this shop: x1 for n = 0 to 3, x2 for 4 to 6, x3 for 7 to 9, x4 for 10 to 13.
  - The shop's "Reroll" costs 50 essence, draws 5 new types, and adds 1 to n.
  - The player's pets (small dragons) open the containers. The player spends all the essence before leaving.

### When the game saves (measured 2026-11-04 on fortune runs of about 12 minutes)

- The game writes nothing while the player stays in the fort or plays a run. It saves at map changes and when a
  launch item is spent. Each character save is followed by a world save (`_WRL.sav`).
- Run start: a few character saves in seconds. Fort (an Infused Beetle spent), then the rupture map
  (`CurrentLevel` = `<Zone>_rupture_NN_Infused`, `CasinoKey` -1 when the cards are revealed), then the fort again,
  once per entry when the player looks for other cards.
- Run end: one character save about 12 minutes later, on a rupture map, with the whole run's gains (essence shop
  included).
- `NN` in the map name is not the rupture level, and the save does not have the level.
- Infused Beetles also drop during runs: their count is not a run marker.
- Results: run 1 +272,400 elven fragments in 11 min 46 s; run 2, after a loot-oriented elven tree, +386,500 in
  11 min 42 s (coins +44 %, Kabarian fragments x2.1, Fortune keys +8). One run per setup proves nothing: the
  history must average many.

## For the game's developers (the player will forward it)

- **Run data in the save.** The rupture level, the fortune card taken (boon and bane), the essence collected and
  spent, the containers bought, the kills (monsters, elites), and the start and end times of each run are not
  saved. A short list of the last runs in the character save would let tools show a farming history without
  manual entry.
- **Saves while playing.** The game writes the save at map changes and a few actions (equipment window's "hide
  helmet" box, crafting, gambling, death, main menu). The periodic autosave (`AutosaveTick_Server`, every
  15 seconds) only saves in one zone. A regular autosave everywhere (every few minutes) would let tools show the
  bag's loot without changing map.
- **Essence lost at the end of a run.** With the essence, a player can spawn flint chests. Opening them spawns
  elites, which drop more containers and more essence. But the essence terminal disappears once the containers
  have spawned, so the player ends the run with essence left that cannot be spent. Keeping the terminal until the
  essence is spent (or converting what is left) would fix it.
