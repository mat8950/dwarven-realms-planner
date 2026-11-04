# Dwarven Realms build planner

Build planner for Dwarven Realms. All game data comes from the installed game
(Steam app 2015240), never from wikis, so it always matches the current patch.

> **Not affiliated with Crater Studios.** Dwarven Realms and its content belong to Crater Studios.
> This repository and its installers contain **no game data**: no tables, texts, textures, fonts, decompiled
> code, mappings or saves. The planner reads them from your own copy of the game, on your own computer.
> You need to own the game to use it.

## Installing

The [releases](https://github.com/mat8950/dwarven-realms-planner/releases) have installers for:

| System | Files |
|---|---|
| Windows 10/11 (x64) | `.msi` or `-setup.exe` |
| Linux (x64) | `.deb` (Debian, Ubuntu), `.rpm` (Fedora, openSUSE) or `.AppImage` (any distribution) |
| macOS 11+ (Apple Silicon) | `.dmg` |

The installers are not signed:

- Windows: SmartScreen asks for confirmation. Click "More info", then "Run anyway".
- macOS: the first time, open the app with right-click > Open. Or run
  `xattr -dr com.apple.quarantine "/Applications/Dwarven Realms Planner.app"`.

### First run: extracting the game data

The app opens on its "Game data" screen (also in the top bar, red after a game update):

1. **Game**: Steam libraries (Windows, Linux, CrossOver on macOS) and Epic (Windows, Heroic on Linux) are searched.
   Otherwise choose the folder that contains `ProjectAlpha`. The save folder is found from the installation.
2. **Mappings** (`.usmap`): the game stores its data without field names, and only the running game can give them.
   "Install the generator in the game" downloads [RE-UE4SS](https://github.com/UE4SS-RE/RE-UE4SS) (latest experimental
   release, checked against GitHub's SHA-256) and installs it with the DRDump mod. Under Proton, add the Steam launch
   option `WINEDLLOVERRIDES="dwmapi=n,b" %command%`. Start the game and load a character: about 15 s later the file
   appears and the app imports it. Then close the game and click "Remove UE4SS from the game" (it removes only what it
   installed). One mappings file stays valid as long as the game's classes do not change.
3. **Extraction** (about a minute): the bundled `dr-extract` reads the game's pak and writes the data to the app's
   data folder. The pak is Oodle-compressed: the Oodle library is downloaded once from the
   [OodleUE](https://github.com/WorkingRobot/OodleUE) mirror and checked against its SHA-256.

The app's folders:

| | Linux | Windows | macOS |
|---|---|---|---|
| Data (`game/<build>/`, `mappings/`, `history/`, `settings.json`) | `~/.local/share/dr-planner` | `%APPDATA%\dr-planner` | `~/Library/Application Support/dr-planner` |
| Cache (`app.log`, `oodle/`) | `~/.cache/dr-planner` | `%LOCALAPPDATA%\dr-planner` | `~/Library/Caches/dr-planner` |

### How the installers are built

`.github/workflows/release.yml` builds them with [tauri-action](https://github.com/tauri-apps/tauri-action) on GitHub's
Windows, Ubuntu 22.04 and macOS (arm64) runners:

- Push a tag `v<version>`, for example `git tag v0.1.0 && git push origin v0.1.0`. The installers are attached to a
  draft release. Check it and publish it by hand. The tag must match the version in `Cargo.toml` and
  `app/src-tauri/tauri.conf.json`.
- Or run the "Release" workflow by hand (Actions tab). The installers are then only kept as workflow artifacts.

`.github/workflows/ci.yml` checks every push: UI types (`npm run check`), translations (`tests/i18n-check.mjs`),
UI build and `cargo test --workspace`.

## Running the app from source

    tools/extract/build-extractor.sh   # dr-extract sidecar (.NET 10 SDK, or Docker): Tauri needs it to build
    cd app && npm install
    npx tauri dev                 # desktop app (Tauri 2), hot reload
    npx tauri build --no-bundle   # release binary: target/release/dr-app

Which data the app loads: `DR_PLANNER_ROOT` if set; in a debug build, this repository's `data/` when
`tools/extract/extract.sh` filled it; else the data extracted for the installed game build; else the newest extraction.

To run it in a browser (handy for headless tests), start the HTTP bridge and Vite:

    cargo run --release --bin serve      # API on 127.0.0.1:5174
    cd app && npx vite                   # http://localhost:5173 (proxies /api)
    node tests/shots.mjs /tmp            # headless screenshots of the main flows

Check the engine against the game from the command line:

    cargo run --release --bin dr-cli -- <character.sav> --compare research/validation/atlantia.json

## Layout

- `tools/extract/extract.sh`: re-extracts everything into `data/extracted/` and `research/pseudo/`. Run it after every game patch.
- `tools/extract/cue4parse/`: `dr-extract`, built on CUE4Parse.
  - `dr-extract game ...` is the app's extraction: it reads the pak directly and does what `extract.sh` and its Python
    helpers do (same output, checked file by file).
  - `dr-extract dumper install|remove ...` installs or removes the UE4SS mappings dumper.
  - Without a subcommand it is the JSON/texture exporter `extract.sh` runs (in the `dotnet/sdk:10.0` Docker image).
  - `tools/extract/build-extractor.sh [<runtime id>]` publishes it as one self-contained file into
    `app/src-tauri/binaries/` (Tauri sidecar).
- `tools/extract/kismet.py`: turns blueprint bytecode JSON into readable pseudo-code.
- `tools/extract/skilltree.py`: writes `data/extracted/skill_tree.json`, the main skill tree layout (node positions, sizes, links)
  read from the `UI_SkillTree_S4realzies` widget.
- `tools/extract/elventree.py`: writes `data/extracted/elven_tree.json`, the skills of the elven tree (the elven harp,
  `UI_CraftingSkillTree_S24`). Its table `DT_Skills_Crafting_S24` also holds 5 old skills the game no longer shows
  (Lucky, Mule, Very Very Lucky, Greater Mule `Luck_Inventory2`, Spirit of the Hermonyan Explorer): the app hides them.
- `data/extracted/FORMAT`: layout version of the extracted data. The app asks for a new extraction when it is older
  than `dr_core::setup::DATA_FORMAT`.
- `tools/extract/ue4ss-dump/DRDump/`: UE4SS Lua mod that writes the `.usmap` mappings.
- `data/mappings/*.usmap`: mappings for the current game build. Without them no table can be read.
- `data/extracted/tables/`: DataTables, string tables and `.locres` files (en, fr), as JSON.
- `data/extracted/blueprints/`: stat-related blueprints, including their bytecode.
- `research/pseudo/`: decompiled blueprints. This is the reference for every formula.
- `data/assets/`: textures (webp) and fonts extracted from the game, served to the UI as static files.
- `crates/dr-core/`: the engine.
  - `vm.rs` interprets the extracted blueprint bytecode.
  - `engine.rs` wires the character like the game does.
  - `catalog.rs` holds the UI data (attribute names/formats/icons, stats panel layout from `UI_Character`, affixes, modifiers).
  - `planner.rs` exposes the save and build views.
  - `setup.rs` finds the game (Steam, Epic), its saves, the app's folders and the extracted builds.
- `crates/dr-cli/`: `dr-cli` (stats, comparison with the game) and `serve` (HTTP bridge).
- `app/`: the desktop UI (Tauri 2 + Svelte 5), which reproduces the in-game character screen.
  `app/src-tauri/src/gamesetup.rs` runs the setup (extraction, dumper) and serves the extracted textures and fonts
  (`drassets://` protocol).
- `research/validation/`: in-game references (screenshots, and a live UE4SS dump of the attribute arrays).

## Regenerating the mappings after a patch

The game cooks assets with unversioned properties, so each new build needs a fresh `.usmap`:

1. Download RE-UE4SS (`experimental-latest`, the `UE4SS_v*.zip` asset). Copy `dwmapi.dll` and `ue4ss/` into
   `ProjectAlpha/Binaries/Win64/`.
2. In that `ue4ss/Mods/` folder, set `mods.txt` to `Keybinds : 1` and `DRDump : 1`, and copy `tools/extract/ue4ss-dump/DRDump` in.
3. Set the Steam launch options to `WINEDLLOVERRIDES="dwmapi=n,b" %command%`. Start the game and load a character.
   About 15 s later DRDump writes `ue4ss/ProjectAlpha-<version>.usmap`.
4. Copy that file into `data/mappings/`. Then remove `dwmapi.dll`, `ue4ss/` and the launch option.
5. Run `tools/extract/extract.sh`.

## Languages

- **Game texts** (stats, items, skills, effects): they come from the game's own language files. `extract.sh` exports
  every `Game.locres` the game ships (de-DE, en, es-ES, fr, ja-JP, ko-KR, pt-BR, ru-RU, zh-Hans-CN). The backend reloads
  them with `set_language`; each language is loaded at most once per run.
- **Planner texts**: they live in `app/src/lib/locales/`.
  - `fr.ts` is the reference. `en.ts` must have all its keys.
  - A language without a dictionary shows English planner texts.
  - To add one, copy `en.ts`, translate it, and register it in `DICTS` in `app/src/lib/i18n.svelte.ts`.
  - `t('key', { name })` fills values. `{n|one|other}` picks a plural form with `Intl.PluralRules`.
  - `node app/tests/i18n-check.mjs` checks that every key used exists and that every dictionary is complete.
- The default language is French. The choice made in the top bar is kept (`localStorage`).

## Inventory and stash

The "Inventaire" button lists the character's inventory (`<id>.sav`, `InventoryItems`) and the home fort stash tabs
(world save `<id>_WRL.sav`, containers `BP_Container_Home_Fort_stash_..._Stash`, `_Stash1`...), laid out like in game.

- **Wanted stats**: each generated item keeps its affix pools (`AffixesPool`: Inherent, Pool1-3, rows of
  `DT_Base_Item_Attributes`), the affixes a reroll can give. An item scores the number of pools (1 to 3) that can roll a
  wanted stat (blue, yellow, red). "My priorities" adds the 5 rollable stats that bring the most on the objective.
- **Gain on the build**: each equipment item is made with the game's `Make Item Data by Handle` and tried in place of the
  item in its slot (rings: the better ring slot; weapons: the weapon in hand, and the stance follows it unless the build
  sets one). The gain is the change of the objective (~15 ms per item). "Try in the build" puts the item's stats in its
  slot.
- `cargo run --release -p dr-cli --example stash -- <save.sav> [n]` lists the tabs and evaluates the first n items.

## Progression history

- **What a snapshot holds**: level, combat rating, DPS total (normal, boss/elite, dummy reading), every attribute,
  the equipped items, the tree rows, the skill levels and the unspent points. Snapshots are stored in
  `~/.local/share/dr-planner/history/<save id>.jsonl`, one file per character.
- **When one is added**: when the app starts and every 2 minutes, the analysis worker checks every save. A save whose
  file time changed is computed as is. A new point is added only if the build changed (level, combat rating, DPS,
  items, tree or skills): XP alone does not count.
- **Gear sets**: players swap between a damage set and a farm set, so each point is classed by its equipped
  kill-loot mods (gold, extra elites, containers, item find). With 3 or more it is "Farm", else "DPS". A click
  changes the class (kept in `localStorage`). The curve follows one set (DPS by default) or both as two series,
  and each point's changes are listed against the previous point of the same set.
- **The History view**: a single curve with one axis (DPS boss/elite, DPS, combat rating, level or tree points),
  hover details, and a table of every point with what changed since the previous one (items, hearts, tree, skills).
- **From the command line** (`cargo run --release -p dr-cli --example history`):
  - `capture` adds the changed saves;
  - `import <sav> <key>` adds an older save copy, sorted by date;
  - `show <key>` lists a character's points.

## Measuring in game

The training dummy (`BP_Building_Dummy`) counts hits for 5 s from the first one. In boss mode it multiplies them by
the player's boss multiplier. The DPS tab shows the same reading ("mannequin 5 s").

To log every hit and calibrate an offhand ability:

1. Close the game. Run `tools/extract/ue4ss-dump/install.sh`. It installs UE4SS from the previous install's backup,
   adds the DRLiveDump and DRDamageLog mods, and needs the launch option `WINEDLLOVERRIDES="dwmapi=n,b" %command%`.
2. Hit the dummy with only that offhand. DRDamageLog writes `ue4ss/DRDamageLog.csv` with game time, damage causer,
   damage, type and crit for each hit.
3. Close the game and run `tools/extract/ue4ss-dump/install.sh --remove`. It saves the logs to
   `research/validation/live-<date>/` and removes UE4SS.
4. Run `python3 tools/validate/damagelog.py research/validation/live-<date>/DRDamageLog.csv`. Per causer class it gives
   instances, hits per instance, procs, hits per proc and time between procs: the numbers `proc_hits` needs.

## Build tools (what the app does)

- **Items**:
  - Edit, add and remove affixes, mods and Monograms on any slot.
  - Items may carry the same mod several times (stacks), so edits are made by attribute index.
- **Weapon swap**: any of the 296 game weapons (the `Items.Weapon.*` rows of `DT_Items`) can replace the one in hand. Its rarity, tier and hearts are kept.
  The item is built with the game's own `Make Item Data by Handle`, and the stance follows the weapon.
- **Gambler forge (hearts)**: `UI_ItemGambler.Upgrade` multiplies every stat of the item by 1.1 per heart.
  - The exceptions are the tags `CheckTagsForMonograms` skips: `Items.Modifiers`, `Attributes.GlobalModifiers`,
    `Attributes.Base.Mastery`, and `Attributes.Abilities.*` except `*.DamageMultiplier`.
  - Heart n+1 succeeds with probability (10 - n)/10, and a failure destroys the item.
  - The app can show the build without hearts, with 10 everywhere, or with hearts set per item.
- **Capacities**:
  - Offhand abilities follow `Try to Spawn Player Ability`: `Get Base Player Elemental Damage`
    (elemental damage x (1 + (1 + damage multiplier) x elemental damage %), i.e. ElementalDamage.Display on average)
    x `GetDamageMultiplierForOffhandAbility` (ability multiplier + bonus + affinity bonuses, x element multiplier;
    `GlobalModifiers.X.ToY` skills add one element's % to another).
    They crit only with the OffhandCrit mod: chance = total crit chance x 0.1 (the stance crit chance is part of the
    total, see `UpdateStanceDamage`), so crit chance counts up to 1000%. The `EleAsBasePhys` / `BonusEleAsBonusPhys`
    mods set their damage to 0.
  - Hits per proc: Lightning Plasma follows `BP_PlayerSkill_LightningPlasma` and `BP_LightningBeamSingle`.
    It spawns 1 + AdditionalBeam beams. With GrowLarger, each beam deals +5% per extra beam and level.
    Each beam ticks 8 + Ticks times (+GrowLarger, at most +3), every 0.33 s - 0.044 x GrowLarger (0.198 s from 3).
    Each tick deals x (1 + 0.1 x Ticks) and hits every overlapping enemy. Other abilities count one hit per proc
    Their actors are mini simulations: projectiles, geometry, summons with AI, randomness, and in Fire Beam a damage
    multiplier compounded on every timeline frame. They need measuring in game rather than reading.
    Procs fire on weapon hits: after the cooldown, the next hit (rate of the stance's basic combo) and the
    proc chance roll start the next one, so the time between procs is cooldown + (1/chance - 1/2) / hit rate.
    Check (boss dummy, 2026-10-29): estimated dummy reading 4.56 Qa/s, game meter 4.30 Qa/s.
  - Offhand cooldown (`Calculate Offhand Ability Cooldown`): the 1/2/3-offhand cooldown, x0.9 with a 4th offhand,
    / (1 + affinity cooldown bonus) / (1 + cooldown reduction).
  - The default objective is the DPS total of the DPS tab (chosen main attack + chosen offhands) against a boss or
    an elite. The combat rating weighs physical damage much more and misleads elemental builds.
  - Boss damage (`DamageSystem.Damage.Boss%`) applies to every enemy with an elite health bar, so bosses and elites
    (`BP_Enemy_Base.Damage Taken Main`: `is Elite Health Bar?` x `Get Character Boss Damage Multiplier`).
  - Weapon hits deal physical damage only: `TraceDealDamage_*` and `GetBaseDamage` use `Get Base Player Damage`.
    Elemental damage feeds offhand abilities and mods, not weapon hits.
  - Weapon attacks are estimated as the average physical hit (crit included) x attack multiplier x (1 + attack bonus).
    The multiplier itself is applied in combat actors that are not extracted yet.
  - Attack energy cost follows `Start Weapon Attack`: RequiredEnergy (x (1 + `SecondaryBuild.DoubleEnergyMoreDamage`)
    for secondary attacks) + reduction, or 100 with `ProcsTake100EnergyHighDamage`, halved with `EnergyConsumptionReduceHalf`.
- **Skill tree** (the main tree, `UI_SkillTree_S4realzies`, rows of `DT_GENERATED_SkillTree_Main`):
  - The root is always owned. A node costs 1 point and can be bought next to an owned node. It can be refunded
    only if every other owned node stays connected to the root (`IsGraphConnected_IgnoreNode`).
  - Dropdown nodes own one of the rows `<node>_<n>`. For example, `UI_SkillTreeNode_DropDown_6` is option 6 of
    `UI_SkillTreeNode_DropDown`, not the row of the node `UI_SkillTreeNode_DropDown_6`.
  - The budget is the points spent in the save plus its unspent `AvailableSkillPoints`.
  - A node's bonus is `GetSkillDataAttributes` at level 1. Each bonus is computed once by the VM; the tree is then summed
    tag by tag, on top of the cached total of the other skills (stance skills, cards, crafting). A whole save costs
    ~0.4 s in `MakeSkillsAttributes`; a tree edit costs ~30 ms.
  - The tree view ranks the available nodes by their gain on the objective (one probe per distinct bonus, ~360 probes),
    and a click on a distant node buys the shortest path to it.
- **Other skills** (stance skill tables `DT_Skills_<Stance>`, crystal cards, crafting): their levels can be edited
  freely in the "Postures & cartes" tab. The app checks neither stance points nor requirements. A skill's bonus is
  (Attribute + SkillLevels[0].BonusAttributes) x level. The view shows the gain of one more level and the loss of one
  less. Like the tree, each (table, row, level) bonus is computed once by the VM and then cached.
- **Priorities**:
  - Gains are marginal and computed at the build's current values, so diminishing returns are built in.
    For example, cooldown = base / (1 + cooldown reduction): the effective reduction (1 - 1 / (1 + x)) flattens,
    and the wait for the next weapon hit takes a growing share of the proc cycle.
  - The affix picker probes each choice with the roll it would have on the open item, not a typical roll.
  - Stats that can no longer grow are left out of the "keys" list and marked "max" in the panel. "Can no longer grow"
    means no rollable affix, no unowned tree node and no skill below its max level (gems are not counted).
    Example: projectile affinity bonuses, whose only source is the tree.
  - Element conversions (`GlobalModifiers.X.ToY`, e.g. "De l'arcane à la foudre" on a pet) make one element's %
    count for another. The DPS tab lists them with the item that grants them.
- **Conditional mods**: many mods only act in a game state the stat code checks. The app measures them with
  that state active and marks the gain with `*`. The states are: Bloodlust, Blood and Life, Phasing Trance,
  Light and Dark Shroud (buffs at max stacks), Unholy Void (`DamageCircle.Base`), Berserker Fury
  (`is Colossus Warrior Active?`), 100 rupture essences, and a ranged weapon. A mod that creates its state
  (AllowPhasing, Bloodlust.Base, Shroud, DamageCircle, Colossus.Base) is measured from "no state" to "state + mod".
  The states can also be set for real in "Active effects" (essences, Unholy Void, Berserker Fury, buffs).
  - Mods whose effect never shows in the stats get a label instead of a number: "loot" for gold, elites,
    containers, experience and gathering mods, "not quantified" for procs and ability behaviour.
  - A mod the stat code reads (`stat_mods`, from BP_AttributesComponent) that moves nothing shows 0 instead.
- **Objective and impact analysis**: the second worker adds one typical roll of each stat to the build and recomputes
  (about 5 ms per stat). It ranks affixes by their gain on the chosen objective (combat rating, an attack or an offhand)
  and can hide the stats that have no effect.

## Stat pipeline (build 24115719)

Everything below is in `BP_AttributesComponent` unless stated otherwise.

`CalculateAttributes` works in four steps:

1. It sums the attribute arrays, as tag-to-value pairs: `BaseAttributes`, `PlayerAttributes`, `EquipmentAttributes`,
   `EffectAttributes`, `SelectedItemAttributes`, then the racial bonuses.
   `PlayerAttributes` is `MakeSkillsAttributes(CharacterSkills) + Attributes` (the allocated stat points), from `BP_Character_Player.LoadCharacterData`.
2. It calls `Calculate Attribute Dependencies`:
   - stat buff mods;
   - `X% -> X` for the seven characteristics;
   - the `DT_Attributes` dependency graph, for example Luck gives +0.001 Fire%, Arcane%, Lightning% and XPBonus;
   - `CacheHighestStat`;
   - stance scaling, for example `Stance Scaling Spears` gives Agility -> Spear%, SpearCritDamage%, MaxHealth and Armor;
   - about 150 item-mod functions;
   - `Armor *= 1 + Armor%`, `AttackSpeed += 3 * Dexterity`, paragon, potions, phasing, shroud and bloodlust.
3. It calls `Calculate Stance Attributes(stance)`, then `PostStanceMods` and `Set Calculated Elemental Damage Bonus`.
4. It applies `ApplyAugmentation_CriticalDamage`.

Displayed damage comes from `BP_Character_Player.Get Base Player Damage FORCED(crit)`:

    Base.Damage * (1 + Damage.Multiplier) * (1 + StanceDamageMultiplier + Base.Damage% + (crit ? Base.CriticalDamage : 0))

Combat Rating comes from `BP_PlayerManagerComponent.Get Combat Rating Attribute`, with `c = clamp(Base.CriticalChance, 0, 1)`:

    (PhysDamage + ElementalDamage) * (1 - c) + PhysCritDamage * c

In the stats panel (`UI_Character`), each row is a `UI_Attribute_Value` widget bound to one attribute tag.
The row text comes from `BP_GameLibrary.GetAttributeValueText`.

## Licence

The planner's code is licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option. Unless you state otherwise, any contribution you submit for inclusion in this project, as defined in
the Apache-2.0 licence, is dual licensed as above, without any additional terms or conditions.

This licence covers the planner's own code only. It grants no right on Dwarven Realms or its content. Do not commit
game files or anything extracted from them (`data/extracted/`, `data/assets/`, `data/mappings/`, `research/`,
`.usmap`, `.sav`): `.gitignore` excludes them.
