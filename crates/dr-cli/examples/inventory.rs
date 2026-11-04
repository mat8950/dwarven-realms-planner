//! Dumps the item structure of a save: `inventory <save.sav> [n]` prints the fields of inventory item n;
//! with a world save (`_WRL.sav`) it lists the top-level properties.
use dr_core::value::Value;

fn show(v: &Value, depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth);
    match v {
        Value::Struct(s) => {
            out.push_str(&format!("struct {}\n", s.ty));
            for (k, f) in &s.fields {
                out.push_str(&format!("{pad}  {k}: "));
                show(f, depth + 1, out);
            }
        }
        Value::Array(a) => {
            out.push_str(&format!("array[{}]\n", a.len()));
            if let Some(first) = a.first() {
                out.push_str(&format!("{pad}  [0]: "));
                show(first, depth + 1, out);
            }
        }
        other => out.push_str(&format!("{}\n", format!("{other:?}").chars().take(100).collect::<String>())),
    }
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("save path");
    let n: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(0);
    let sav = dr_core::save::load(path.as_ref())?;
    let mut out = String::new();
    match sav.get("HostPlayerData").cloned() {
        Some(Value::Struct(p)) => {
            let inv = p.fields.iter().find(|(k, _)| k.starts_with("InventoryItems")).map(|(_, v)| v.clone());
            if let Some(Value::Array(items)) = inv {
                let row = |it: &Value| match it {
                    Value::Struct(s) => s.fields.iter().find(|(k, _)| k.starts_with("ItemHandle")).and_then(|(_, h)| match h {
                        Value::Struct(h) => h.get("RowName").map(|r| r.name().to_string()),
                        _ => None,
                    }),
                    _ => None,
                };
                let used: Vec<(usize, String)> = items.iter().enumerate().filter_map(|(i, it)| row(it).filter(|r| r != "None").map(|r| (i, r))).collect();
                out.push_str(&format!("used slots: {} {:?}\n", used.len(), used.iter().take(12).collect::<Vec<_>>()));
                let n = if n == 0 { used.first().map(|u| u.0).unwrap_or(0) } else { n };
                out.push_str(&format!("{} inventory items; item {n}: ", items.len()));
                if let Some(it) = items.get(n) {
                    show(it, 0, &mut out);
                }
            }
        }
        _ if std::env::args().nth(3).as_deref() == Some("containers") => {
            for (_, v) in &sav.fields {
                let Value::Array(levels) = v else { continue };
                for level in levels {
                    let Value::Struct(l) = level else { continue };
                    let Some((_, Value::Map(cs))) = l.fields.iter().find(|(k, _)| k.starts_with("ContainerComponents")) else { continue };
                    for (name, c) in cs {
                        let mut line = format!("{:?}: ", name);
                        if let Value::Struct(c) = c {
                            if let Some((_, Value::Array(items))) = c.fields.iter().find(|(k, _)| k.starts_with("Items")) {
                                let used: Vec<String> = items
                                    .iter()
                                    .filter_map(|it| match it {
                                        Value::Struct(s) => s.fields.iter().find(|(k, _)| k.starts_with("ItemHandle")).and_then(|(_, h)| match h {
                                            Value::Struct(h) => h.get("RowName").map(|r| r.name().to_string()),
                                            _ => None,
                                        }),
                                        _ => None,
                                    })
                                    .filter(|r| r != "None")
                                    .collect();
                                line.push_str(&format!("used {} {:?} ", used.len(), used.iter().take(6).collect::<Vec<_>>()));
                                if std::env::args().nth(4).is_some_and(|w| format!("{name:?}").contains(&w)) {
                                    if let Some(it) = items.iter().find(|it| matches!(it, Value::Struct(s) if s.fields.iter().any(|(k, v)| k.starts_with("isGenerated") && matches!(v, Value::Bool(true))))) {
                                        show(it, 0, &mut out);
                                    }
                                }
                            }
                            out.push_str(&line);
                            out.push('\n');
                            continue;
                        }
                        #[allow(unreachable_code)]
                        if let Value::Struct(c) = c {
                            for (k, f) in &c.fields {
                                let desc = match f {
                                    Value::Array(a) => format!("array[{}]", a.len()),
                                    Value::Struct(s) => format!("struct {}", s.ty),
                                    o => format!("{o:?}").chars().take(40).collect(),
                                };
                                line.push_str(&format!("{k}={desc}  "));
                            }
                        }
                        out.push_str(&line);
                        out.push('\n');
                    }
                }
            }
        }
        _ => {
            for (k, v) in &sav.fields {
                out.push_str(&format!("{k}: "));
                show(v, 0, &mut out);
            }
        }
    }
    print!("{out}");
    Ok(())
}
