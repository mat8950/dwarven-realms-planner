using CUE4Parse.FileProvider;
using CUE4Parse.UE4.Versions;
using CUE4Parse.MappingsProvider.Usmap;
using CUE4Parse.UE4.Localization;
using CUE4Parse.UE4.Readers;
using CUE4Parse.UE4.Assets.Exports.Texture;
using CUE4Parse.UE4.Assets.Exports.Engine.Font;
using CUE4Parse_Conversion.Options;
using CUE4Parse_Conversion.Textures;
using Newtonsoft.Json;

// usage: app <root> <outDir> <filters> [json|assets]
// <filters>: comma-separated substrings of package paths, or @file with one per line.
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
