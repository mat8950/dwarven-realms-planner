//! Engine functions (KismetMathLibrary, array/map libraries, gameplay tags, data
//! tables) used by the stat blueprints. Returns `None` for unknown functions.

use anyhow::Result;

use crate::value::{ObjId, Value};
use crate::vm::Vm;

pub fn call(vm: &mut Vm, _this: ObjId, owner: &str, func: &str, a: &mut [Value]) -> Result<Option<Value>> {
    let lib = owner.rsplit('.').next().unwrap_or(owner);
    let r = match lib {
        "KismetMathLibrary" => math(func, a),
        "KismetArrayLibrary" => array(func, a),
        "BlueprintMapLibrary" => map(func, a),
        "BlueprintGameplayTagLibrary" => tags(func, a),
        "KismetStringLibrary" | "KismetTextLibrary" => text(func, a),
        "DataTableFunctionLibrary" => return data_table(vm, func, a),
        "KismetSystemLibrary" => match func {
            "IsValid" | "IsValidClass" => Some(Value::Bool(matches!(a.first(), Some(Value::Object(_) | Value::Asset(_))))),
            "IsServer" | "IsStandalone" => Some(Value::Bool(true)),
            "IsDedicatedServer" | "IsPackagedForDistribution" => Some(Value::Bool(func == "IsPackagedForDistribution")),
            "PrintString" | "PrintText" | "Delay" | "RetriggerableDelay" => Some(Value::None),
            "MakeLiteralInt" | "MakeLiteralFloat" | "MakeLiteralDouble" | "MakeLiteralBool" | "MakeLiteralName" | "MakeLiteralString" | "MakeLiteralByte" | "MakeLiteralText" => a.first().cloned(),
            _ => None,
        },
        "NetPushModelHelpers" => Some(Value::None),
        "GameplayStatics" => match func {
            "GetPlayerController" => Some(vm.controller.clone()),
            _ => None,
        },
        _ => None,
    };
    if r.is_some() {
        return Ok(r);
    }
    // Some calls come through as virtual (no owner): try every library by name.
    Ok(match func {
        "IsWithEditor" => Some(Value::Bool(false)),
        _ if owner.is_empty() => math(func, a).or_else(|| array(func, a)).or_else(|| map(func, a)).or_else(|| tags(func, a)),
        _ => None,
    })
}

fn f(a: &[Value], i: usize) -> f64 {
    a.get(i).map(Value::f64).unwrap_or(0.0)
}

fn n(a: &[Value], i: usize) -> i64 {
    a.get(i).map(Value::i64).unwrap_or(0)
}

fn b(a: &[Value], i: usize) -> bool {
    a.get(i).map(Value::bool).unwrap_or(false)
}

fn math(func: &str, a: &mut [Value]) -> Option<Value> {
    let fl = |x: f64| Some(Value::Float(x));
    let int = |x: i64| Some(Value::Int(x));
    let bo = |x: bool| Some(Value::Bool(x));
    match func {
        "Not_PreBool" => return bo(!b(a, 0)),
        "BooleanAND" => return bo(b(a, 0) && b(a, 1)),
        "BooleanOR" => return bo(b(a, 0) || b(a, 1)),
        "BooleanXOR" => return bo(b(a, 0) ^ b(a, 1)),
        "BooleanNAND" => return bo(!(b(a, 0) && b(a, 1))),
        "EqualEqual_BoolBool" => return bo(b(a, 0) == b(a, 1)),
        "NotEqual_BoolBool" => return bo(b(a, 0) != b(a, 1)),
        "FMax" => return fl(f(a, 0).max(f(a, 1))),
        "FMin" => return fl(f(a, 0).min(f(a, 1))),
        "Max" | "MaxInt64" => return int(n(a, 0).max(n(a, 1))),
        "Min" | "MinInt64" => return int(n(a, 0).min(n(a, 1))),
        "FClamp" => return fl(f(a, 0).max(f(a, 1)).min(f(a, 2))),
        "Clamp" | "ClampInt64" => return int(n(a, 0).max(n(a, 1)).min(n(a, 2))),
        "FTrunc" | "FTrunc64" => return int(f(a, 0).trunc() as i64),
        "Round" | "Round64" => return int(f(a, 0).round() as i64),
        "FFloor" | "FFloor64" => return int(f(a, 0).floor() as i64),
        "FCeil" | "FCeil64" => return int(f(a, 0).ceil() as i64),
        "Abs" => return fl(f(a, 0).abs()),
        "Abs_Int" | "Abs_Int64" => return int(n(a, 0).abs()),
        "Sqrt" => return fl(f(a, 0).sqrt()),
        "Loge" => return fl(f(a, 0).ln()),
        "Exp" => return fl(f(a, 0).exp()),
        "Log" => return fl(f(a, 0).ln() / f(a, 1).ln()),
        "MultiplyMultiply_FloatFloat" => return fl(f(a, 0).powf(f(a, 1))),
        "FMod" | "FMod64" => return fl(f(a, 0) % f(a, 1)),
        "SelectInt" => return if b(a, 2) { int(n(a, 0)) } else { int(n(a, 1)) },
        "SelectFloat" => return if b(a, 2) { fl(f(a, 0)) } else { fl(f(a, 1)) },
        "SelectName" | "SelectString" | "SelectObject" => return if b(a, 2) { a.first().cloned() } else { a.get(1).cloned() },
        "Lerp" => return fl(f(a, 0) + (f(a, 1) - f(a, 0)) * f(a, 2)),
        "MapRangeClamped" => {
            let t = ((f(a, 0) - f(a, 1)) / (f(a, 2) - f(a, 1))).clamp(0.0, 1.0);
            return fl(f(a, 3) + (f(a, 4) - f(a, 3)) * t);
        }
        "MakeLiteralDouble" | "MakeLiteralFloat" | "MakeLiteralInt" | "MakeLiteralByte" | "MakeLiteralBool" => return a.first().cloned(),
        _ => {}
    }
    if let Some(rest) = func.strip_prefix("Conv_") {
        let to = rest.split_once("To").map(|(_, t)| t).unwrap_or(rest);
        let v = a.first().cloned().unwrap_or_default();
        return Some(match to {
            "Double" | "Float" => Value::Float(v.f64()),
            "Int" | "Int64" | "Integer" => Value::Int(v.f64().trunc() as i64),
            "Byte" => Value::Byte(v.i64() as u8),
            "Bool" => Value::Bool(v.bool()),
            _ => v,
        });
    }
    // Binary operators: <Op>_<TypeA><TypeB>.
    let (op, types) = func.split_once('_')?;
    let float = types.contains("Double") || types.contains("Float");
    // Names, strings and object references compare by identity, not as numbers.
    let keyed = ["Name", "Str", "Object", "Class"].iter().any(|t| types.contains(t));
    let (x, y) = (f(a, 0), f(a, 1));
    let (i, j) = (n(a, 0), n(a, 1));
    Some(match op {
        "Add" => if float { Value::Float(x + y) } else { Value::Int(i.wrapping_add(j)) },
        "Subtract" => if float { Value::Float(x - y) } else { Value::Int(i.wrapping_sub(j)) },
        "Multiply" => if float { Value::Float(x * y) } else { Value::Int(i.wrapping_mul(j)) },
        "Divide" => {
            if float {
                Value::Float(if y == 0.0 { 0.0 } else { x / y })
            } else {
                Value::Int(if j == 0 { 0 } else { i / j })
            }
        }
        "Percent" => if float { Value::Float(if y == 0.0 { 0.0 } else { x % y }) } else { Value::Int(if j == 0 { 0 } else { i % j }) },
        "Greater" => Value::Bool(if float { x > y } else { i > j }),
        "GreaterEqual" => Value::Bool(if float { x >= y } else { i >= j }),
        "Less" => Value::Bool(if float { x < y } else { i < j }),
        "LessEqual" => Value::Bool(if float { x <= y } else { i <= j }),
        "EqualEqual" => Value::Bool(if keyed { a[0].key_eq(&a[1]) } else if float { x == y } else { i == j }),
        "NotEqual" => Value::Bool(if keyed { !a[0].key_eq(&a[1]) } else if float { x != y } else { i != j }),
        "NearlyEqual" => Value::Bool((x - y).abs() <= f(a, 2)),
        _ => return None,
    })
}

fn array(func: &str, a: &mut [Value]) -> Option<Value> {
    let item = a.get(1).cloned().unwrap_or_default();
    let idx = n(a, 1);
    let Some(Value::Array(arr)) = a.first_mut().map(|v| {
        if !matches!(v, Value::Array(_)) {
            *v = Value::Array(Vec::new());
        }
        v
    }) else {
        return None;
    };
    Some(match func {
        "Array_Length" => Value::Int(arr.len() as i64),
        "Array_LastIndex" => Value::Int(arr.len() as i64 - 1),
        "Array_IsValidIndex" => Value::Bool(idx >= 0 && (idx as usize) < arr.len()),
        "Array_Clear" => {
            arr.clear();
            Value::None
        }
        "Array_Add" => {
            arr.push(item);
            Value::Int(arr.len() as i64 - 1)
        }
        "Array_AddUnique" => match arr.iter().position(|x| x.key_eq(&item)) {
            Some(_) => Value::Int(-1),
            None => {
                arr.push(item);
                Value::Int(arr.len() as i64 - 1)
            }
        },
        "Array_Contains" => Value::Bool(arr.iter().any(|x| x.key_eq(&item))),
        "Array_Find" => Value::Int(arr.iter().position(|x| x.key_eq(&item)).map(|p| p as i64).unwrap_or(-1)),
        "Array_Remove" => {
            if idx >= 0 && (idx as usize) < arr.len() {
                arr.remove(idx as usize);
            }
            Value::None
        }
        "Array_RemoveItem" => {
            let before = arr.len();
            arr.retain(|x| !x.key_eq(&item));
            Value::Bool(arr.len() != before)
        }
        "Array_Append" => {
            if let Value::Array(src) = item {
                arr.extend(src);
            }
            Value::None
        }
        "Array_Resize" => {
            arr.resize(idx.max(0) as usize, Value::None);
            Value::None
        }
        "Array_Get" => {
            let v = arr.get(idx.max(0) as usize).cloned().unwrap_or_default();
            if let Some(out) = a.get_mut(2) {
                *out = v;
            }
            Value::None
        }
        "Array_Set" => {
            let v = a.get(2).cloned().unwrap_or_default();
            if let Some(Value::Array(arr)) = a.first_mut() {
                let i = idx.max(0) as usize;
                if i >= arr.len() {
                    arr.resize(i + 1, Value::None);
                }
                arr[i] = v;
            }
            Value::None
        }
        "Array_Insert" => {
            let at = n(a, 2).max(0) as usize;
            if let Some(Value::Array(arr)) = a.first_mut() {
                arr.insert(at.min(arr.len()), item);
            }
            Value::None
        }
        _ => return None,
    })
}

fn map(func: &str, a: &mut [Value]) -> Option<Value> {
    let key = a.get(1).cloned().unwrap_or_default();
    if let Some(v) = a.first_mut() {
        if !matches!(v, Value::Map(_)) {
            *v = Value::Map(Vec::new());
        }
    }
    let Some(Value::Map(m)) = a.first_mut() else { return None };
    let pos = m.iter().position(|(k, _)| k.key_eq(&key));
    Some(match func {
        "Map_Length" => Value::Int(m.len() as i64),
        "Map_Clear" => {
            m.clear();
            Value::None
        }
        "Map_Contains" => Value::Bool(pos.is_some()),
        "Map_Remove" => match pos {
            Some(p) => {
                m.remove(p);
                Value::Bool(true)
            }
            None => Value::Bool(false),
        },
        "Map_Add" => {
            let v = a.get(2).cloned().unwrap_or_default();
            let Some(Value::Map(m)) = a.first_mut() else { return None };
            match pos {
                Some(p) => m[p].1 = v,
                None => m.push((key, v)),
            }
            Value::None
        }
        "Map_Find" => {
            let found = pos.map(|p| m[p].1.clone());
            let ok = found.is_some();
            if let Some(out) = a.get_mut(2) {
                *out = found.unwrap_or_default();
            }
            Value::Bool(ok)
        }
        "Map_Values" => {
            let vals: Vec<Value> = m.iter().map(|(_, v)| v.clone()).collect();
            if let Some(out) = a.get_mut(1) {
                *out = Value::Array(vals);
            }
            Value::None
        }
        "Map_Keys" => {
            let keys: Vec<Value> = m.iter().map(|(k, _)| k.clone()).collect();
            if let Some(out) = a.get_mut(1) {
                *out = Value::Array(keys);
            }
            Value::None
        }
        _ => return None,
    })
}

fn tag_name(v: &Value) -> &str {
    v.name()
}

fn tags(func: &str, a: &mut [Value]) -> Option<Value> {
    let t0 = a.first().map(|v| tag_name(v).to_string()).unwrap_or_default();
    let t1 = a.get(1).map(|v| tag_name(v).to_string()).unwrap_or_default();
    let matches = |t: &str, parent: &str, exact: bool| t.eq_ignore_ascii_case(parent) || (!exact && t.len() > parent.len() && t[..parent.len()].eq_ignore_ascii_case(parent) && t.as_bytes()[parent.len()] == b'.');
    Some(match func {
        "EqualEqual_GameplayTag" => Value::Bool(t0.eq_ignore_ascii_case(&t1)),
        "NotEqual_GameplayTag" => Value::Bool(!t0.eq_ignore_ascii_case(&t1)),
        "GetTagName" => Value::Name(t0.as_str().into()),
        "MakeLiteralGameplayTag" => a.first().cloned().unwrap_or_default(),
        "IsGameplayTagValid" => Value::Bool(!t0.is_empty() && t0 != "None"),
        "MatchesTag" => Value::Bool(matches(&t0, &t1, b(a, 2))),
        "MakeGameplayTagContainerFromTag" => Value::Array(vec![a.first().cloned().unwrap_or_default()]),
        "HasTag" => {
            let exact = b(a, 2);
            let list = match a.first() {
                Some(Value::Struct(s)) => s.get("GameplayTags").cloned(),
                Some(v @ Value::Array(_)) => Some(v.clone()),
                _ => None,
            };
            let hit = matches!(list, Some(Value::Array(ref l)) if l.iter().any(|x| matches(x.name(), &t1, exact)));
            Value::Bool(hit)
        }
        _ => return None,
    })
}

fn text(func: &str, a: &mut [Value]) -> Option<Value> {
    let s = |v: &Value| match v {
        Value::Float(x) => format!("{x}"),
        other => other.name().to_string(),
    };
    Some(match func {
        f if f.starts_with("Conv_") => Value::Str(s(a.first()?).as_str().into()),
        "Concat_StrStr" => Value::Str(format!("{}{}", s(&a[0]), s(&a[1])).as_str().into()),
        _ => return None,
    })
}

fn data_table(vm: &mut Vm, func: &str, a: &mut [Value]) -> Result<Option<Value>> {
    let table = a.first().map(|v| v.name().to_string()).unwrap_or_default();
    Ok(match func {
        "GetDataTableRowNames" => {
            let rows = vm.game.table(&table)?;
            let names = rows.iter().map(|(k, _)| Value::Name(k.clone())).collect();
            if let Some(out) = a.get_mut(1) {
                *out = Value::Array(names);
            }
            Some(Value::None)
        }
        "GetDataTableRowFromName" => {
            let rows = vm.game.table(&table)?;
            let key = a.get(1).map(|v| v.name().to_string()).unwrap_or_default();
            let row = rows.iter().find(|(k, _)| k.eq_ignore_ascii_case(&key)).map(|(_, v)| v.clone());
            let ok = row.is_some();
            if let Some(out) = a.get_mut(2) {
                *out = row.unwrap_or_default();
            }
            Some(Value::Bool(ok))
        }
        "DoesDataTableRowExist" => {
            let rows = vm.game.table(&table)?;
            let key = a.get(1).map(|v| v.name().to_string()).unwrap_or_default();
            Some(Value::Bool(rows.iter().any(|(k, _)| k.eq_ignore_ascii_case(&key))))
        }
        _ => None,
    })
}
