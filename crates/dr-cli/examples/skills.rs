//! Lists the skills stored in a character save (table, row, level).
use dr_core::value::Value;

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("save path");
    let sav = dr_core::save::load(path.as_ref())?;
    let Some(Value::Struct(p)) = sav.get("HostPlayerData").cloned() else { anyhow::bail!("no HostPlayerData") };
    for (k, v) in &p.fields {
        let kind = match v {
            Value::Array(a) => format!("array[{}]", a.len()),
            Value::Struct(s) => format!("struct {}", s.ty),
            other => format!("{other:?}").chars().take(60).collect(),
        };
        println!("{k}: {kind}");
    }
    if std::env::args().nth(2).as_deref() == Some("raw") {
        if let Some((_, Value::Array(skills))) = p.fields.iter().find(|(k, _)| k.starts_with("CharacterSkills")) {
            println!("{:?}", skills.first());
        }
    }
    let mut counts = std::collections::BTreeMap::<String, (usize, i64)>::new();
    if let Some((_, Value::Array(skills))) = p.fields.iter().find(|(k, _)| k.starts_with("CharacterSkills")) {
        for s in skills {
            let Value::Struct(s) = s else { continue };
            let f = |n: &str| s.fields.iter().find(|(k, _)| k.starts_with(n)).map(|(_, v)| v.clone()).unwrap_or_default();
            let (table, row) = match f("Handle") {
                Value::Struct(h) => (h.get("DataTable").map(|v| v.name().to_string()).unwrap_or_default(), h.get("RowName").map(|v| v.name().to_string()).unwrap_or_default()),
                _ => Default::default(),
            };
            let e = counts.entry(table.clone()).or_default();
            e.0 += 1;
            e.1 += f("Level").i64();
            if std::env::args().nth(2).is_some_and(|t| table.contains(&t)) {
                println!("  {table} {row} lvl {}", f("Level").i64());
            }
        }
    }
    for (t, (n, l)) in counts {
        println!("{t}: {n} skills, {l} levels");
    }
    Ok(())
}
