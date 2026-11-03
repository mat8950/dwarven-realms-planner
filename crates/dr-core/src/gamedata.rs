//! Game data loaded from `data/extracted/`: blueprint classes, struct schemas,
//! data tables and localisation.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::{Context, Result};
use serde_json::Value as Json;

use crate::bytecode::{Function, Str, Ty, quoted, struct_name};
use crate::value::{StructVal, Value};

pub struct Class {
    pub name: Str,
    pub parent: Option<Str>,
    pub functions: HashMap<Str, Rc<Function>>,
    pub props: Vec<(Str, Ty)>,
    /// Class default object values (only the properties the CDO overrides).
    pub defaults: HashMap<Str, Value>,
    /// Component templates of this class (`AttributesComponent` -> overridden props).
    pub templates: HashMap<Str, HashMap<Str, Value>>,
}

pub struct GameData {
    pub root: PathBuf,
    pub classes: HashMap<Str, Class>,
    pub structs: HashMap<Str, Vec<(Str, Ty)>>,
    /// Data table name -> (row name, row value), in table order.
    tables: std::cell::RefCell<HashMap<Str, Rc<Vec<(Str, Value)>>>>,
    table_files: HashMap<Str, PathBuf>,
    /// Localised strings: (table id or namespace, key) -> text.
    pub strings: HashMap<(String, String), String>,
    /// Source (English) text of the string tables, to catch stale translations.
    pub source_strings: HashMap<(String, String), String>,
    /// Enum name -> (value, display name), in value order.
    pub enums: HashMap<Str, Vec<(u8, String)>>,
    /// Locres folder of the loaded texts (`fr`, `en`, `de-DE`...).
    pub lang: String,
}

/// Languages the extracted game data has texts for (locres folder names).
pub fn languages(root: &Path) -> Vec<String> {
    let dir = root.join("data/extracted/tables/Localization/Game");
    let mut out: Vec<String> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().join("Game.locres.json").exists())
        .filter_map(|e| e.file_name().to_str().map(str::to_string))
        .collect();
    out.sort();
    out
}

/// `E_X::NewEnumeratorN` -> enum value. User-defined enumerator names are not their values.
static ENUM_VALUES: std::sync::OnceLock<HashMap<String, u8>> = std::sync::OnceLock::new();

/// Value of an enumerator label (`E_EquipmentSlot::NewEnumerator11` -> 5).
pub fn enum_value(label: &str) -> Option<u8> {
    if let Some(v) = ENUM_VALUES.get().and_then(|m| m.get(label)) {
        return Some(*v);
    }
    label.rsplit_once("NewEnumerator").and_then(|(_, n)| n.parse().ok())
}

impl GameData {
    /// Loads everything under `<root>/data/extracted`. `lang` is a locres folder name (`en`, `fr`).
    pub fn load(root: &Path, lang: &str) -> Result<GameData> {
        let ex = root.join("data/extracted");
        let mut gd = GameData {
            root: root.to_path_buf(),
            classes: HashMap::new(),
            structs: HashMap::new(),
            tables: Default::default(),
            table_files: HashMap::new(),
            strings: HashMap::new(),
            source_strings: HashMap::new(),
            enums: HashMap::new(),
            lang: lang.to_string(),
        };
        let mut values = HashMap::new();
        for f in walk(&ex.join("tables"))? {
            if f.file_stem().and_then(|s| s.to_str()).is_some_and(|s| s.starts_with("E_")) {
                gd.load_enum(&f, &mut values)?;
            }
        }
        let _ = ENUM_VALUES.set(values);
        for f in walk(&ex.join("blueprints"))? {
            gd.load_class(&f).with_context(|| format!("loading {}", f.display()))?;
        }
        for f in walk(&ex.join("tables"))? {
            let stem = f.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
            if stem.starts_with("STR_") {
                gd.load_struct(&f)?;
            } else if stem.starts_with("DT_") {
                gd.table_files.insert(stem.into(), f);
            } else if stem.starts_with("ST_") {
                gd.load_string_table(&f)?;
            }
        }
        let locres = ex.join(format!("tables/Localization/Game/{lang}/Game.locres.json"));
        if locres.exists() {
            gd.load_locres(&locres)?;
        }
        gd.add_engine_structs();
        Ok(gd)
    }

    fn load_class(&mut self, path: &Path) -> Result<()> {
        let d: Vec<Json> = serde_json::from_str(&fs::read_to_string(path)?)?;
        let Some(cls) = d.iter().find(|e| e["Type"] == "BlueprintGeneratedClass") else { return Ok(()) };
        let name: Str = cls["Name"].as_str().unwrap_or("?").into();
        let parent = cls["SuperStruct"]["ObjectName"].as_str().map(|s| quoted(s).into());
        let props = cls["ChildProperties"].as_array().into_iter().flatten().map(|p| (p["Name"].as_str().unwrap_or("?").into(), Ty::from_property(p))).collect();
        let mut functions = HashMap::new();
        for e in d.iter().filter(|e| e["Type"] == "Function") {
            let f = Function::parse(e)?;
            functions.insert(f.name.clone(), Rc::new(f));
        }
        let mut defaults = HashMap::new();
        if let Some(cdo) = d.iter().find(|e| e["Name"].as_str().is_some_and(|n| n.starts_with("Default__"))) {
            for (k, v) in cdo["Properties"].as_object().into_iter().flatten() {
                defaults.insert(k.as_str().into(), json_to_value(v));
            }
        }
        let mut templates = HashMap::new();
        for e in &d {
            let Some(n) = e["Name"].as_str().and_then(|n| n.strip_suffix("_GEN_VARIABLE")) else { continue };
            let props: HashMap<Str, Value> = e["Properties"].as_object().into_iter().flatten().map(|(k, v)| (k.as_str().into(), json_to_value(v))).collect();
            templates.insert(n.into(), props);
        }
        self.classes.insert(name.clone(), Class { name, parent, functions, props, defaults, templates });
        Ok(())
    }

    fn load_enum(&mut self, path: &Path, values: &mut HashMap<String, u8>) -> Result<()> {
        let d: Vec<Json> = serde_json::from_str(&fs::read_to_string(path)?)?;
        for e in d.iter().filter(|e| e["Type"] == "UserDefinedEnum") {
            let display: HashMap<&str, &str> = e["Properties"]["DisplayNameMap"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|x| Some((x["Key"].as_str()?, x["Value"]["LocalizedString"].as_str().or(x["Value"]["SourceString"].as_str())?)))
                .collect();
            let mut list = Vec::new();
            for (label, v) in e["Names"].as_object().into_iter().flatten() {
                let Some(v) = v.as_u64() else { continue };
                values.insert(label.clone(), v as u8);
                let short = label.rsplit("::").next().unwrap_or(label);
                if !short.ends_with("_MAX") {
                    list.push((v as u8, display.get(short).copied().unwrap_or(short).to_string()));
                }
            }
            list.sort();
            self.enums.insert(e["Name"].as_str().unwrap_or("?").into(), list);
        }
        Ok(())
    }

    fn load_struct(&mut self, path: &Path) -> Result<()> {
        let d: Vec<Json> = serde_json::from_str(&fs::read_to_string(path)?)?;
        for e in d.iter().filter(|e| e["Type"] == "UserDefinedStruct") {
            let fields = e["ChildProperties"].as_array().into_iter().flatten().map(|p| (p["Name"].as_str().unwrap_or("?").into(), Ty::from_property(p))).collect();
            self.structs.insert(e["Name"].as_str().unwrap_or("?").into(), fields);
        }
        Ok(())
    }

    fn load_string_table(&mut self, path: &Path) -> Result<()> {
        let d: Vec<Json> = serde_json::from_str(&fs::read_to_string(path)?)?;
        for e in &d {
            let st = &e["StringTable"];
            let id = st["TableNamespace"].as_str().unwrap_or("").to_string();
            for (k, v) in st["KeysToEntries"].as_object().into_iter().flatten() {
                if let Some(s) = v.as_str() {
                    self.strings.entry((id.clone(), k.clone())).or_insert_with(|| s.to_string());
                    self.source_strings.insert((id.clone(), k.clone()), s.to_string());
                }
            }
        }
        Ok(())
    }

    fn load_locres(&mut self, path: &Path) -> Result<()> {
        let d: Json = serde_json::from_str(&fs::read_to_string(path)?)?;
        // CUE4Parse serialises FTextLocalizationResource as { namespace: { key: text } }.
        for (ns, entries) in d.as_object().into_iter().flatten() {
            for (k, v) in entries.as_object().into_iter().flatten() {
                let text = v.as_str().map(str::to_string).or_else(|| v["LocalizedString"].as_str().map(str::to_string));
                if let Some(t) = text {
                    self.strings.insert((ns.clone(), k.clone()), t);
                }
            }
        }
        Ok(())
    }

    fn add_engine_structs(&mut self) {
        let mut add = |n: &str, f: &[(&str, Ty)]| {
            self.structs.entry(n.into()).or_insert_with(|| f.iter().map(|(k, t)| ((*k).into(), t.clone())).collect());
        };
        add("GameplayTag", &[("TagName", Ty::Name)]);
        add("GameplayTagContainer", &[("GameplayTags", Ty::Array), ("ParentTags", Ty::Array)]);
        add("DataTableRowHandle", &[("DataTable", Ty::Object), ("RowName", Ty::Name)]);
        add("Vector", &[("X", Ty::Double), ("Y", Ty::Double), ("Z", Ty::Double)]);
        add("Vector2D", &[("X", Ty::Double), ("Y", Ty::Double)]);
        add("LinearColor", &[("R", Ty::Float), ("G", Ty::Float), ("B", Ty::Float), ("A", Ty::Float)]);
    }

    /// Zero value for a property type.
    pub fn default_of(&self, ty: &Ty) -> Value {
        match ty {
            Ty::Bool => Value::Bool(false),
            Ty::Byte => Value::Byte(0),
            Ty::Int | Ty::Int64 => Value::Int(0),
            Ty::Float | Ty::Double => Value::Float(0.0),
            Ty::Name => Value::Name("None".into()),
            Ty::Str | Ty::Text => Value::Str("".into()),
            Ty::Array | Ty::Set => Value::Array(Vec::new()),
            Ty::Map => Value::Map(Vec::new()),
            Ty::Struct(n) => self.default_struct(n),
            Ty::Object | Ty::Other => Value::None,
        }
    }

    pub fn default_struct(&self, name: &str) -> Value {
        let fields = self.structs.get(name).map(|f| f.iter().map(|(k, t)| (k.clone(), self.default_of(t))).collect()).unwrap_or_default();
        Value::Struct(Box::new(StructVal { ty: name.into(), fields }))
    }

    /// Rows of a data table, by asset name (`DT_Attributes`).
    pub fn table(&self, name: &str) -> Result<Rc<Vec<(Str, Value)>>> {
        if let Some(t) = self.tables.borrow().get(name) {
            return Ok(t.clone());
        }
        let path = self.table_files.get(name).with_context(|| format!("unknown data table {name}"))?;
        let d: Vec<Json> = serde_json::from_str(&fs::read_to_string(path)?)?;
        let rows: Vec<(Str, Value)> = d[0]["Rows"].as_object().into_iter().flatten().map(|(k, v)| (k.as_str().into(), json_to_value(v))).collect();
        let rows = Rc::new(rows);
        self.tables.borrow_mut().insert(name.into(), rows.clone());
        Ok(rows)
    }

    /// Function lookup walking up the class hierarchy.
    pub fn find_function(&self, class: &str, func: &str) -> Option<(Str, Rc<Function>)> {
        let mut c = self.classes.get(class);
        while let Some(cls) = c {
            if let Some(f) = cls.functions.get(func) {
                return Some((cls.name.clone(), f.clone()));
            }
            c = cls.parent.as_deref().and_then(|p| self.classes.get(p));
        }
        None
    }

    /// Initial property values of a new object of `class` (type defaults overridden by the CDO).
    pub fn new_props(&self, class: &str) -> HashMap<Str, Value> {
        let mut chain = Vec::new();
        let mut c = self.classes.get(class);
        while let Some(cls) = c {
            chain.push(cls);
            c = cls.parent.as_deref().and_then(|p| self.classes.get(p));
        }
        let mut props = HashMap::new();
        for cls in chain.iter().rev() {
            for (k, t) in &cls.props {
                props.insert(k.clone(), self.default_of(t));
            }
            for (k, v) in &cls.defaults {
                props.insert(k.clone(), v.clone());
            }
        }
        props
    }

    /// Property overrides of component `component` as placed in `class` (nearest class wins).
    pub fn component_template(&self, class: &str, component: &str) -> HashMap<Str, Value> {
        let mut chain = Vec::new();
        let mut c = self.classes.get(class);
        while let Some(cls) = c {
            chain.push(cls);
            c = cls.parent.as_deref().and_then(|p| self.classes.get(p));
        }
        let mut out = HashMap::new();
        for cls in chain.iter().rev() {
            if let Some(t) = cls.templates.get(component) {
                for (k, v) in t {
                    out.insert(k.clone(), v.clone());
                }
            }
        }
        out
    }

    /// Localised text, with its numbers taken from the source text when the translation
    /// lags behind a balance change (FR "Gagnez 50 Chance" vs EN "Gain 200 Luck", code: 200).
    pub fn text_checked(&self, table: &str, key: &str) -> Option<String> {
        let tr = self.text(table, key)?;
        let Some(src) = self.source_strings.get(&(table.to_string(), key.to_string())) else { return Some(tr.to_string()) };
        Some(fix_numbers(tr, src, &self.lang))
    }

    /// Localised text for a string-table / locres key.
    pub fn text(&self, table: &str, key: &str) -> Option<&str> {
        self.strings.get(&(table.to_string(), key.to_string())).map(String::as_str)
    }
}

/// Converts exported property JSON to a VM value. Struct types are inferred from shape.
pub fn json_to_value(v: &Json) -> Value {
    match v {
        Json::Null => Value::None,
        Json::Bool(b) => Value::Bool(*b),
        Json::Number(n) => {
            if n.is_f64() {
                Value::Float(n.as_f64().unwrap_or(0.0))
            } else {
                Value::Int(n.as_i64().unwrap_or(0))
            }
        }
        Json::String(s) => {
            if s.contains("::") {
                if let Some(n) = enum_value(s) {
                    return Value::Byte(n);
                }
            }
            Value::Name(s.as_str().into())
        }
        Json::Array(a) => Value::Array(a.iter().map(json_to_value).collect()),
        Json::Object(o) => {
            if o.len() == 1 {
                if let Some(t) = o.get("TagName") {
                    return Value::tag(t.as_str().unwrap_or("None"));
                }
            }
            if o.contains_key("ObjectName") && o.contains_key("ObjectPath") {
                return Value::Asset(quoted(o["ObjectName"].as_str().unwrap_or("")).into());
            }
            if o.contains_key("TableId") || o.contains_key("SourceString") || o.contains_key("LocalizedString") {
                let s = o.get("LocalizedString").and_then(|x| x.as_str()).filter(|s| !s.is_empty()).or(o.get("SourceString").and_then(|x| x.as_str())).or(o.get("Key").and_then(|x| x.as_str())).unwrap_or("");
                return Value::Str(s.into());
            }
            let ty: &str = if o.contains_key("RowName") && o.contains_key("DataTable") { "DataTableRowHandle" } else { "" };
            let fields = o.iter().map(|(k, v)| (Str::from(k.as_str()), json_to_value(v))).collect();
            Value::Struct(Box::new(StructVal { ty: ty.into(), fields }))
        }
    }
}

pub fn struct_name_of(j: &Json) -> String {
    struct_name(j)
}

fn walk(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d)? {
            let p = e?.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "json") {
                out.push(p);
            }
        }
    }
    out.sort();
    Ok(out)
}

/// Numbers of a text, as (byte range, value, followed by `%`); decimal comma or point.
fn numbers(s: &str) -> Vec<(std::ops::Range<usize>, f64, bool)> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i].is_ascii_digit() {
            let start = i;
            while i < b.len() && (b[i].is_ascii_digit() || ((b[i] == b'.' || b[i] == b',') && i + 1 < b.len() && b[i + 1].is_ascii_digit())) {
                i += 1;
            }
            if let Ok(v) = s[start..i].replace(',', ".").parse() {
                let pct = s[i..].trim_start_matches([' ', '\u{a0}', '\u{202f}']).starts_with('%');
                out.push((start..i, v, pct));
            }
        } else {
            i += 1;
        }
    }
    out
}

/// Fixes stale numbers of a translation against its source text. Numbers found in both
/// (in any order: languages order phrases differently) are kept. A stale number is replaced
/// by the source's new number of the same kind (percent or plain) when that pairing is
/// unambiguous; otherwise the translation is kept and the source text is appended.
pub fn fix_numbers(translated: &str, source: &str, lang: &str) -> String {
    let (t, s) = (numbers(translated), numbers(source));
    let same = |a: f64, b: f64| (a - b).abs() < 1e-9;
    let mut pool: Vec<(f64, bool)> = s.iter().map(|x| (x.1, x.2)).collect();
    let mut stale = Vec::new();
    for (range, v, pct) in &t {
        match pool.iter().position(|p| same(p.0, *v)) {
            Some(i) => {
                pool.remove(i);
            }
            None => stale.push((range.clone(), *pct)),
        }
    }
    if stale.is_empty() {
        return translated.to_string();
    }
    let mut fixes = Vec::new();
    for kind in [false, true] {
        let old: Vec<_> = stale.iter().filter(|x| x.1 == kind).collect();
        let new: Vec<_> = pool.iter().filter(|x| x.1 == kind).collect();
        // Two stale numbers of one kind could be swapped by the phrase order.
        if old.len() != new.len() || old.len() > 1 {
            let label = if lang == "fr" { "à jour" } else { "EN" };
            return format!("{translated} [{label} : {source}]");
        }
        if let (Some(o), Some(n)) = (old.first(), new.first()) {
            fixes.push((o.0.clone(), n.0));
        }
    }
    fixes.sort_by_key(|f| std::cmp::Reverse(f.0.start));
    let mut out = translated.to_string();
    for (range, v) in fixes {
        let text = if v.fract() == 0.0 { format!("{v}") } else { format!("{v}").replace('.', ",") };
        out.replace_range(range, &text);
    }
    out
}
