// dr-extract: exports the Dwarven Realms data the planner needs, from the player's own game files.
//
// usage:
//   dr-extract game <gameDir> <outRoot> --usmap <file> [--build <id>] [--cache <dir>]
//       Reads <gameDir>/ProjectAlpha/Content/Paks and writes <outRoot>/data/extracted and <outRoot>/data/assets
//       (the layout dr-core's GameData reads). Progress goes to stdout as one JSON object per line:
//       {"step":"tables","done":12,"total":340}, then {"step":"done"}; a fatal error is {"error":"..."} and exit 1.
//       Oodle (the pak compression) is downloaded once into --cache and checked against a SHA-256.
//   dr-extract dumper install|remove|status <gameDir>
//       Installs RE-UE4SS (latest experimental release, checked against GitHub's SHA-256) with the DRDump mod in
//       the game, which writes the mappings (.usmap) once a character is loaded; or removes what it installed.
//   dr-extract <root> <outDir> <filters> [json|assets]
//       Developer mode used by tools/extract/extract.sh, on files unpacked by repak.
//       <filters>: comma-separated substrings of package paths, or @file with one per line.
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using CUE4Parse.Compression;
using CUE4Parse.FileProvider;
using CUE4Parse.MappingsProvider.Usmap;
using CUE4Parse.UE4.Assets.Exports.Engine.Font;
using CUE4Parse.UE4.Assets.Exports.Texture;
using CUE4Parse.UE4.Localization;
using CUE4Parse.UE4.Readers;
using CUE4Parse.UE4.Versions;
using CUE4Parse_Conversion.Options;
using CUE4Parse_Conversion.Textures;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using OodleDotNet;

if (args.Length > 0 && args[0] == "game") return GameExport.Run(args[1..]);
if (args.Length > 0 && args[0] == "dumper") return Dumper.Run(args[1..]);
return Legacy.Run(args);

static class Legacy {
  public static int Run(string[] args) {
    var root = args[0]; var outDir = args[1];
    var filters = args[2].StartsWith("@") ? File.ReadAllLines(args[2][1..]).Where(l => l.Length > 0).ToArray() : args[2].Split(',');
    var mode = args.Length > 3 ? args[3] : "json";

    var provider = new DefaultFileProvider(root, SearchOption.AllDirectories, new VersionContainer(EGame.GAME_UE5_3));
    if (Environment.GetEnvironmentVariable("USMAP") is string m && File.Exists(m)) provider.MappingsContainer = new FileUsmapTypeMappingsProvider(m);
    provider.ReadScriptData = true;
    provider.Initialize();
    provider.Mount();

    var exact = new HashSet<string>(filters.Where(f => f.EndsWith(".uasset")), StringComparer.OrdinalIgnoreCase);
    bool Wanted(string k) => exact.Contains(k) || filters.Any(f => !f.EndsWith(".uasset") && k.Contains(f, StringComparison.OrdinalIgnoreCase));

    int ok = 0, fail = 0;
    foreach (var f in provider.Files.Keys.Where(Wanted).ToList()) {
      try {
        if (mode == "assets") {
          if (!f.EndsWith(".uasset")) continue;
          foreach (var e in provider.LoadPackage(f).GetExports()) {
            var basePath = Path.Combine(outDir, Path.ChangeExtension(f, null));
            if (e is UTexture tex) {
              var bmp = tex.Decode(ETexturePlatform.DesktopMobile);
              if (bmp == null) continue;
              var bytes = TextureEncoder.Encode(bmp, ETextureFormat.Png, false, out var ext, 100);
              Directory.CreateDirectory(Path.GetDirectoryName(basePath));
              File.WriteAllBytes(basePath + ".png", bytes); ok++;
            } else if (e is UFontFace face && face.FontFaceData?.Data is byte[] data && data.Length > 0) {
              Directory.CreateDirectory(Path.GetDirectoryName(basePath));
              File.WriteAllBytes(basePath + ".ttf", data); ok++;
            }
          }
          continue;
        }
        object obj;
        if (f.EndsWith(".locres")) obj = new FTextLocalizationResource(new FByteArchive(f, provider.Files[f].Read()));
        else if (f.EndsWith(".uasset")) obj = provider.LoadPackage(f).GetExports();
        else continue;
        var p = Path.Combine(outDir, f.Replace(".uasset", ".json").Replace(".locres", ".locres.json"));
        Directory.CreateDirectory(Path.GetDirectoryName(p));
        File.WriteAllText(p, JsonConvert.SerializeObject(obj, Formatting.Indented)); ok++;
      } catch (Exception e) { fail++; if (fail < 8) Console.WriteLine($"FAIL {f}: {e.Message}"); }
    }
    Console.WriteLine($"ok={ok} fail={fail}");
    return 0;
  }
}

/// The whole extraction for the app: what tools/extract/extract.sh, anims.py, skilltree.py and textures.py do,
/// read straight from the game's pak.
static class GameExport {
  const string Content = "ProjectAlpha/Content/";

  // Folders the stat code and the UI read from (the ones extract.sh unpacks).
  static readonly string[] Roots = [
    "EasySurvivalRPG/Blueprints/", "EasySurvivalRPG/Pets/", "EasySurvivalRPG/Skins/", "StylizedTigerMount/Blueprints/",
    "Blueprints/", "ElementalInfusion/", "LocalizationSource/", "Localization/",
  ];
  static readonly string[] Tables = ["/DT_", "LocalizationSource/", "/Game.locres", "/STR_", "/E_"];
  // Blueprints whose bytecode holds the stat formulas.
  static readonly string[] Blueprints = [
    "/BP_AttributesComponent.", "/BP_Character_Player.", "/BP_Character_Base.", "/BP_PlayerManagerComponent.", "/BP_GameLibrary.",
    "/BP_ItemsLibrary.", "/UI_Character.", "/BP_PlayerStanceSkillManagerComponent.", "/UI_ItemToolTip.", "/BP_StatusEffect_Base.",
    "/BP_AbilitySystemComponent.", "/BP_RaceManager.", "/BP_PetManager.", "/BP_ContainerComponent.", "/BP_EquipmentComponent.",
    "/BP_HotbarComponent.", "/BP_PlayerController.", "/BP_PotionManager.", "/UI_CharInfo_Cards_Grid.", "/BP_RuptureOrbsComponent.",
  ];
  const string SkillTreeWidget = "EasySurvivalRPG/Blueprints/UserInterface/SkillTree_S3/UI_SkillTree_S4realzies.uasset";
  // The elven tree (the elven harp): its slots name the DT_Skills_Crafting_S24 rows the game offers.
  const string ElvenTreeWidget = "EasySurvivalRPG/Blueprints/DataTables/StanceSkills/UI_CraftingSkillTree_S24.uasset";
  /// Version of the extracted data's layout. dr-core asks for a new extraction below its DATA_FORMAT.
  const int Format = 3;
  static readonly string[] AnimFolders = ["Combat_Animations/WeaponBasicAttacks/", "UnarmedAutoAttacks/"];
  // Whole texture folders: UI chrome (slot silhouettes, rarity backgrounds, frames) and item modifier icons.
  static readonly string[] TextureFolders = ["EasySurvivalRPG/Textures/UI/", "EasySurvivalRPG/Textures/Fonts/", "EasySurvivalRPG/Blueprints/Equipment_Modifiers/"];
  static readonly string[] Fonts = ["CormorantUnicase-Bold", "MasonSanAltBol", "Petrona-VariableFont_wght", "AlegreyaSans-Bold"];

  public static int Run(string[] a) {
    try {
      if (a.Length < 2) throw new ArgumentException("usage: dr-extract game <gameDir> <outRoot> --usmap <file> [--build <id>] [--cache <dir>]");
      string gameDir = a[0], outRoot = Path.GetFullPath(a[1]);
      string Opt(string name) { var i = Array.IndexOf(a, name); return i >= 0 && i + 1 < a.Length ? a[i + 1] : null; }
      var usmap = Opt("--usmap") ?? throw new ArgumentException("--usmap is required: the game cooks its assets without property names");
      var cache = Opt("--cache") ?? Path.Combine(Path.GetTempPath(), "dr-extract");
      var build = Opt("--build") ?? SteamBuild(gameDir) ?? "unknown";
      var paks = Path.Combine(gameDir, "ProjectAlpha", "Content", "Paks");
      if (!Directory.Exists(paks)) throw new DirectoryNotFoundException($"no ProjectAlpha/Content/Paks folder in {gameDir}");
      if (!File.Exists(usmap)) throw new FileNotFoundException($"mappings file not found: {usmap}");

      Step("oodle");
      OodleHelper.Initialize(new Oodle(OodleLibrary(cache)));

      Step("mount");
      var provider = new DefaultFileProvider(paks, SearchOption.TopDirectoryOnly, new VersionContainer(EGame.GAME_UE5_3));
      provider.MappingsContainer = new FileUsmapTypeMappingsProvider(usmap);
      provider.ReadScriptData = true;
      provider.Initialize();
      provider.Mount();
      var keys = provider.Files.Keys.Where(k => k.StartsWith(Content, StringComparison.OrdinalIgnoreCase)).ToList();
      if (keys.Count == 0) throw new InvalidDataException($"no game files found in {paks}");
      string Rel(string k) => k[Content.Length..];

      var tmp = outRoot + ".partial";
      if (Directory.Exists(tmp)) Directory.Delete(tmp, true);
      var extracted = Path.Combine(tmp, "data", "extracted");
      var assets = Path.Combine(tmp, "data", "assets");

      bool InRoots(string k) => Roots.Any(r => Rel(k).StartsWith(r, StringComparison.OrdinalIgnoreCase));
      bool Matches(string k, string[] filters) => filters.Any(f => k.Contains(f, StringComparison.OrdinalIgnoreCase));
      bool Exportable(string k) => k.EndsWith(".uasset") || k.EndsWith(".locres");

      ExportJson(provider, keys.Where(k => Exportable(k) && InRoots(k) && Matches(k, Tables)).ToList(), Path.Combine(extracted, "tables"), "tables", Rel);
      ExportJson(provider, keys.Where(k => Exportable(k) && InRoots(k) && Matches(k, Blueprints)).ToList(), Path.Combine(extracted, "blueprints"), "blueprints", Rel);

      Step("animations");
      var tables = Path.Combine(extracted, "tables");
      var montages = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
      foreach (var row in Rows(tables, "EasySurvivalRPG/Blueprints/Misc/WeaponAttacksSystem/DT_WeaponAttacks.json"))
        foreach (var anim in ((JObject)row.Value).Properties().Where(p => p.Name.StartsWith("Animation")).Select(p => p.Value).OfType<JObject>())
          foreach (var p in anim.Properties().Where(p => p.Name.StartsWith("Anim") && p.Value is JObject))
            if (At(p.Value, "ObjectPath")?.ToString() is string path && path.StartsWith("/Game/")) montages.Add(Content + path.Split('.')[0]["/Game/".Length..] + ".uasset");
      var animKeys = keys.Where(k => montages.Contains(k) || (k.EndsWith(".uasset") && AnimFolders.Any(f => Rel(k).StartsWith(f, StringComparison.OrdinalIgnoreCase)))).ToList();
      var anims = new SortedDictionary<string, JObject>(StringComparer.Ordinal);
      Each(animKeys, "animations", k => {
        if (AnimEntry(Exports(provider, k)) is JObject entry) anims["/Game/" + Path.ChangeExtension(Rel(k), null)] = entry;
      });
      Write(Path.Combine(extracted, "attack_anims.json"), new JObject(anims.Select(kv => new JProperty(kv.Key, kv.Value))));

      Step("skill_tree");
      Write(Path.Combine(extracted, "skill_tree.json"), SkillTree(Exports(provider, Content + SkillTreeWidget)));
      Step("elven_tree");
      Write(Path.Combine(extracted, "elven_tree.json"), ElvenTree(Exports(provider, Content + ElvenTreeWidget)));

      var textures = TexturePaths(tables, keys, Rel);
      Each(textures, "textures", k => {
        foreach (var e in provider.LoadPackage(k).GetExports()) {
          if (e is not UTexture tex) continue;
          var bmp = tex.Decode(ETexturePlatform.DesktopMobile);
          if (bmp == null) continue;
          var bytes = TextureEncoder.Encode(bmp, ETextureFormat.Webp, false, out _, 90);
          var dst = Path.Combine(assets, Path.ChangeExtension(Rel(k), ".webp"));
          Directory.CreateDirectory(Path.GetDirectoryName(dst));
          File.WriteAllBytes(dst, bytes);
        }
      });
      Step("fonts");
      Directory.CreateDirectory(Path.Combine(assets, "fonts"));
      foreach (var f in Fonts) {
        var k = $"{Content}EasySurvivalRPG/Textures/Fonts/{f}.ufont";
        if (provider.Files.TryGetValue(k, out var file)) File.WriteAllBytes(Path.Combine(assets, "fonts", f + ".ttf"), file.Read());
        else Warn($"font not found: {k}");
      }

      File.WriteAllText(Path.Combine(extracted, "FORMAT"), $"{Format}\n");
      File.WriteAllText(Path.Combine(extracted, "BUILD"), $"build {build}\n");
      if (Directory.Exists(outRoot)) Directory.Delete(outRoot, true);
      Directory.CreateDirectory(Path.GetDirectoryName(outRoot));
      Directory.Move(tmp, outRoot);
      Emit(new JObject { ["step"] = "done", ["build"] = build });
      return 0;
    } catch (Exception e) {
      Emit(new JObject { ["error"] = e.Message });
      return 1;
    }
  }

  static void Emit(JObject o) { Console.Out.WriteLine(o.ToString(Formatting.None)); Console.Out.Flush(); }
  static void Step(string step) => Emit(new JObject { ["step"] = step });
  static void Warn(string message) => Emit(new JObject { ["warn"] = message });

  /// Runs `work` on each key, reporting progress about every 2%. A file that fails is reported and skipped;
  /// more than 5% failures is fatal (wrong mappings for this game build, most likely).
  static void Each(List<string> keys, string step, Action<string> work) {
    int done = 0, failed = 0, every = Math.Max(1, keys.Count / 50);
    Emit(new JObject { ["step"] = step, ["done"] = 0, ["total"] = keys.Count });
    foreach (var k in keys) {
      try { work(k); } catch (Exception e) {
        failed++;
        if (failed <= 5) Warn($"{k}: {e.Message}");
      }
      if (++done % every == 0 || done == keys.Count) Emit(new JObject { ["step"] = step, ["done"] = done, ["total"] = keys.Count });
    }
    if (failed > Math.Max(3, keys.Count / 20))
      throw new InvalidDataException($"{failed} of {keys.Count} {step} files could not be read: the mappings (.usmap) probably do not match this game build");
  }

  static void ExportJson(DefaultFileProvider provider, List<string> keys, string outDir, string step, Func<string, string> rel) =>
    Each(keys, step, k => {
      object obj = k.EndsWith(".locres") ? new FTextLocalizationResource(new FByteArchive(k, provider.Files[k].Read())) : provider.LoadPackage(k).GetExports();
      var p = Path.Combine(outDir, rel(k).Replace(".uasset", ".json").Replace(".locres", ".locres.json"));
      Directory.CreateDirectory(Path.GetDirectoryName(p));
      File.WriteAllText(p, LegacyPaths(JsonConvert.SerializeObject(obj, Formatting.Indented)));
    });

  static readonly System.Text.RegularExpressions.Regex ResolvedPath = new("\"ObjectPath\": \"ProjectAlpha/Content/([^\"]+?)\\.\\d+\"");

  /// CUE4Parse writes a reference to a package it can read as "ProjectAlpha/Content/<path>.<export index>", and
  /// one it cannot read as "/Game/<path>". extract.sh exports the tables and blueprints with only the Roots folders
  /// unpacked, and dr-core expects that: references outside Roots keep the "/Game/<path>" form.
  static string LegacyPaths(string json) => ResolvedPath.Replace(json, m =>
    Roots.Any(r => m.Groups[1].Value.StartsWith(r, StringComparison.OrdinalIgnoreCase)) ? m.Value : $"\"ObjectPath\": \"/Game/{m.Groups[1].Value}\"");

  static JArray Exports(DefaultFileProvider provider, string key) => JArray.Parse(JsonConvert.SerializeObject(provider.LoadPackage(key).GetExports()));

  static void Write(string path, JToken value) {
    Directory.CreateDirectory(Path.GetDirectoryName(path));
    File.WriteAllText(path, value.ToString(Formatting.Indented));
  }

  static IEnumerable<JProperty> Rows(string tables, string rel) {
    var path = Path.Combine(tables, rel);
    if (!File.Exists(path)) return [];
    return (At(JArray.Parse(File.ReadAllText(path)).FirstOrDefault(), "Rows") as JObject)?.Properties() ?? [];
  }

  /// `t[a][b]...`, or null where a step is not an object (Python's `(x.get(a) or {}).get(b)`).
  static JToken At(JToken t, params string[] path) {
    foreach (var k in path) t = (t as JObject)?[k];
    return t;
  }
  static JObject Obj(JToken t) => t is JObject o && o.HasValues ? o : null;

  static bool IsNumber(JToken t) => t != null && (t.Type == JTokenType.Float || t.Type == JTokenType.Integer);
  static double Num(JToken t, double fallback) => IsNumber(t) ? t.Value<double>() : fallback;

  /// Timing of one attack animation (port of anims.py): length, rate, the FinishAbility notify (weapon skills:
  /// when the next attack can start) and the start of the ContinueAttack window (basic combos: the next hit).
  static JObject AnimEntry(JArray exports) {
    var seq = exports.OfType<JObject>().FirstOrDefault(e => e["Type"]?.ToString() == "AnimSequence");
    if (seq == null) return null;
    var byName = new Dictionary<string, JObject>();
    foreach (var e in exports.OfType<JObject>()) if (e["Name"]?.ToString() is string n) byName[n] = e;
    var p = seq["Properties"] as JObject ?? [];
    double length = Num(p["SequenceLength"], 0.0);
    var entry = new JObject { ["length"] = length, ["rate"] = Num(p["RateScale"], 1.0) };
    double? finish = null, cont = null;
    foreach (var n in (p["Notifies"] as JArray ?? []).OfType<JObject>()) {
      var link = n["LinkValue"];
      if (!IsNumber(link)) continue;
      var t = link.Value<double>();
      var reference = Obj(n["Notify"]) ?? Obj(n["NotifyStateClass"]);
      var name = reference?["ObjectName"]?.ToString() ?? "";
      if (name.Contains("FinishAbility")) finish = Math.Min(finish ?? t, t);
      if (name.Contains("NotifyState_ContinueAttack")) {
        var obj = byName.GetValueOrDefault(name.Split(':')[^1].TrimEnd('\''));
        var next = At(obj, "Properties", "NextAnim") as JObject ?? [];
        var anim = next.Properties().FirstOrDefault(x => x.Name.StartsWith("Anim"))?.Value as JObject;
        if (anim?["ObjectPath"]?.ToString() is string path && path.Length > 0 && (cont == null || t < cont)) {
          cont = t;
          entry["continue"] = t;
          entry["next"] = GamePath(path);
          entry["next_play_rate"] = next.Properties().FirstOrDefault(x => x.Name.StartsWith("InPlayRate"))?.Value ?? 1.0;
          entry["next_attack_speed"] = next.Properties().FirstOrDefault(x => x.Name.StartsWith("AffectedByAttackSpeed"))?.Value ?? true;
        }
      }
    }
    entry["finish"] = finish ?? length;
    return new JObject(entry.Properties().OrderBy(x => x.Name, StringComparer.Ordinal));
  }

  static string GamePath(string p) {
    p = p.Split('.')[0];
    var i = p.IndexOf("Content/", StringComparison.Ordinal);
    return i >= 0 ? "/Game/" + p[(i + "Content/".Length)..] : p;
  }

  /// Main skill tree layout (port of skilltree.py). Each node widget sits in a CanvasPanelSlot (top-left offset,
  /// auto size = NodeSize). Slot names repeat across canvases, so each node's slot is found by its content.
  static JObject SkillTree(JArray objs) {
    const double defaultSize = 30.0; // NodeSize of the UI_SkillTreeNode_Base / _DropDown class defaults
    var slots = new Dictionary<string, JObject>();
    foreach (var o in objs.OfType<JObject>().Where(o => o["Type"]?.ToString() == "CanvasPanelSlot")) {
      var content = At(o, "Properties", "Content", "ObjectName")?.ToString() ?? "";
      slots[content.Split('.')[^1].TrimEnd('\'')] = o["Properties"] as JObject ?? [];
    }
    var nodes = new JArray();
    foreach (var o in objs.OfType<JObject>()) {
      var kind = o["Type"]?.ToString() switch { "UI_SkillTreeNode_Base_C" => "base", "UI_SkillTreeNode_DropDown_C" => "dropdown", _ => null };
      if (kind == null) continue;
      var p = o["Properties"] as JObject ?? [];
      var slot = slots.GetValueOrDefault(o["Name"]?.ToString() ?? "") ?? [];
      var off = At(slot, "LayoutData", "Offsets") as JObject ?? [];
      var size = p["NodeSize"] as JObject ?? [];
      nodes.Add(new JObject {
        ["id"] = o["Name"],
        ["kind"] = kind,
        ["x"] = Num(off["Left"], 0.0),
        ["y"] = Num(off["Top"], 0.0),
        ["w"] = Num(size["X"], defaultSize),
        ["h"] = Num(size["Y"], defaultSize),
        ["root"] = p["IsRootNode"]?.Type == JTokenType.Boolean && p["IsRootNode"].Value<bool>(),
        ["links"] = p["ConnectedNodes"] ?? new JArray(),
        ["category"] = At(p, "Category", "RowName") ?? JValue.CreateNull(),
      });
    }
    if (nodes.Count == 0) throw new InvalidDataException("no skill tree node found in UI_SkillTree_S4realzies");
    return new JObject { ["nodes"] = nodes };
  }

  /// Elven tree slots (port of elventree.py): each UI_SkillSlot names its skill row; its CanvasPanelSlot (found by
  /// content, slot names repeat) gives its position. The table has rows the tree does not show.
  static JObject ElvenTree(JArray objs) {
    var slots = new Dictionary<string, JObject>();
    foreach (var o in objs.OfType<JObject>().Where(o => o["Type"]?.ToString() == "CanvasPanelSlot")) {
      var content = At(o, "Properties", "Content", "ObjectName")?.ToString() ?? "";
      slots[content.Split('.')[^1].TrimEnd('\'')] = o["Properties"] as JObject ?? [];
    }
    var outSlots = new JArray();
    foreach (var o in objs.OfType<JObject>().Where(o => o["Type"]?.ToString() == "UI_SkillSlot_C")) {
      var row = At(o, "Properties", "SkillHandle", "RowName")?.ToString();
      if (string.IsNullOrEmpty(row) || row == "None") continue;
      var off = At(slots.GetValueOrDefault(o["Name"]?.ToString() ?? ""), "LayoutData", "Offsets") as JObject ?? [];
      outSlots.Add(new JObject { ["id"] = o["Name"], ["row"] = row, ["x"] = Num(off["Left"], 0.0), ["y"] = Num(off["Top"], 0.0) });
    }
    if (outSlots.Count == 0) throw new InvalidDataException("no skill slot found in UI_CraftingSkillTree_S24");
    return new JObject { ["slots"] = outSlots };
  }

  /// The textures the planner UI shows (port of textures.py): icons of item attributes, abilities, weapon
  /// attacks, skills, status effects, rupture fortunes, equipment and resources, plus whole UI folders.
  static List<string> TexturePaths(string tables, List<string> keys, Func<string, string> rel) {
    var wanted = new HashSet<string> { "/Game/EasySurvivalRPG/Textures/New_Icons/dwarven-heart" };
    void Icons(JToken o) {
      if (o is JObject obj) {
        foreach (var p in obj.Properties()) {
          if (p.Name.StartsWith("Icon") && p.Value is JObject v && v["ObjectPath"]?.ToString() is string path && path.Length > 0) wanted.Add(path);
          Icons(p.Value);
        }
      } else if (o is JArray arr) foreach (var v in arr) Icons(v);
    }
    void Table(string r) { foreach (var row in Rows(tables, r)) Icons(row.Value); }
    Table("EasySurvivalRPG/Blueprints/DataTables/GeneratedItems/DT_Base_Item_Attributes.json");
    Table("EasySurvivalRPG/Blueprints/Abilities/SkillAbilities/Localized/DT_PlayerAbilities.json");
    Table("EasySurvivalRPG/Blueprints/Misc/WeaponAttacksSystem/DT_WeaponAttacks.json");
    Table("EasySurvivalRPG/Blueprints/UserInterface/SkillTree_S3/SkillDataTables/DT_GENERATED_SkillTree_Main.json");
    var stance = Path.Combine(tables, "EasySurvivalRPG/Blueprints/DataTables/StanceSkills");
    if (Directory.Exists(stance))
      foreach (var f in Directory.GetFiles(stance, "DT_*Skills*.json").OrderBy(f => f, StringComparer.Ordinal)) Table(Path.GetRelativePath(tables, f));
    Table("EasySurvivalRPG/Blueprints/DataTables/StanceSkills/Localized/DT_Skills_Crafting_S24.json");
    Table("EasySurvivalRPG/Blueprints/DataTables/LocalizedDataTables/DT_StatusEffects.json");
    Table("EasySurvivalRPG/Blueprints/Zones/Fortune/Data/DT_RuptureFortunes.json");
    foreach (var row in Rows(tables, "EasySurvivalRPG/Blueprints/DataTables/DT_Items.json")) {
      var tags = string.Concat(((JObject)row.Value).Properties().Where(p => p.Name.StartsWith("ItemTags")).Select(p => p.Value.ToString(Formatting.None)));
      if (tags.Contains("Items.Equipment") || tags.Contains("Items.Weapon") || tags.Contains("Items.Resource")) Icons(row.Value);
    }

    var all = new HashSet<string>(keys, StringComparer.OrdinalIgnoreCase);
    var outSet = new SortedSet<string>(StringComparer.Ordinal);
    // References are `/Game/A/B.B`, or `ProjectAlpha/Content/A/B.0` for assets inside the exported folders.
    foreach (var p in wanted) {
      var path = p.Split('.')[0];
      var rest = path.StartsWith("/Game/") ? path["/Game/".Length..] : path.StartsWith(Content) ? path[Content.Length..] : null;
      if (rest != null && all.Contains(Content + rest + ".uasset")) outSet.Add(Content + rest + ".uasset");
    }
    string[] skip = ["BP_", "UI_", "DT_", "STR_", "E_"];
    foreach (var k in keys)
      if (k.EndsWith(".uasset") && TextureFolders.Any(d => rel(k).StartsWith(d, StringComparison.OrdinalIgnoreCase)) && !skip.Any(s => Path.GetFileName(k).StartsWith(s)))
        outSet.Add(k);
    return [.. outSet];
  }

  /// Build id from the Steam manifest next to the game folder (steamapps/appmanifest_2015240.acf).
  static string SteamBuild(string gameDir) {
    try {
      var acf = Path.Combine(gameDir, "..", "..", "appmanifest_2015240.acf");
      if (!File.Exists(acf)) return null;
      foreach (var line in File.ReadLines(acf)) {
        var parts = line.Split('"', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries).Where(s => s.Length > 0).ToArray();
        if (parts.Length >= 2 && parts[0] == "buildid") return parts[1];
      }
    } catch (IOException) { }
    return null;
  }

  // Oodle 2.9.10 from the OodleUE mirror of Epic's SDK (the build repak uses), checked against its SHA-256.
  const string OodleUrl = "https://github.com/WorkingRobot/OodleUE/raw/refs/heads/main/Engine/Source/Programs/Shared/EpicGames.Oodle/Sdk/2.9.10/";

  static string OodleLibrary(string cacheDir) {
    var (path, name, sha) =
      OperatingSystem.IsWindows() ? ("win/redist", "oo2core_9_win64.dll", "6f5d41a7892ea6b2db420f2458dad2f84a63901c9a93ce9497337b16c195f457")
      : OperatingSystem.IsMacOS() ? ("mac/lib", "liboo2coremac64.2.9.10.dylib", "b09af35f6b84a61e2b6488495c7927e1cef789b969128fa1c845e51a475ec501")
      : RuntimeInformation.ProcessArchitecture == Architecture.Arm64 ? ("linuxarm/lib", "liboo2corelinuxarm64.so.9", "161a8ecca8cc2d4ea6469779c2cc529ed5bb2765d99466273c29fdbef4657374")
      : ("linux/lib", "liboo2corelinux64.so.9", "ed7e98f70be1254a80644efd3ae442ff61f854a2fe9debb0b978b95289884e9c");
    Directory.CreateDirectory(cacheDir);
    var file = Path.Combine(cacheDir, name);
    static string Sha(byte[] b) => Convert.ToHexString(SHA256.HashData(b)).ToLowerInvariant();
    if (File.Exists(file) && Sha(File.ReadAllBytes(file)) == sha) return file;
    using var http = new HttpClient { Timeout = TimeSpan.FromMinutes(2) };
    var bytes = http.GetByteArrayAsync(OodleUrl + path + "/" + name).GetAwaiter().GetResult();
    if (Sha(bytes) != sha) throw new InvalidDataException($"downloaded {name} does not match its expected SHA-256");
    File.WriteAllBytes(file, bytes);
    return file;
  }
}

/// The mappings dumper: RE-UE4SS plus the DRDump mod (tools/extract/ue4ss-dump/DRDump), in the game's Win64 folder.
/// Only what it installed is removed: the list is kept in dr-planner-ue4ss.json next to the game executable.
static class Dumper {
  const string Release = "https://api.github.com/repos/UE4SS-RE/RE-UE4SS/releases/tags/experimental-latest";
  const string ManifestName = "dr-planner-ue4ss.json";

  static void Emit(JObject o) { Console.Out.WriteLine(o.ToString(Formatting.None)); Console.Out.Flush(); }

  public static int Run(string[] a) {
    try {
      if (a.Length < 2) throw new ArgumentException("usage: dr-extract dumper install|remove|status <gameDir>");
      var win64 = Path.Combine(a[1], "ProjectAlpha", "Binaries", "Win64");
      if (!Directory.Exists(win64)) throw new DirectoryNotFoundException($"no ProjectAlpha/Binaries/Win64 folder in {a[1]}");
      var manifest = Path.Combine(win64, ManifestName);
      switch (a[0]) {
        case "status":
          Emit(new JObject { ["installed"] = File.Exists(manifest), ["running"] = GameRunning() });
          return 0;
        case "install":
          Install(win64, manifest);
          return 0;
        case "remove":
          Remove(win64, manifest);
          return 0;
        default:
          throw new ArgumentException($"unknown dumper command {a[0]}");
      }
    } catch (Exception e) {
      Emit(new JObject { ["error"] = e.Message });
      return 1;
    }
  }

  /// The game process (Windows, or Wine/Proton where the process name is cut to 15 characters).
  static bool GameRunning() =>
    System.Diagnostics.Process.GetProcesses().Any(p => { try { return p.ProcessName.StartsWith("ProjectAlpha-Wi", StringComparison.OrdinalIgnoreCase); } catch { return false; } });

  static void Install(string win64, string manifest) {
    if (GameRunning()) throw new InvalidOperationException("the game is running: close it first");
    if (!File.Exists(manifest) && (File.Exists(Path.Combine(win64, "dwmapi.dll")) || Directory.Exists(Path.Combine(win64, "ue4ss"))))
      throw new InvalidOperationException("UE4SS is already installed in this game folder: add the DRDump mod to it by hand (see the README)");

    Emit(new JObject { ["step"] = "download" });
    using var http = new HttpClient { Timeout = TimeSpan.FromMinutes(5) };
    http.DefaultRequestHeaders.UserAgent.ParseAdd("dr-planner");
    var release = JObject.Parse(http.GetStringAsync(Release).GetAwaiter().GetResult());
    var asset = (release["assets"] as JArray ?? []).OfType<JObject>()
      .FirstOrDefault(x => x["name"]?.ToString() is string n && n.StartsWith("UE4SS_v") && n.EndsWith(".zip"))
      ?? throw new InvalidDataException("no UE4SS_v*.zip in the latest RE-UE4SS experimental release");
    var bytes = http.GetByteArrayAsync(asset["browser_download_url"].ToString()).GetAwaiter().GetResult();
    var digest = asset["digest"]?.ToString();
    if (digest != null && digest.StartsWith("sha256:")) {
      var sha = Convert.ToHexString(SHA256.HashData(bytes)).ToLowerInvariant();
      if (sha != digest["sha256:".Length..]) throw new InvalidDataException($"{asset["name"]} does not match its SHA-256");
    }

    Emit(new JObject { ["step"] = "install" });
    // A reinstall keeps the list of the first install (those files exist now, but are still ours).
    var created = File.Exists(manifest)
      ? (JObject.Parse(File.ReadAllText(manifest))["files"] as JArray ?? []).Select(f => f.ToString()).ToList()
      : new List<string>();
    using (var zip = new System.IO.Compression.ZipArchive(new MemoryStream(bytes))) {
      foreach (var top in zip.Entries.Select(e => e.FullName.Split('/')[0]).Distinct())
        if (!created.Contains(top) && !File.Exists(Path.Combine(win64, top)) && !Directory.Exists(Path.Combine(win64, top))) created.Add(top);
      var root = Path.GetFullPath(win64) + Path.DirectorySeparatorChar;
      foreach (var e in zip.Entries) {
        var dst = Path.GetFullPath(Path.Combine(win64, e.FullName));
        if (!dst.StartsWith(root, StringComparison.Ordinal)) throw new InvalidDataException($"unsafe path in the archive: {e.FullName}");
        if (e.FullName.EndsWith('/')) { Directory.CreateDirectory(dst); continue; }
        Directory.CreateDirectory(Path.GetDirectoryName(dst));
        System.IO.Compression.ZipFileExtensions.ExtractToFile(e, dst, true);
      }
    }
    var mods = Path.Combine(win64, "ue4ss", "Mods");
    var script = Path.Combine(mods, "DRDump", "Scripts", "main.lua");
    Directory.CreateDirectory(Path.GetDirectoryName(script));
    using (var src = typeof(Dumper).Assembly.GetManifestResourceStream("DRDump.main.lua") ?? throw new InvalidDataException("DRDump mod missing from dr-extract"))
    using (var dst = File.Create(script)) src.CopyTo(dst);
    // Only the dumper (and UE4SS's own keybinds): the other sample mods are not needed.
    File.WriteAllText(Path.Combine(mods, "mods.txt"), "Keybinds : 1\nDRDump : 1\n");
    File.WriteAllText(manifest, new JObject { ["files"] = new JArray(created), ["release"] = asset["name"] }.ToString());
    Emit(new JObject { ["step"] = "done", ["release"] = asset["name"] });
  }

  static void Remove(string win64, string manifest) {
    if (!File.Exists(manifest)) throw new InvalidOperationException("the planner did not install UE4SS in this game folder");
    if (GameRunning()) throw new InvalidOperationException("the game is running: close it first");
    var files = (JObject.Parse(File.ReadAllText(manifest))["files"] as JArray ?? []).Select(f => f.ToString());
    foreach (var f in files) {
      if (f.Contains("..") || f.Contains('/') || f.Contains('\\') || f.Length == 0) continue;
      var p = Path.Combine(win64, f);
      if (Directory.Exists(p)) Directory.Delete(p, true);
      else if (File.Exists(p)) File.Delete(p);
    }
    File.Delete(manifest);
    Emit(new JObject { ["step"] = "done" });
  }
}
