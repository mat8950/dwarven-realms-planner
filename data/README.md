# data/

This folder holds the game data the planner reads. It is **not** in the repository: the data
belongs to Crater Studios and is extracted from your own installation of Dwarven Realms.

- `mappings/`: the `.usmap` mappings of your game build (see "Regenerating the mappings" in the main README).
- `extracted/`: DataTables, string tables, blueprints and language files, as JSON.
- `assets/`: icons, textures and fonts, converted for the UI.

Developers fill it with `tools/extract/extract.sh`.
