//! Blueprint (Kismet) bytecode, parsed from the CUE4Parse JSON export.

use std::collections::HashMap;
use std::rc::Rc;

use anyhow::{Context, Result, anyhow, bail};
use serde_json::Value as Json;

pub type Str = Rc<str>;

/// Static type of a property, used to zero-initialise locals and struct fields.
#[derive(Debug, Clone)]
pub enum Ty {
    Bool,
    Byte,
    Int,
    Int64,
    Float,
    Double,
    Name,
    Str,
    Text,
    Struct(Str),
    Array,
    Map,
    Set,
    Object,
    Other,
}

impl Ty {
    pub fn from_property(p: &Json) -> Ty {
        match p["Type"].as_str().unwrap_or("") {
            "BoolProperty" => Ty::Bool,
            "ByteProperty" | "EnumProperty" => Ty::Byte,
            "IntProperty" | "UInt32Property" | "Int16Property" | "Int8Property" | "UInt16Property" => Ty::Int,
            "Int64Property" | "UInt64Property" => Ty::Int64,
            "FloatProperty" => Ty::Float,
            "DoubleProperty" => Ty::Double,
            "NameProperty" => Ty::Name,
            "StrProperty" => Ty::Str,
            "TextProperty" => Ty::Text,
            "StructProperty" => Ty::Struct(struct_name(&p["Struct"]).into()),
            "ArrayProperty" => Ty::Array,
            "MapProperty" => Ty::Map,
            "SetProperty" => Ty::Set,
            "ObjectProperty" | "ClassProperty" | "InterfaceProperty" | "SoftObjectProperty" | "WeakObjectProperty" => Ty::Object,
            _ => Ty::Other,
        }
    }
}

/// `"UserDefinedStruct'STR_Attribute'"` -> `STR_Attribute`.
pub fn struct_name(j: &Json) -> String {
    let s = j["ObjectName"].as_str().or_else(|| j.as_str()).unwrap_or("");
    quoted(s).to_string()
}

/// Text between the first pair of single quotes, or the whole string.
pub fn quoted(s: &str) -> &str {
    match (s.find('\''), s.rfind('\'')) {
        (Some(a), Some(b)) if b > a => &s[a + 1..b],
        _ => s,
    }
}

#[derive(Debug, Clone)]
pub enum CallKind {
    /// Resolved by name on the context object (virtual / local virtual).
    Virtual,
    /// Fully qualified `Owner:Function` (final, math, local final).
    Final,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Local(Str),
    Instance(Str),
    Int(i64),
    Float(f64),
    Byte(u8),
    Bool(bool),
    Name(Str),
    Str(Str),
    Text(Str),
    Nothing,
    SelfRef,
    NoObject,
    Object(Str),
    StructConst { ty: Str, fields: Vec<Expr> },
    ArrayConst(Vec<Expr>),
    SetArray { target: Box<Expr>, items: Vec<Expr> },
    MapConst(Vec<(Expr, Expr)>),
    SetMap { target: Box<Expr>, items: Vec<(Expr, Expr)> },
    Call { kind: CallKind, owner: Option<Str>, func: Str, args: Vec<Expr> },
    Context { obj: Box<Expr>, inner: Box<Expr> },
    Member { base: Box<Expr>, field: Str },
    Let { target: Box<Expr>, value: Box<Expr> },
    Cast { conv: Str, target: Box<Expr> },
    DynCast { class: Str, target: Box<Expr> },
    Switch { index: Box<Expr>, cases: Vec<(Expr, Expr)>, default: Box<Expr> },
    ArrayGetByRef { array: Box<Expr>, index: Box<Expr> },
    Unsupported(Str),
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Expr(Expr),
    Jump(u32),
    JumpIfNot(Expr, u32),
    PushFlow(u32),
    PopFlow,
    PopFlowIfNot(Expr),
    Return(Expr),
    End,
    ComputedJump(Expr),
    /// Write into the ubergraph's persistent frame.
    LetPersistent(Str, Expr),
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: Str,
    pub out: bool,
    pub ret: bool,
}

#[derive(Debug)]
pub struct Function {
    pub name: Str,
    pub params: Vec<Param>,
    pub locals: Vec<(Str, Ty)>,
    pub stmts: Vec<(u32, Stmt)>,
    pub offsets: HashMap<u32, usize>,
}

impl Function {
    pub fn parse(j: &Json) -> Result<Function> {
        let name: Str = j["Name"].as_str().unwrap_or("?").into();
        let mut params = Vec::new();
        let mut locals = Vec::new();
        for p in j["ChildProperties"].as_array().into_iter().flatten() {
            let pname: Str = p["Name"].as_str().unwrap_or("?").into();
            let flags = p["PropertyFlags"].as_str().unwrap_or("");
            let has = |f: &str| flags.split(" | ").any(|x| x.trim() == f);
            if has("Parm") {
                params.push(Param { name: pname.clone(), out: has("OutParm"), ret: has("ReturnParm") });
            }
            locals.push((pname, Ty::from_property(p)));
        }
        let mut stmts = Vec::new();
        for s in j["ScriptBytecode"].as_array().into_iter().flatten() {
            let off = s["StatementIndex"].as_u64().context("statement without index")? as u32;
            let st = parse_stmt(s).with_context(|| format!("{name} @{off}"))?;
            stmts.push((off, st));
        }
        let offsets = stmts.iter().enumerate().map(|(i, (o, _))| (*o, i)).collect();
        Ok(Function { name, params, locals, stmts, offsets })
    }
}

fn parse_stmt(s: &Json) -> Result<Stmt> {
    let tok = s["Token"].as_str().unwrap_or("");
    let off = |k: &str| s[k].as_u64().map(|v| v as u32).ok_or_else(|| anyhow!("{tok}: missing {k}"));
    Ok(match tok {
        "EX_Jump" => Stmt::Jump(off("CodeOffset")?),
        "EX_JumpIfNot" => Stmt::JumpIfNot(parse_expr(&s["BooleanExpression"])?, off("CodeOffset")?),
        "EX_PushExecutionFlow" => Stmt::PushFlow(off("PushingAddress")?),
        "EX_PopExecutionFlow" => Stmt::PopFlow,
        "EX_PopExecutionFlowIfNot" => Stmt::PopFlowIfNot(parse_expr(&s["BooleanExpression"])?),
        "EX_Return" => Stmt::Return(parse_expr(&s["ReturnExpression"]).unwrap_or(Expr::Nothing)),
        "EX_EndOfScript" => Stmt::End,
        "EX_ComputedJump" => Stmt::ComputedJump(parse_expr(&s["CodeOffsetExpression"])?),
        "EX_LetValueOnPersistentFrame" => {
            Stmt::LetPersistent(prop_name(&s["DestinationProperty"]).into(), parse_expr(&s["AssignmentExpression"])?)
        }
        _ => Stmt::Expr(parse_expr(s)?),
    })
}

/// Name of a property reference, in either the `{Owner, Property}` or `{Path}` form.
fn prop_name(v: &Json) -> String {
    if let Some(n) = v["Property"]["Name"].as_str() {
        return n.to_string();
    }
    if let Some(p) = v["Path"].as_array().and_then(|a| a.last()).and_then(|x| x.as_str()) {
        return p.to_string();
    }
    "?".to_string()
}

fn exprs(v: &Json) -> Result<Vec<Expr>> {
    v.as_array().into_iter().flatten().map(parse_expr).collect()
}

fn pairs(v: &Json) -> Result<Vec<(Expr, Expr)>> {
    v.as_array().into_iter().flatten().map(|kv| Ok((parse_expr(&kv["Key"])?, parse_expr(&kv["Value"])?))).collect()
}

fn bx(v: &Json) -> Result<Box<Expr>> {
    Ok(Box::new(parse_expr(v)?))
}

pub fn parse_expr(e: &Json) -> Result<Expr> {
    if e.is_null() {
        return Ok(Expr::Nothing);
    }
    let tok = e["Token"].as_str().unwrap_or("");
    Ok(match tok {
        "EX_LocalVariable" | "EX_LocalOutVariable" => Expr::Local(prop_name(&e["Variable"]).into()),
        "EX_InstanceVariable" | "EX_DefaultVariable" | "EX_ClassSparseDataVariable" => Expr::Instance(prop_name(&e["Variable"]).into()),
        "EX_IntConst" | "EX_Int64Const" | "EX_UInt64Const" | "EX_IntConstByte" => Expr::Int(e["Value"].as_i64().unwrap_or(0)),
        "EX_IntZero" => Expr::Int(0),
        "EX_IntOne" => Expr::Int(1),
        "EX_FloatConst" | "EX_DoubleConst" => Expr::Float(e["Value"].as_f64().unwrap_or(0.0)),
        "EX_ByteConst" => Expr::Byte(e["Value"].as_u64().unwrap_or(0) as u8),
        "EX_True" => Expr::Bool(true),
        "EX_False" => Expr::Bool(false),
        "EX_NameConst" => Expr::Name(e["Value"].as_str().unwrap_or("None").into()),
        "EX_StringConst" | "EX_UnicodeStringConst" => Expr::Str(e["Value"].as_str().unwrap_or("").into()),
        "EX_TextConst" => {
            let v = &e["Value"];
            let s = v["KeyString"]["Value"].as_str().or(v["SourceString"].as_str()).unwrap_or("");
            Expr::Text(s.into())
        }
        "EX_Nothing" | "EX_NothingInt32" | "EX_EndFunctionParms" | "EX_Breakpoint" | "EX_Tracepoint" | "EX_WireTracepoint" => Expr::Nothing,
        "EX_Self" => Expr::SelfRef,
        "EX_NoObject" | "EX_NoInterface" => Expr::NoObject,
        "EX_ObjectConst" => Expr::Object(quoted(e["Value"]["ObjectName"].as_str().unwrap_or("")).into()),
        "EX_StructConst" => Expr::StructConst { ty: struct_name(&e["Struct"]).into(), fields: exprs(&e["Properties"])? },
        "EX_ArrayConst" | "EX_SetConst" => Expr::ArrayConst(exprs(&e["Values"])?),
        "EX_SetArray" | "EX_SetSet" => Expr::SetArray { target: bx(&e["AssigningProperty"])?, items: exprs(&e["Elements"])? },
        "EX_MapConst" => Expr::MapConst(pairs(&e["Values"])?),
        "EX_SetMap" => Expr::SetMap { target: bx(&e["MapProperty"])?, items: pairs(&e["Values"])? },
        "EX_CallMath" | "EX_FinalFunction" | "EX_LocalFinalFunction" => {
            let f = quoted(e["Function"]["ObjectName"].as_str().unwrap_or(""));
            let (owner, func) = f.rsplit_once(':').unwrap_or(("", f));
            Expr::Call { kind: CallKind::Final, owner: Some(owner.into()), func: func.into(), args: exprs(&e["Parameters"])? }
        }
        "EX_VirtualFunction" | "EX_LocalVirtualFunction" => {
            let f = e["Function"].as_str().map(str::to_string).unwrap_or_else(|| quoted(e["Function"]["ObjectName"].as_str().unwrap_or("")).to_string());
            Expr::Call { kind: CallKind::Virtual, owner: None, func: f.as_str().into(), args: exprs(&e["Parameters"])? }
        }
        "EX_CallMulticastDelegate" => Expr::Nothing,
        "EX_Context" | "EX_Context_FailSilent" | "EX_ClassContext" => Expr::Context { obj: bx(&e["ObjectExpression"])?, inner: bx(&e["ContextExpression"])? },
        "EX_InterfaceContext" => parse_expr(&e["InterfaceValue"])?,
        "EX_StructMemberContext" => Expr::Member { base: bx(&e["StructExpression"])?, field: prop_name(&e["Property"]).into() },
        "EX_Let" | "EX_LetBool" | "EX_LetObj" | "EX_LetWeakObjPtr" | "EX_LetDelegate" | "EX_LetMulticastDelegate" => {
            Expr::Let { target: bx(&e["Variable"])?, value: bx(&e["Expression"])? }
        }
        "EX_Cast" | "EX_PrimitiveCast" => Expr::Cast { conv: e["ConversionType"].as_str().unwrap_or("").into(), target: bx(&e["Target"])? },
        "EX_DynamicCast" | "EX_MetaCast" | "EX_ObjToInterfaceCast" | "EX_CrossInterfaceCast" | "EX_InterfaceToObjCast" => {
            Expr::DynCast { class: quoted(e["InterfaceClass"]["ObjectName"].as_str().or(e["ClassPtr"]["ObjectName"].as_str()).unwrap_or("")).into(), target: bx(&e["Target"])? }
        }
        "EX_SwitchValue" => {
            let mut cases = Vec::new();
            for c in e["Cases"].as_array().into_iter().flatten() {
                cases.push((parse_expr(&c["CaseIndexValueTerm"])?, parse_expr(&c["CaseTerm"])?));
            }
            Expr::Switch { index: bx(&e["IndexTerm"])?, cases, default: bx(&e["DefaultTerm"])? }
        }
        "EX_ArrayGetByRef" => Expr::ArrayGetByRef { array: bx(&e["ArrayVariable"])?, index: bx(&e["ArrayIndex"])? },
        "" => bail!("expression without token: {}", truncate(e)),
        other => Expr::Unsupported(other.into()),
    })
}

fn truncate(e: &Json) -> String {
    let s = e.to_string();
    s.chars().take(200).collect()
}
