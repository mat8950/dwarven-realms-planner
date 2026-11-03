//! Interpreter for Blueprint bytecode. It runs the game's own stat code so the
//! planner stays exact across patches: re-extract, no re-coding.

use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;

use anyhow::{Result, anyhow, bail};

use crate::bytecode::{CallKind, Expr, Function, Stmt, Str};
use crate::gamedata::GameData;
use crate::natives;
use crate::value::{ObjId, StructVal, Value};

pub struct Object {
    pub class: Str,
    pub props: HashMap<Str, Value>,
}

/// Host implementation of a function the extracted blueprints do not contain.
pub type Stub = Rc<dyn Fn(&mut Vm, ObjId, &mut [Value]) -> Result<Value>>;

pub struct Vm<'g> {
    pub game: &'g GameData,
    pub objects: Vec<Object>,
    /// Overrides by function name, checked before blueprint code.
    pub stubs: HashMap<String, Stub>,
    pub controller: Value,
    /// Unknown properties / functions hit while running (for diagnostics).
    pub missing: BTreeSet<String>,
    singletons: HashMap<Str, ObjId>,
    persistent: HashMap<ObjId, HashMap<Str, Value>>,
    steps: u64,
    /// Most steps any top-level call used since it was last reset (debug info).
    pub peak_steps: u64,
    depth: u32,
}

pub struct Frame {
    pub locals: HashMap<Str, Value>,
}

enum Place {
    Local(Str),
    Inst(ObjId, Str),
    Member(Box<Place>, Str),
    Index(Box<Place>, usize),
}

/// Interpreted statements allowed per top-level call (guards against runaway loops).
pub const MAX_STEPS: u64 = 20_000_000;

impl<'g> Vm<'g> {
    pub fn new(game: &'g GameData) -> Vm<'g> {
        Vm {
            game,
            objects: Vec::new(),
            stubs: HashMap::new(),
            controller: Value::None,
            missing: BTreeSet::new(),
            singletons: HashMap::new(),
            persistent: HashMap::new(),
            steps: 0,
            peak_steps: 0,
            depth: 0,
        }
    }

    /// Creates an object of `class` with its default property values.
    pub fn spawn(&mut self, class: &str) -> ObjId {
        let props = self.game.new_props(class);
        self.objects.push(Object { class: class.into(), props });
        self.objects.len() - 1
    }

    pub fn set(&mut self, obj: ObjId, prop: &str, v: Value) {
        self.objects[obj].props.insert(prop.into(), v);
    }

    pub fn get(&self, obj: ObjId, prop: &str) -> Value {
        self.objects[obj].props.get(prop).cloned().unwrap_or_default()
    }

    pub fn stub(&mut self, name: &str, f: impl Fn(&mut Vm, ObjId, &mut [Value]) -> Result<Value> + 'static) {
        self.stubs.insert(name.to_string(), Rc::new(f));
    }

    fn singleton(&mut self, class: &str) -> ObjId {
        if let Some(o) = self.singletons.get(class) {
            return *o;
        }
        let o = self.spawn(class);
        self.singletons.insert(class.into(), o);
        o
    }

    /// Calls a blueprint function by name on `obj`. `args` holds the non-return
    /// parameters in order; out parameters are written back into it.
    pub fn invoke(&mut self, obj: ObjId, func: &str, args: &mut Vec<Value>) -> Result<Value> {
        let class = self.objects[obj].class.clone();
        let (_, f) = self.game.find_function(&class, func).ok_or_else(|| anyhow!("{class}::{func} not found"))?;
        // The step limit guards one top-level call, not the VM's whole lifetime.
        let top = self.depth == 0;
        if top {
            self.steps = 0;
        }
        let r = self.run(obj, &f, args);
        if top {
            self.peak_steps = self.peak_steps.max(self.steps);
        }
        r.map_err(|e| e.context(format!("{class}::{func} after {} steps", self.steps)))
    }

    fn run(&mut self, this: ObjId, f: &Rc<Function>, args: &mut [Value]) -> Result<Value> {
        if self.depth >= 200 {
            bail!("call depth exceeded in {}", f.name);
        }
        self.depth += 1;
        let mut fr = Frame { locals: HashMap::with_capacity(f.locals.len()) };
        for (n, t) in &f.locals {
            fr.locals.insert(n.clone(), self.game.default_of(t));
        }
        let uber = f.name.starts_with("ExecuteUbergraph");
        if uber {
            if let Some(p) = self.persistent.get(&this) {
                for (k, v) in p {
                    fr.locals.insert(k.clone(), v.clone());
                }
            }
        }
        let params: Vec<_> = f.params.iter().filter(|p| !p.ret).collect();
        for (p, a) in params.iter().zip(args.iter()) {
            fr.locals.insert(p.name.clone(), a.clone());
        }
        let r = self.exec(this, f, &mut fr);
        self.depth -= 1;
        r.map_err(|e| e.context(format!("in {}", f.name)))?;
        if uber {
            self.persistent.insert(this, fr.locals.clone());
        }
        for (i, p) in params.iter().enumerate() {
            if p.out && i < args.len() {
                args[i] = fr.locals.get(&p.name).cloned().unwrap_or_default();
            }
        }
        Ok(f.params.iter().find(|p| p.ret).and_then(|p| fr.locals.get(&p.name).cloned()).unwrap_or_default())
    }

    fn exec(&mut self, this: ObjId, f: &Function, fr: &mut Frame) -> Result<()> {
        let mut pc = 0usize;
        let mut flow: Vec<u32> = Vec::new();
        let jump = |off: u32| f.offsets.get(&off).copied().ok_or_else(|| anyhow!("bad jump target {off}"));
        while pc < f.stmts.len() {
            self.steps += 1;
            if self.steps > MAX_STEPS {
                bail!("step limit exceeded");
            }
            let (off, st) = &f.stmts[pc];
            match st {
                Stmt::Expr(e) => {
                    self.eval(fr, this, e).map_err(|err| err.context(format!("@{off}")))?;
                    pc += 1;
                }
                Stmt::Jump(t) => pc = jump(*t)?,
                Stmt::JumpIfNot(c, t) => {
                    if self.eval(fr, this, c)?.bool() {
                        pc += 1
                    } else {
                        pc = jump(*t)?
                    }
                }
                Stmt::PushFlow(a) => {
                    flow.push(*a);
                    pc += 1;
                }
                Stmt::PopFlow => match flow.pop() {
                    Some(a) => pc = jump(a)?,
                    None => break,
                },
                Stmt::PopFlowIfNot(c) => {
                    if self.eval(fr, this, c)?.bool() {
                        pc += 1;
                    } else {
                        match flow.pop() {
                            Some(a) => pc = jump(a)?,
                            None => break,
                        }
                    }
                }
                Stmt::Return(e) => {
                    self.eval(fr, this, e)?;
                    break;
                }
                Stmt::End => break,
                Stmt::ComputedJump(e) => pc = jump(self.eval(fr, this, e)?.i64() as u32)?,
                Stmt::LetPersistent(name, e) => {
                    let v = self.eval(fr, this, e)?;
                    self.persistent.entry(this).or_default().insert(name.clone(), v);
                    pc += 1;
                }
            }
        }
        Ok(())
    }

    pub fn eval(&mut self, fr: &mut Frame, this: ObjId, e: &Expr) -> Result<Value> {
        Ok(match e {
            Expr::Local(n) => fr.locals.get(n).cloned().unwrap_or_default(),
            Expr::Instance(n) => self.read_prop(this, n),
            Expr::Int(i) => Value::Int(*i),
            Expr::Float(x) => Value::Float(*x),
            Expr::Byte(b) => Value::Byte(*b),
            Expr::Bool(b) => Value::Bool(*b),
            Expr::Name(s) => Value::Name(s.clone()),
            Expr::Str(s) | Expr::Text(s) => Value::Str(s.clone()),
            Expr::Nothing | Expr::NoObject => Value::None,
            Expr::SelfRef => Value::Object(this),
            Expr::Object(n) => Value::Asset(n.clone()),
            Expr::StructConst { ty, fields } => {
                let names: Vec<Str> = self.game.structs.get(ty).map(|f| f.iter().map(|(k, _)| k.clone()).collect()).unwrap_or_default();
                let mut vals = Vec::with_capacity(fields.len());
                for (i, fe) in fields.iter().enumerate() {
                    let k = names.get(i).cloned().unwrap_or_else(|| format!("_{i}").into());
                    vals.push((k, self.eval(fr, this, fe)?));
                }
                Value::Struct(Box::new(StructVal { ty: ty.clone(), fields: vals }))
            }
            Expr::ArrayConst(items) => {
                let mut v = Vec::with_capacity(items.len());
                for it in items {
                    v.push(self.eval(fr, this, it)?);
                }
                Value::Array(v)
            }
            Expr::SetArray { target, items } => {
                let mut v = Vec::with_capacity(items.len());
                for it in items {
                    v.push(self.eval(fr, this, it)?);
                }
                if let Some(p) = self.place(fr, this, target)? {
                    self.write(fr, &p, Value::Array(v));
                }
                Value::None
            }
            Expr::MapConst(items) => {
                let mut m = Vec::with_capacity(items.len());
                for (k, v) in items {
                    m.push((self.eval(fr, this, k)?, self.eval(fr, this, v)?));
                }
                Value::Map(m)
            }
            Expr::SetMap { target, items } => {
                let mut m = Vec::with_capacity(items.len());
                for (k, v) in items {
                    m.push((self.eval(fr, this, k)?, self.eval(fr, this, v)?));
                }
                if let Some(p) = self.place(fr, this, target)? {
                    self.write(fr, &p, Value::Map(m));
                }
                Value::None
            }
            Expr::Let { target, value } => {
                let v = self.eval(fr, this, value)?;
                match self.place(fr, this, target)? {
                    Some(p) => self.write(fr, &p, v),
                    // Writing through a null object (camera, movement...): the game skips it too.
                    None if matches!(**target, Expr::Context { .. }) => {
                        self.missing.insert(format!("null context for assignment {}", describe(target)));
                    }
                    None => bail!("assignment to non-lvalue {target:?}"),
                }
                Value::None
            }
            Expr::Context { obj, inner } => match self.eval(fr, this, obj)? {
                Value::Object(o) => self.eval_on(fr, this, o, inner)?,
                Value::Asset(a) => {
                    // Library default object (`Default__BP_GameLibrary_C`, `Default__KismetMathLibrary`).
                    let cls = a.strip_prefix("Default__").unwrap_or(&a).to_string();
                    let o = if self.game.classes.contains_key(cls.as_str()) { self.singleton(&cls) } else { this };
                    self.eval_on(fr, this, o, inner)?
                }
                _ => {
                    self.missing.insert(format!("null context for {}", describe(inner)));
                    Value::None
                }
            },
            Expr::Member { base, field } => match self.eval(fr, this, base)? {
                Value::Struct(s) => s.get(field).cloned().unwrap_or_default(),
                _ => Value::None,
            },
            Expr::Cast { conv, target } => {
                let v = self.eval(fr, this, target)?;
                match &**conv {
                    "CST_DoubleToFloat" => Value::Float(v.f64() as f32 as f64),
                    "CST_ObjectToBool" | "CST_InterfaceToBool" | "CST_ObjectToBool2" | "CST_InterfaceToBool2" => Value::Bool(matches!(v, Value::Object(_) | Value::Asset(_))),
                    _ => v,
                }
            }
            Expr::DynCast { class, target } => {
                let v = self.eval(fr, this, target)?;
                match v {
                    Value::Object(o) if self.is_a(o, class) => v,
                    Value::Object(_) => Value::None,
                    other => other,
                }
            }
            Expr::Switch { index, cases, default } => {
                let i = self.eval(fr, this, index)?;
                for (c, t) in cases {
                    if self.eval(fr, this, c)?.key_eq(&i) {
                        return self.eval(fr, this, t);
                    }
                }
                self.eval(fr, this, default)?
            }
            Expr::ArrayGetByRef { array, index } => {
                let a = self.eval(fr, this, array)?;
                let i = self.eval(fr, this, index)?.i64();
                match a {
                    Value::Array(v) => v.get(i.max(0) as usize).cloned().unwrap_or_default(),
                    _ => Value::None,
                }
            }
            Expr::Call { kind, owner, func, args } => self.call(fr, this, this, kind, owner.as_deref(), func, args)?,
            Expr::Unsupported(t) => bail!("unsupported bytecode {t}"),
        })
    }

    /// Evaluates `inner` on the context object `target`. Like the game's
    /// `ProcessContextOpcode`, call arguments still read the caller's variables.
    fn eval_on(&mut self, fr: &mut Frame, caller: ObjId, target: ObjId, inner: &Expr) -> Result<Value> {
        match inner {
            Expr::Call { kind, owner, func, args } => self.call(fr, caller, target, kind, owner.as_deref(), func, args),
            _ => self.eval(fr, target, inner),
        }
    }

    fn is_a(&self, obj: ObjId, class: &str) -> bool {
        let mut c = Some(self.objects[obj].class.clone());
        let mut known = false;
        while let Some(cls) = c {
            if &*cls == class {
                return true;
            }
            match self.game.classes.get(&*cls) {
                Some(k) => {
                    known = true;
                    c = k.parent.clone();
                }
                None => c = None,
            }
        }
        // Interfaces and classes we did not extract: trust the host.
        !known || !self.game.classes.contains_key(class)
    }

    fn read_prop(&mut self, obj: ObjId, name: &str) -> Value {
        match self.objects[obj].props.get(name) {
            Some(v) => v.clone(),
            None => {
                self.missing.insert(format!("{}.{}", self.objects[obj].class, name));
                Value::None
            }
        }
    }

    /// Calls `func` on `this`; `caller` owns the frame the arguments are read from.
    fn call(&mut self, fr: &mut Frame, caller: ObjId, this: ObjId, kind: &CallKind, owner: Option<&str>, func: &str, args: &[Expr]) -> Result<Value> {
        if let Some(s) = self.stubs.get(func).cloned() {
            let mut vals = self.eval_args(fr, caller, args)?;
            let r = s(self, this, &mut vals)?;
            self.write_back(fr, caller, args, &vals)?;
            return Ok(r);
        }

        let target = self.resolve(this, kind, owner, func);
        let Some((obj, f)) = target else {
            return self.call_native(fr, caller, this, owner, func, args);
        };
        let mut vals = self.eval_args(fr, caller, args)?;
        let r = self.run(obj, &f, &mut vals)?;
        // Only out params of the callee are written back.
        let params: Vec<_> = f.params.iter().filter(|p| !p.ret).collect();
        for (i, p) in params.iter().enumerate() {
            if p.out && i < args.len() {
                if let Some(pl) = self.place(fr, caller, &args[i])? {
                    self.write(fr, &pl, std::mem::take(&mut vals[i]));
                }
            }
        }
        Ok(r)
    }

    fn eval_args(&mut self, fr: &mut Frame, this: ObjId, args: &[Expr]) -> Result<Vec<Value>> {
        let mut vals = Vec::with_capacity(args.len());
        for a in args {
            vals.push(self.eval(fr, this, a)?);
        }
        Ok(vals)
    }

    /// Native call: arguments that are variables are moved out of their slot and back,
    /// so containers such as `AttributesNamed` are never copied. A variable used by
    /// several arguments (`FClamp(X, X, Max)`) is copied, and written back once.
    fn call_native(&mut self, fr: &mut Frame, caller: ObjId, this: ObjId, owner: Option<&str>, func: &str, args: &[Expr]) -> Result<Value> {
        let mut places = Vec::with_capacity(args.len());
        for a in args {
            places.push(self.place(fr, caller, a)?);
        }
        let shared = |i: usize| places.iter().enumerate().any(|(j, q)| j != i && matches!((&places[i], q), (Some(p), Some(q)) if same_root(p, q)));
        let shared: Vec<bool> = (0..args.len()).map(shared).collect();
        // Expressions first, so none of them reads a variable that was moved out.
        let mut vals: Vec<Value> = Vec::with_capacity(args.len());
        for (a, p) in args.iter().zip(&places) {
            vals.push(if p.is_none() { self.eval(fr, caller, a)? } else { Value::None });
        }
        for (i, p) in places.iter().enumerate() {
            if let Some(p) = p {
                let s = slot(&mut self.objects, &mut fr.locals, p);
                vals[i] = if shared[i] { s.clone() } else { std::mem::take(s) };
            }
        }
        let r = natives::call(self, this, owner.unwrap_or(""), func, &mut vals);
        for (i, (p, v)) in places.iter().zip(vals).enumerate() {
            let Some(p) = p else { continue };
            if shared[i] && places[..i].iter().flatten().any(|q| same_root(p, q)) {
                continue;
            }
            *slot(&mut self.objects, &mut fr.locals, p) = v;
        }
        match r? {
            Some(r) => Ok(r),
            None => {
                let cls = self.objects[this].class.clone();
                self.missing.insert(format!("call {}::{}", owner.unwrap_or(&cls), func));
                Ok(Value::None)
            }
        }
    }

    fn resolve(&mut self, this: ObjId, kind: &CallKind, owner: Option<&str>, func: &str) -> Option<(ObjId, Rc<Function>)> {
        match kind {
            CallKind::Virtual => {
                let cls = self.objects[this].class.clone();
                self.game.find_function(&cls, func).map(|(_, f)| (this, f))
            }
            CallKind::Final => match owner {
                Some(o) if self.game.classes.contains_key(o) => {
                    let obj = if self.is_a(this, o) && self.game.find_function(&self.objects[this].class, func).is_some() { this } else { self.singleton(o) };
                    self.game.find_function(o, func).map(|(_, f)| (obj, f))
                }
                _ => None,
            },
        }
    }

    fn write_back(&mut self, fr: &mut Frame, this: ObjId, args: &[Expr], vals: &[Value]) -> Result<()> {
        for (a, v) in args.iter().zip(vals) {
            if let Some(p) = self.place(fr, this, a)? {
                self.write(fr, &p, v.clone());
            }
        }
        Ok(())
    }

    fn place(&mut self, fr: &mut Frame, this: ObjId, e: &Expr) -> Result<Option<Place>> {
        Ok(match e {
            Expr::Local(n) => Some(Place::Local(n.clone())),
            Expr::Instance(n) => Some(Place::Inst(this, n.clone())),
            Expr::Member { base, field } => self.place(fr, this, base)?.map(|b| Place::Member(Box::new(b), field.clone())),
            Expr::Context { obj, inner } => match self.eval(fr, this, obj)? {
                Value::Object(o) => self.place(fr, o, inner)?,
                _ => None,
            },
            Expr::ArrayGetByRef { array, index } => {
                let i = self.eval(fr, this, index)?.i64().max(0) as usize;
                self.place(fr, this, array)?.map(|b| Place::Index(Box::new(b), i))
            }
            _ => None,
        })
    }

    fn write(&mut self, fr: &mut Frame, p: &Place, v: Value) {
        *slot(&mut self.objects, &mut fr.locals, p) = v;
    }
}

fn slot<'a>(objects: &'a mut [Object], locals: &'a mut HashMap<Str, Value>, p: &Place) -> &'a mut Value {
    match p {
        Place::Local(n) => locals.entry(n.clone()).or_default(),
        Place::Inst(o, n) => objects[*o].props.entry(n.clone()).or_default(),
        Place::Member(b, f) => {
            let v = slot(objects, locals, b);
            if !matches!(v, Value::Struct(_)) {
                *v = Value::Struct(Box::new(StructVal { ty: "".into(), fields: Vec::new() }));
            }
            match v {
                Value::Struct(s) => s.get_mut(f),
                _ => unreachable!(),
            }
        }
        Place::Index(b, i) => {
            let v = slot(objects, locals, b);
            if !matches!(v, Value::Array(_)) {
                *v = Value::Array(Vec::new());
            }
            match v {
                Value::Array(a) => {
                    if *i >= a.len() {
                        a.resize(*i + 1, Value::None);
                    }
                    &mut a[*i]
                }
                _ => unreachable!(),
            }
        }
    }
}

fn root(p: &Place) -> &Place {
    match p {
        Place::Member(b, _) | Place::Index(b, _) => root(b),
        _ => p,
    }
}

fn same_root(a: &Place, b: &Place) -> bool {
    match (root(a), root(b)) {
        (Place::Local(x), Place::Local(y)) => x == y,
        (Place::Inst(o, x), Place::Inst(q, y)) => o == q && x == y,
        _ => false,
    }
}

fn describe(e: &Expr) -> String {
    match e {
        Expr::Instance(n) => format!("property {n}"),
        Expr::Call { func, .. } => format!("call {func}"),
        _ => "expression".to_string(),
    }
}
