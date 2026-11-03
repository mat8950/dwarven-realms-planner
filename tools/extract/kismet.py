"""Pretty-print CUE4Parse Kismet bytecode JSON as pseudo-code.

usage: python3 kismet.py <export.json> [function-name-substring ...]
"""
import json
import re
import sys


def short(path):
    s = path.split("'")[-2] if "'" in path else path
    return s.split(":")[-1].split(".")[-1]


def var(v):
    p = v.get("Variable", v)
    if isinstance(p, dict) and "Property" in p:
        return p["Property"]["Name"]
    if isinstance(p, dict) and "Path" in p:
        return p["Path"][-1]
    if isinstance(p, dict) and "New" in p:
        return p["New"]["Path"][-1] if p["New"].get("Path") else "?"
    return str(p)


MATH = {
    "Add_DoubleDouble": "+", "Subtract_DoubleDouble": "-", "Multiply_DoubleDouble": "*",
    "Divide_DoubleDouble": "/", "Greater_DoubleDouble": ">", "GreaterEqual_DoubleDouble": ">=",
    "Less_DoubleDouble": "<", "LessEqual_DoubleDouble": "<=", "EqualEqual_DoubleDouble": "==",
    "Add_IntInt": "+", "Subtract_IntInt": "-", "Multiply_IntInt": "*", "Divide_IntInt": "/",
    "Greater_IntInt": ">", "GreaterEqual_IntInt": ">=", "Less_IntInt": "<", "LessEqual_IntInt": "<=",
    "EqualEqual_IntInt": "==", "NotEqual_IntInt": "!=", "BooleanAND": "&&", "BooleanOR": "||",
    "Multiply_IntFloat": "*", "Percent_IntInt": "%", "NotEqual_DoubleDouble": "!=",
    "EqualEqual_NameName": "==", "NotEqual_NameName": "!=", "EqualEqual_ByteByte": "==",
    "NotEqual_ByteByte": "!=", "EqualEqual_BoolBool": "==",
}


def expr(e):
    if e is None:
        return "None"
    t = e.get("Token", "")
    if t in ("EX_LocalVariable", "EX_InstanceVariable", "EX_LocalOutVariable", "EX_DefaultVariable"):
        n = var(e)
        return ("self." if t == "EX_InstanceVariable" else "") + n
    if t in ("EX_DoubleConst", "EX_FloatConst", "EX_IntConst", "EX_ByteConst", "EX_Int64Const"):
        return repr(e["Value"])
    if t == "EX_NameConst":
        return f'"{e["Value"]}"'
    if t == "EX_StringConst":
        return f'"{e["Value"]}"'
    if t == "EX_True":
        return "true"
    if t == "EX_False":
        return "false"
    if t == "EX_Self":
        return "self"
    if t == "EX_Nothing":
        return ""
    if t == "EX_NoObject":
        return "null"
    if t == "EX_ObjectConst":
        return short(e["Value"]["ObjectName"]) if isinstance(e.get("Value"), dict) else str(e.get("Value"))
    if t == "EX_StructConst":
        sname = short(e["Struct"]["ObjectName"])
        props = [expr(p) for p in e.get("Properties", [])]
        if sname == "GameplayTag" and len(props) == 1:
            return "Tag(" + props[0].replace("EasyRPG.", "") + ")"
        return f"{sname}{{{', '.join(props)}}}"
    if t in ("EX_ArrayConst", "EX_SetArray"):
        items = e.get("Elements") or e.get("Values") or []
        lhs = expr(e["AssigningProperty"]) + " = " if t == "EX_SetArray" and e.get("AssigningProperty") else ""
        return lhs + "[" + ", ".join(expr(x) for x in items) + "]"
    if t == "EX_CallMath" or t == "EX_FinalFunction" or t == "EX_LocalFinalFunction" or t == "EX_VirtualFunction" or t == "EX_LocalVirtualFunction" or t == "EX_CallMulticastDelegate":
        fn = e.get("Function")
        fname = short(fn["ObjectName"]) if isinstance(fn, dict) else str(fn)
        args = [expr(p) for p in e.get("Parameters", [])]
        if fname in MATH and len(args) == 2:
            return f"({args[0]} {MATH[fname]} {args[1]})"
        if fname == "Not_PreBool":
            return f"!{args[0]}"
        return f"{fname}({', '.join(args)})"
    if t == "EX_Context" or t == "EX_Context_FailSilent":
        return f"{expr(e['ObjectExpression'])}.{expr(e['ContextExpression'])}"
    if t == "EX_StructMemberContext":
        member = var({"Variable": e["Property"]}) if isinstance(e["Property"], dict) else str(e["Property"])
        return f"{expr(e['StructExpression'])}.{re.sub(r'_[0-9]+_[0-9A-F]{32}$', '', member)}"
    if t in ("EX_Let", "EX_LetBool", "EX_LetObj", "EX_LetWeakObjPtr", "EX_LetDelegate", "EX_LetMulticastDelegate", "EX_LetValueOnPersistentFrame"):
        lhs = expr(e["Variable"]) if "Variable" in e else var({"Variable": e.get("DestinationProperty", {})})
        return f"{lhs} = {expr(e.get('Expression') or e.get('AssignmentExpression'))}"
    if t in ("EX_Cast", "EX_DynamicCast", "EX_MetaCast", "EX_ObjToInterfaceCast", "EX_PrimitiveCast", "EX_InterfaceToObjCast"):
        return f"cast({expr(e.get('Target') or e.get('Expression') or e.get('CastExpression'))})"
    if t == "EX_InterfaceContext":
        return expr(e["InterfaceValue"])
    if t == "EX_ArrayGetByRef":
        return f"{expr(e['ArrayVariable'])}[{expr(e['ArrayIndex'])}]"
    if t == "EX_TextConst":
        v = e.get("Value") or {}
        return '"' + str(v.get("SourceString") or v.get("Value") or v.get("LocalizedString") or v) + '"'
    if t in ("EX_BindDelegate", "EX_AddMulticastDelegate", "EX_RemoveMulticastDelegate", "EX_ClearMulticastDelegate"):
        return f"<{t[3:]} {e.get('FunctionName', '')}>"
    if t in ("EX_MapConst", "EX_SetMap", "EX_SetConst", "EX_SetSet"):
        vals = e.get("Values") or e.get("Elements") or []
        return "{" + ", ".join(expr(x) if isinstance(x, dict) else str(x) for x in vals) + "}"
    if t in ("EX_RotationConst", "EX_TransformConst", "EX_BitFieldConst", "EX_SkipOffsetConst", "EX_SoftObjectConst"):
        return f"{t[3:]}({e.get('Value')})"
    if t == "EX_SwitchValue":
        cases = ", ".join(f"{expr(c['CaseIndexValueTerm'])}: {expr(c['CaseTerm'])}" for c in e.get("Cases", []))
        return f"switch({expr(e['IndexTerm'])}) {{{cases}, default: {expr(e['DefaultTerm'])}}}"
    if t == "EX_VectorConst":
        return f"Vec{e.get('Value')}"
    return f"<{t}>"


def stmt(e):
    t = e.get("Token", "")
    if t == "EX_JumpIfNot":
        return f"if not {expr(e['BooleanExpression'])}: goto {e['CodeOffset']}"
    if t == "EX_Jump":
        return f"goto {e['CodeOffset']}"
    if t == "EX_PushExecutionFlow":
        return f"push_flow {e['PushingAddress']}"
    if t == "EX_PopExecutionFlow":
        return "pop_flow"
    if t == "EX_PopExecutionFlowIfNot":
        return f"pop_flow_if_not {expr(e['BooleanExpression'])}"
    if t == "EX_Return":
        r = expr(e.get("ReturnExpression") or e.get("Expression"))
        return "return " + r
    if t == "EX_EndOfScript":
        return "end"
    if t == "EX_ComputedJump":
        return f"goto computed {expr(e.get('CodeOffsetExpression'))}"
    return expr(e)


def dump(fn):
    out = [f"### {fn['Name']}"]
    params = [p["Name"] for p in fn.get("ChildProperties", []) if "Parm" in p.get("PropertyFlags", "")]
    out.append("params: " + ", ".join(params))
    for s in fn.get("ScriptBytecode") or []:
        out.append(f"{s.get('StatementIndex', '?'):>6}: {stmt(s)}")
    return "\n".join(out)


def main():
    data = json.load(open(sys.argv[1]))
    pats = sys.argv[2:]
    for e in data:
        if e.get("Type") != "Function":
            continue
        if pats and not any(p.lower() in e["Name"].lower() for p in pats):
            continue
        print(dump(e))
        print()


if __name__ == "__main__":
    main()
