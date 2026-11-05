Dwarven Realms Planner {version} - test version
================================================

Thank you for testing! This app shows your character's stats and damage,
and tells you which changes make your character stronger.

The first start takes about 5 minutes. After that, the app opens at once.
Follow the steps below in order.


STEP 1 - Unzip
--------------
Right-click the zip file, choose "Extract All...", then click "Extract".
You get a folder named "Dwarven Realms Planner".
Do not run the app from inside the zip: it will not work.


STEP 2 - Open the app
---------------------
Open the "Dwarven Realms Planner" folder and double-click
"Dwarven Realms Planner".

A blue window may say "Windows protected your PC". This is normal for a
test app. Click "More info", then click "Run anyway".


STEP 3 - Find the game
----------------------
The app opens on a page called "Game data", with three numbered parts:
1 (the game), 2 (mappings) and 3 (extraction). Steps 3, 4 and 5 below
follow them in order.
If the page is not in English, choose "English" in the box at the top
right of the page.
- If you see a green tick next to the game folder: go to step 4.
- If not: click "Choose the game folder..." and select your
  Dwarven Realms folder. In Steam: right-click the game, then
  "Manage" > "Browse local files" shows where it is.


STEP 4 - Read the game once (about 2 minutes)
---------------------------------------------
The app must look inside the running game one time.
The game must be CLOSED first, then OPEN, then CLOSED again:
1. Close the game if it is open.
2. In the app, click "Install the generator in the game".
   Wait until the list of steps appears.
3. Now start the game normally and load your character.
   (If the game was still open at step 2, close it and start it again.)
4. Wait about 15 seconds. In the app, the line
   "Mappings file found and imported." appears.
5. Close the game.
6. In the app, click "Remove UE4SS from the game".
   This puts the game back exactly as it was.

If your antivirus shows a warning during this step, it is about the
small helper tool the app adds to the game for a moment (UE4SS). It is
a well-known, free tool for this game engine, and step 6 removes it.


STEP 5 - Extract the data (about 1 minute)
-------------------------------------------
Click "Extract the data" and wait for the bar to fill.
Then click "Close". Done!


Using the app
-------------
- The app opens your last played character.
  To see another one, use "Character" at the top.
- Click an item to change it: the stats show what it changes.
- "Inventory" at the top shows your bag and your fort stash.
  Click "Evaluate the items" to see which items would make you stronger.
- "Session" at the top shows what you earned while playing, and your
  farming runs (elven fragments per run and per hour).
- Keep the app open while you play: it reads each save the game writes.
- The game only saves when you change map. To make it save without
  changing map: open your character's Equipment window and tick, then
  untick, "hide helmet". The app updates by itself a few seconds later.
- Text too small or too big? Press Ctrl and + or Ctrl and - (Ctrl and 0
  for automatic), or use the zoom box at the top right.
- To change the language, use the language box at the top right.
- After a game update, click "Game data" at the top, then
  "Extract the data" again.


Something went wrong?
---------------------
Tell me what you did and what you saw (a screenshot helps a lot).
If you can, also send this file:
  1. Press the Windows key + R.
  2. Type   %LOCALAPPDATA%\dr-planner   and press Enter.
  3. Send the file "app.log" from the folder that opens.

If the app window stays white or empty: install "Microsoft Edge
WebView2" (free, from Microsoft):
  https://developer.microsoft.com/microsoft-edge/webview2/
Choose "Evergreen Bootstrapper", run it, then open the app again.


Removing the app
----------------
Delete the "Dwarven Realms Planner" folder. To also delete its data:
press Windows key + R, type  %APPDATA%  and press Enter, then delete
the "dr-planner" folder. Do the same with  %LOCALAPPDATA% .


The app contains no game files: it reads them from your own copy of the
game, and they stay on your computer. Dwarven Realms belongs to Crater
Studios. This app is not made or supported by Crater Studios.
License: MIT or Apache-2.0 (see the LICENSE files).
