#!/usr/bin/env python3
"""Enterprise codegen: Saleor schema.graphql -> lean async-graphql Rust.

Reads /tmp/schema_ir.json (registry) + dashboard gql docs, emits
crates/graphql/src/gen.rs containing ONLY what the Dashboard selects:

  * objects: dashboard-selected fields (intersected with schema), all
    nullable/relaxed outputs (stubs are trivially safe, no DB cost);
  * inputs/enums: exact Saleor shapes, but only the transitive closure
    needed by root args + variable declarations (variable coercion);
  * unions/interfaces: only where dashboard uses inline fragments;
  * missing query/mutation roots with zero-cost stub resolvers
    (None / [] / empty connections / { errors: [] }).

Phases:
  python3 scripts/schema_codegen.py --plan    # print coverage plan
  python3 scripts/schema_codegen.py --emit    # write gen.rs

Replaces incomplete hand-written types (see REPLACE list); kept types are
left untouched. Never hand-edit gen.rs — fix this script and re-run.
"""
import json
import re
import sys
import glob
import os

IR = "/tmp/schema_ir.json"
SCHEMA = "/home/matin/Desktop/dev/saleor/saleor-core/saleor/graphql/schema.graphql"
DASH_SRC = "/home/matin/Desktop/dev/saleor/dashboard/src"
OUT = "/home/matin/Desktop/dev/rustygod-saleor/crates/graphql/src/gen.rs"

# Hand-written GraphQL types that are COMPLETE for dashboard selections
# (verified vs usage) — codegen must not redefine these names.
KEPT = {
    "AccountError", "Address", "Announcement", "AppBrand", "AppBrandLogo",
    "AppCountableConnection", "AppExtension", "AppExtensionCountableConnection",
    "CountryDisplay", "CreateToken", "LanguageDisplay", "LimitInfo", "Limits",
    "MetadataItem", "OrderCountableConnection", "PageInfo", "Permission",
    "ProductCountableConnection", "ShopSettingsUpdate", "StockSettings",
    "UserPermission", "EventDeliveryAttemptCountableConnection",
    "EventDeliveryCountableConnection", "Query", "Mutation",
    # leaf helpers our schema owns
    "Image", "Money", "TaxedMoney",
}

# Hand-written types codegen REPLACES (incomplete vs dashboard). After --emit,
# delete these structs and adapt constructors to gen:: equivalents.
REPLACE = {
    "App", "AppProblem", "AppToken", "Channel", "EventDelivery",
    "EventDeliveryAttempt", "Order", "OrderLine", "Product", "ProductVariant",
    "User", "Webhook",
}

RUST_KW = {"as", "break", "const", "continue", "crate", "else", "enum", "extern",
           "false", "fn", "for", "if", "impl", "in", "let", "loop", "match",
           "mod", "move", "mut", "pub", "ref", "return", "self", "Self",
           "static", "struct", "super", "trait", "true", "type", "unsafe",
           "use", "where", "while", "async", "await", "dyn", "abstract",
           "become", "box", "do", "final", "macro", "override", "priv",
           "typeof", "unsized", "virtual", "yield", "try"}


def snake(name):
    s = re.sub(r"([A-Z]+)([A-Z][a-z])", r"\1_\2", name)
    s = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", s).lower()
    return ("r#" if s in RUST_KW else "") + s


def pascal(name):
    parts = re.split(r"[^A-Za-z0-9]+", name)
    out = "".join(p[:1].upper() + p[1:] for p in parts if p)
    if not out or out[0].isdigit():
        out = "V" + out
    if out in RUST_KW or out in ("Self", "Self_"):
        out += "_"
    return out


def argid(name):
    """Unused-proof argument ident: `_arg_where` (prefix defeats keywords,
    leading underscore defeats unused warnings)."""
    s = snake(name)
    if s.startswith("r#"):
        s = s[2:]
    if s.startswith("_"):
        s = "x" + s
    return "_arg_" + s


def unwrap(t):
    return re.sub(r"[\[\]!]", "", t)


def is_list(t):
    return t.startswith("[")


TOKEN = re.compile(r"\.\.\.|[A-Za-z_][A-Za-z0-9_]*|[{()}:,!=$@\"\[\]]|\$[A-Za-z_][A-Za-z0-9_]*")


def tokenize(doc):
    doc = re.sub(r"\$\{[^{}]*\}", " ", doc)
    doc = re.sub(r'"(?:[^"\\]|\\.)*"', '""', doc)
    return TOKEN.findall(doc)


def parse_selection(toks, pos=0):
    """-> (sels, pos). sel = (name, sub|None, cond|None, has_args)."""
    assert toks[pos] == "{"
    pos += 1
    sels = []
    while pos < len(toks) and toks[pos] != "}":
        t = toks[pos]
        if t == "...":
            pos += 1
            if toks[pos] == "on":
                cond = toks[pos + 1]
                pos += 2
                if pos < len(toks) and toks[pos] == "{":
                    sub, pos = parse_selection(toks, pos)
                    sels.append(("...on", sub, cond, False))
                else:
                    sels.append(("...on", [], cond, False))
            else:
                sels.append(("..." + toks[pos], None, None, False))
                pos += 1
        elif re.match(r"[A-Za-z_]", t) and t != "__typename":
            if t.startswith("__"):
                pos += 1
                continue
            name = t
            pos += 1
            if pos < len(toks) and toks[pos] == ":":
                pos += 1  # alias: real field follows; keep alias (response key)
                name = toks[pos]
                pos += 1
            has_args = False
            while pos < len(toks) and toks[pos] in ("(", "@"):
                if toks[pos] == "@":
                    pos += 2 if pos + 1 < len(toks) and toks[pos + 1] not in ("(", "{") else 1
                    if pos < len(toks) and toks[pos] == "(":
                        d = 1
                        pos += 1
                        while d:
                            d += toks[pos] == "("
                            d -= toks[pos] == ")"
                            pos += 1
                else:
                    has_args = True
                    d = 1
                    pos += 1
                    while d:
                        d += toks[pos] == "("
                        d -= toks[pos] == ")"
                        pos += 1
            if pos < len(toks) and toks[pos] == "{":
                sub, pos = parse_selection(toks, pos)
                sels.append((name, sub, None, has_args))
            else:
                sels.append((name, None, None, has_args))
        else:
            pos += 1
    return sels, pos + 1


def load_docs():
    ops, frags = [], {}
    for pat in (DASH_SRC + "/**/*.ts", DASH_SRC + "/**/*.tsx"):
        for f in glob.glob(pat, recursive=True):
            if "hooks.generated" in f or ".test." in f or ".stories." in f or "fixtures" in f:
                continue
            try:
                t = open(f, errors="replace").read()
            except OSError:
                continue
            for m in re.finditer(r"gql`(.*?)`", t, re.S):
                doc = m.group(1)
                for fm in re.finditer(r"fragment\s+(\w+)\s+on\s+(\w+)\s*\{", doc):
                    fname, ftype = fm.groups()
                    j = doc.find("{", fm.end() - 1)
                    try:
                        sels, _ = parse_selection(tokenize(doc[j:]))
                        frags[fname] = (ftype, sels)
                    except (AssertionError, IndexError):
                        pass
                om = re.search(r"^\s*(query|mutation|subscription)\s+(\w+)", doc, re.M)
                if not om:
                    continue
                kind, name = om.groups()
                j = doc.find("{", om.end())
                # variable declarations: $v: Type
                var_types = dict(re.findall(r"\$(\w+)\s*:\s*([A-Za-z_][A-Za-z0-9_\[\]!]*)", doc[:j]))
                try:
                    sels, _ = parse_selection(tokenize(doc[j:]))
                except (AssertionError, IndexError):
                    continue
                ops.append({"kind": kind, "name": name, "sel": sels,
                            "vars": var_types, "page": f.split("dashboard/src/")[-1].split("/")[0]})
    return ops, frags


SCALAR_MAP = {
    "String": "String", "Int": "i32", "Float": "f64", "Boolean": "bool",
    "ID": "ID", "Decimal": "String", "PositiveDecimal": "String",
    "Date": "DateTime<Utc>", "DateTime": "DateTime<Utc>",
    "DateTimeTz": "DateTime<Utc>", "Day": "i32", "Hour": "i32",
    "Minute": "i32", "UUID": "String", "JSONString": "String",
    "JSON": "serde_json::Value", "GenericScalar": "serde_json::Value",
    "Metadata": "serde_json::Value", "WeightScalar": "serde_json::Value",
    "PositiveInt": "i32", "Upload": "GenUpload", "_Any": "String",
}


class Gen:
    def __init__(self, reg):
        self.reg = reg
        self.usage = {}      # typename -> {field: has_args}
        self.cond = {}       # inline-conditioned typename -> set(contexts)
        self.fragcond = {}   # fragment-conditioned typename -> set(contexts)
        self.var_types = set()
        self.implements = {}  # object -> [interfaces]
        self.parse_implements()

    def parse_implements(self):
        src = open(SCHEMA).read()
        for m in re.finditer(r"^type\s+([A-Za-z_][A-Za-z0-9_]*)\s+implements\s+([A-Za-z_0-9\s&|!,\[\]]+?)(?:@|\{|$)", src, re.M):
            name, impls = m.groups()
            self.implements[name] = re.findall(r"[A-Za-z_][A-Za-z0-9_]*", impls)

    def walk(self, sels, typename, ctx="root"):
        types = self.reg["types"]
        if typename in ("Query", "Mutation"):
            fields = self.reg["query"] if typename == "Query" else self.reg["mutation"]
        elif typename in types:
            fields = types[typename]["fields"]
        elif typename in self.reg.get("unions", {}):
            # union parent: no direct fields; descend into spreads only
            for name, sub, cond, _ in sels:
                if name == "...on":
                    self.cond.setdefault(cond, set()).add(ctx)
                    self.walk(sub, cond, ctx + ">" + cond)
                elif name.startswith("..."):
                    f = FRAGS.get(name[3:])
                    if f:
                        self.fragcond.setdefault(f[0], set()).add(ctx)
                        self.walk(f[1], f[0], ctx + ">" + name)
            return
        else:
            return
        for name, sub, cond, has_args in sels:
            if name == "...on":
                self.cond.setdefault(cond, set()).add(ctx)
                self.walk(sub, cond, ctx + ">" + cond)
                continue
            if name.startswith("..."):
                f = FRAGS.get(name[3:])
                if f:
                    # fragment-on-type conditioning (unions/interfaces need this)
                    self.fragcond.setdefault(f[0], set()).add(ctx)
                    self.walk(f[1], f[0], ctx + ">" + name)
                continue
            if name not in fields:
                continue
            u = self.usage.setdefault(typename, {})
            u[name] = u.get(name, False) or has_args
            if sub:
                # record parent even for __typename-only children
                self.usage.setdefault(unwrap(fields[name]["type"]), {})
                self.walk(sub, unwrap(fields[name]["type"]), ctx + "." + name)

    def analyze(self, ops):
        for op in ops:
            base = "Query" if op["kind"] == "query" else "Mutation"
            self.walk(op["sel"], base, op["name"])
            for v in op["vars"].values():
                self.var_types.add(unwrap(v))
        # keep only schema-known fields (drops parser noise)
        types = self.reg["types"]
        for t in list(self.usage):
            if t in ("Query", "Mutation"):
                known = self.reg["query"] if t == "Query" else self.reg["mutation"]
            elif t in types:
                known = types[t]["fields"]
            else:
                # union/interface/unknown parent: keep as-is (handled specially)
                continue
            self.usage[t] = {k: v for k, v in self.usage[t].items() if k in known}


FRAGS = {}


def split_top_level(body):
    """Split a block into top-level entries, respecting parens/braces and
    triple-quoted description strings."""
    entries, depth, cur, instr = [], 0, [], False
    i = 0
    while i < len(body):
        c = body[i]
        if c == '"' and body[i:i+3] == '"""':
            instr = not instr
            cur.append('"""'); i += 3; continue
        if not instr:
            if c in "([{":
                depth += 1
            elif c in ")]}":
                depth -= 1
            elif c == "\n" and depth == 0:
                entries.append("".join(cur)); cur = []
                i += 1; continue
        cur.append(c); i += 1
    if "".join(cur).strip():
        entries.append("".join(cur))
    return [e.strip() for e in entries if e.strip()]


def split_args(raw):
    """'(a: T, b: U!)' -> [(name, type)]. Strips description strings."""
    inner = raw.strip()
    if not (inner.startswith("(") and inner.endswith(")")):
        return []
    out = []
    for entry in split_top_level(inner[1:-1]):
        entry = re.sub(r'^""".*?"""\s*', "", entry, flags=re.S).strip()
        m = re.match(r"([A-Za-z_][A-Za-z0-9_]*)\s*:\s*(=\s*[^,]+?,\s*)?([A-Za-z_\[][A-Za-z0-9_\[\]!]*)", entry)
        if m:
            out.append((m.group(1), m.group(3)))
    return out


KEPT_PATHS = {
    "MetadataItem": "crate::common::MetadataItem",
    "Permission": "crate::commerce::GqlPermission",
    "Money": "crate::common::Money",
    "TaxedMoney": "crate::order::GqlTaxedMoney",
    "Image": "crate::account::GqlImage",
    "UserPermission": "crate::account::GqlUserPermission",
    "CountryDisplay": "crate::common::GqlCountryDisplay",
    "StockSettings": "crate::common::GqlStockSettings",
    "PageInfo": "crate::common::PageInfo",
    "LanguageDisplay": "crate::commerce::GqlLanguageDisplay",
    "LimitInfo": "crate::commerce::GqlLimitInfo",
    "Limits": "crate::commerce::GqlLimits",
    "AccountError": "crate::account::GqlAccountError",
    "Address": "crate::order::GqlAddress",
    "Announcement": "crate::commerce::GqlAnnouncement",
    "AppBrand": "crate::apps::GqlAppBrand",
    "AppBrandLogo": "crate::apps::GqlAppBrandLogo",
    "AppExtension": "crate::apps::GqlAppExtension",
    "CreateToken": "crate::account::GqlTokenCreate",
    "AppCountableConnection": "crate::apps::GqlAppConnection",
    "AppExtensionCountableConnection": "crate::apps::GqlAppExtensionConnection",
    "OrderCountableConnection": "crate::order::GqlOrderConnection",
    "ProductCountableConnection": "crate::catalog::GqlProductConnection",
    "EventDeliveryCountableConnection": "crate::apps::GqlEventDeliveryConnection",
    "EventDeliveryAttemptCountableConnection": "crate::apps::GqlEventDeliveryAttemptConnection",
}


def build_model(g, reg, ops):
    """Coherent generation model from analyzed docs."""
    types, inputs, enums, unions = reg["types"], reg["inputs"], reg["enums"], reg["unions"]
    iface_names = {t for t, v in types.items() if v["kind"] == "interface"}
    ours = set(json.load(open(IR))["ours"])
    conditioned = set(g.cond) | set(g.fragcond)
    M = {"structs": {}, "unions": {}, "ifaces": {}, "inputs": {},
         "enums": {}, "roots": [], "methods": [], "boxes": set(),
         "union_fields": {}, "reports": []}
    R = M["reports"]

    # ---- spread parents: fragment -> concrete parents it's spread under ----
    spread_parents = {}  # (fragname, fragtype) -> set(parents)

    def walk_spread(sels, typename, parent):
        fields = None
        if typename in ("Query", "Mutation"):
            fields = reg["query"] if typename == "Query" else reg["mutation"]
        elif typename in types:
            fields = types[typename]["fields"]
        for name, sub, cond, _ in sels:
            if name == "...on":
                walk_spread(sub, cond, parent)
            elif name.startswith("..."):
                f = FRAGS.get(name[3:])
                if f:
                    spread_parents.setdefault((name[3:], f[0]), set()).add(parent)
                    walk_spread(f[1], f[0], parent)
            elif sub and fields and name in fields:
                walk_spread(sub, unwrap(fields[name]["type"]), unwrap(fields[name]["type"]))

    for op in ops:
        walk_spread(op["sel"], "Query" if op["kind"] == "query" else "Mutation", None)
    for fname, (ftype, sels) in FRAGS.items():
        walk_spread(sels, ftype, None)

    frag_parents_of = {}  # interface -> set(concrete parents of frag-on-iface spreads)
    for (fname, ftype), parents in spread_parents.items():
        if ftype in iface_names:
            frag_parents_of.setdefault(ftype, set()).update(
                p for p in parents if p and p in types and types[p]["kind"] == "type")

    # ---- interfaces ----
    for iface in sorted(({t for t in g.usage if t in iface_names}
                         | {t for t in conditioned if t in iface_names}
                         | set(frag_parents_of))):
        if iface not in types:
            continue
        fields = {f for f in g.usage.get(iface, {}) if f in types[iface]["fields"]}
        impls = set()
        for c in conditioned:
            if c in g.implements and iface in g.implements[c]:
                impls.add(c)
        impls |= frag_parents_of.get(iface, set())
        if iface == "Node":
            impls |= frag_parents_of.get("ObjectWithMetadata", set())
        impls = {c for c in impls if c in types and types[c]["kind"] == "type"}
        if not impls:
            R.append(f"iface {iface}: NO implementors, skipped")
            continue
        M["ifaces"][iface] = {"fields": sorted(fields), "implementors": sorted(impls)}

    # ---- unions ----
    for u, members in unions.items():
        if u in ("_Entity", "_Service"):
            continue
        cond_here = conditioned & set(members)
        used = [m for m in members if m in g.usage]
        if u in g.usage or cond_here:
            variants = sorted(set(cond_here) | set(used)) or list(members)
            M["unions"][u] = [v for v in variants if v in types]

    # ---- union-typed fields (diverge) ----
    field_conds = {}
    for op in ops:
        collect_conds(op["sel"], "Query" if op["kind"] == "query" else "Mutation",
                      types, reg, field_conds, set())
    for fname, (ftype, sels) in FRAGS.items():
        collect_conds(sels, ftype, types, reg, field_conds, set())
    for (parent, field), conds in field_conds.items():
        if field == "*":
            continue
        ftype = field_type_of(parent, field, types, reg)
        if not ftype:
            continue
        base = unwrap(ftype)
        if base in unions:
            others = sorted({c for c in conds if c != base and c in types
                             and types[c]["kind"] == "type"})
            if others:
                M["union_fields"][(parent, field)] = (base, others)
                for o in others:
                    if o not in M["unions"].get(base, []):
                        M["unions"].setdefault(base, [])
                        if o not in M["unions"][base]:
                            M["unions"][base].append(o)
        elif base in iface_names:
            pass  # real interface handles it (implementors include conditioned)

    # ---- structs ----
    want = {t for t in g.usage if t in types and types[t]["kind"] == "type"
            and t not in ("Query", "Mutation")} - set(KEPT)
    want |= {c for c in conditioned if c in types and types[c]["kind"] == "type"} - set(KEPT)
    for u, vs in M["unions"].items():
        want.update(v for v in vs if v not in KEPT)
    for _i, info in M["ifaces"].items():
        want.update(i for i in info["implementors"] if i not in KEPT)
    for t in sorted(want):
        if t in types and types[t]["kind"] == "type":
            M["structs"][t] = True

    # ---- roots ----
    # NOTE: one operation can select MULTIPLE roots (e.g. UpdateShopSettings
    # selects shopSettingsUpdate + shopAddressUpdate) — register them all.
    op_roots = {(o["kind"], s[0]) for o in ops if o["sel"] for s in o["sel"]
                if not s[0].startswith("...")}
    for kind, roots in (("query", reg["query"]), ("mutation", reg["mutation"])):
        for name, info in roots.items():
            if name in ours or name.startswith("__"):
                continue
            if (kind, name) not in op_roots:
                continue
            M["roots"].append((kind, name, info))

    # ---- inputs/enums closure ----
    need_inputs, need_enums = set(), set()

    def use_in_ty(t):
        b = unwrap(t)
        if b in inputs:
            if b in need_inputs:
                return
            need_inputs.add(b)
            for _f, finfo in inputs[b].items():
                use_in_ty(finfo["type"])
        elif b in enums:
            need_enums.add(b)

    for kind, name, info in M["roots"]:
        for _, at in split_args(info["args"]):
            use_in_ty(at)
    for v in g.var_types:
        if v in inputs or v in enums:
            use_in_ty(v)
    for t in M["structs"]:
        for f, has_args in g.usage.get(t, {}).items():
            if has_args and f in types[t]["fields"]:
                args = split_args(types[t]["fields"][f]["args"])
                M["methods"].append((t, f, args))
                for _, at in args:
                    use_in_ty(at)
    M["inputs"] = {k: inputs[k] for k in sorted(need_inputs)}
    M["enums"] = {k: enums[k] for k in sorted(need_enums)}

    # ---- boxes ----
    gen_names = set(M["structs"])
    deps = {}
    for t in M["structs"]:
        for f in g.usage.get(t, {}):
            if f not in types[t]["fields"]:
                continue
            b = unwrap(types[t]["fields"][f]["type"])
            if b in gen_names or b in M["unions"] or b in M["ifaces"]:
                deps[(t, f)] = b
    member_edges = {u: vs for u, vs in M["unions"].items()}
    member_edges.update({i: info["implementors"] for i, info in M["ifaces"].items()})
    M["boxes"] = find_boxes(gen_names, set(M["unions"]) | set(M["ifaces"]), deps, member_edges)
    return M


def plan(g, reg, ops, emit=False):
    M = build_model(g, reg, ops)
    types = reg["types"]
    STRUCTS = set(M["structs"])
    # ---- verification ----
    problems = []
    for kind, name, info in M["roots"]:
        b = unwrap(info["type"])
        if is_list(info["type"]):
            ok = (b in reg["enums"] or b in SCALAR_MAP or b in STRUCTS
                  or b in KEPT_PATHS or b in M["unions"] or b in M["ifaces"])
        elif kind == "mutation":
            ok = b in STRUCTS or b in KEPT_PATHS
        else:
            ok = (b in reg["enums"] or b in SCALAR_MAP or b in STRUCTS
                  or b in KEPT_PATHS or b in M["unions"] or b in M["ifaces"])
        if not ok:
            problems.append(f"root {kind} {name}: unresolvable return {info['type']}")
    our_fields = json.load(open("/tmp/our_fields.json"))
    for t in KEPT:
        if t in ("Query", "Mutation"):
            continue
        need = set(g.usage.get(t, {})) - set(our_fields.get(t, []))
        if need:
            problems.append(f"KEPT {t} missing dashboard fields: {sorted(need)}")
    for iname, info in M["ifaces"].items():
        for v in info["implementors"]:
            if v in KEPT and v in our_fields:
                missing = [f for f in info["fields"] if f not in our_fields[v]]
                if missing:
                    problems.append(f"iface {iname} impl KEPT {v} lacks {missing}")
    n_methods = sum(1 for t in M["structs"] for f, ha in g.usage.get(t, {}).items() if ha)
    print(f"OPS={len(ops)} STRUCTS={len(M['structs'])} IFACES={list(M['ifaces'])} "
          f"UNIONS={list(M['unions'])} INPUTS={len(M['inputs'])} ENUMS={len(M['enums'])} "
          f"ROOTS={len(M['roots'])} METHODS~{n_methods} BOXES={len(M['boxes'])} "
          f"UNION_FIELDS={len(M['union_fields'])} PROBLEMS={len(problems)}")
    for (p, f), (b, others) in sorted(M["union_fields"].items()):
        print(f"  unionfield {p}.{f}: declared={b} others={others}")
    for iname, info in M["ifaces"].items():
        print(f"  iface {iname}: fields={info['fields']} impl={len(info['implementors'])}")
    for u, vs in M["unions"].items():
        print(f"  union {u}: {vs}")
    for r in M["reports"]:
        print(f"  NOTE: {r}")
    for p in problems:
        print(f"  PROBLEM: {p}")
    # kept types with arg-methods needed (hand-add later)
    types = reg["types"]
    for t, fmap in g.usage.items():
        if t in KEPT and t in types:
            for f, ha in fmap.items():
                if ha and f in types[t]["fields"]:
                    print(f"  HAND-METHOD needed: {t}.{f}{types[t]['fields'][f]['args'][:60]}")
    if emit:
        emit_all(reg, g, M)
    return 0


KEPT_INPUTS = {"MetadataInput": "crate::common::MetadataInput"}
KEPT_ENUMS = {
    "IconThumbnailFormatEnum": "crate::apps::IconThumbnailFormatEnum",
    "AppTypeEnum": "crate::apps::AppTypeEnum",
}
# Hand inputs that must be DELETED and replaced by generated full shapes
# (dashboard sends full Saleor inputs to these).
DELETE_HAND_INPUTS = {"ProductCreateInput"}
# Main-schema (3.23) fields removed in 3.24 that the released dashboard still
# selects (its @lockSchema client gate keeps them on main-mode builds).
# Emitted as stub fields so both schema modes validate.
LEGACY_FIELDS = {
    "Attribute": [
        ("filterableInStorefront", "Option<bool>"),
        ("availableInGrid", "Option<bool>"),
        ("storefrontSearchPosition", "Option<i32>"),
    ],
}


def emit_all(reg, g, M):
    types = reg["types"]
    STRUCTS = set(M["structs"])
    UNIONS = set(M["unions"])
    IFACES = set(M["ifaces"])
    O = []
    W = O.append
    W("//! GENERATED — DO NOT EDIT. Source: scripts/schema_codegen.py\n"
      "//! Saleor schema.graphql shapes, dashboard-selected fields only.\n"
      "//! All outputs relaxed-nullable; stubs are zero-cost (no DB).\n"
      "//! Real-data wiring: replace stub bodies, keep names.\n")
    W("use async_graphql::*;\nuse chrono::{DateTime, Utc};\n")
    W("use crate::common::PageInfo;\n")

    def in_rust(gql):
        base, lst, nn = unwrap(gql), is_list(gql), gql.endswith("!")
        if base in KEPT_INPUTS:
            r = KEPT_INPUTS[base]
        elif base in KEPT_ENUMS:
            r = KEPT_ENUMS[base]
        elif base in M["inputs"]:
            r = base
        elif base in M["enums"]:
            r = base
        elif base in SCALAR_MAP:
            r = SCALAR_MAP[base]
        else:
            M["reports"].append(f"in_ty fallback JSON for {gql}")
            r = "serde_json::Value"
        if lst:
            inner = r
            return f"Vec<{inner}>" if nn or True else f"Vec<{inner}>"
        return r if nn else f"Option<{r}>"

    def out_rust(parent, field, gql):
        if (parent, field) in M["union_fields"]:
            u, _ = M["union_fields"][(parent, field)]
            r = u
        else:
            base = unwrap(gql)
            if base in reg["enums"]:
                r = "String"
            elif base in SCALAR_MAP:
                r = SCALAR_MAP[base]
            elif base in KEPT_PATHS:
                r = KEPT_PATHS[base]
            elif base in STRUCTS or base in UNIONS or base in IFACES:
                r = base
            else:
                M["reports"].append(f"out_ty fallback JSON for {parent}.{field}: {gql}")
                r = "serde_json::Value"
        boxed = (parent, field) in M["boxes"]
        if is_list(gql):
            return f"Vec<{'Box<' + r + '>' if boxed else r}>"
        return f"Option<{'Box<' + r + '>' if boxed else r}>"

    def method_default(ret):
        return "vec![]" if ret.startswith("Vec<") else "None"

    def gql_name(t):
        return t

    # ---- Upload scalar (multipart deferred; JSON-string stub keeps SDL valid) ----
    W("\n#[derive(Clone, Debug)]\npub struct GenUpload;\n")
    W("#[Scalar(name = \"Upload\")]\nimpl ScalarType for GenUpload {\n"
      "    fn parse(value: Value) -> InputValueResult<Self> {\n"
      "        match &value { Value::String(_) | Value::Null => Ok(GenUpload), _ => Err(InputValueError::expected_type(value)), }\n"
      "    }\n"
      "    fn to_value(&self) -> Value { Value::Null }\n}\n")
    for k in list(SCALAR_MAP):
        if k == "Upload":
            SCALAR_MAP[k] = "GenUpload"
    # ---- String-backed Saleor scalars (wire-compatible: Saleor serializes
    # Decimal/PositiveDecimal/JSONString as JSON strings). WeightScalar is
    # lenient (accepts numbers too) since variants send raw weights.
    W("\nmacro_rules! gen_string_scalar {\n"
      "    ($rust:ident, $gql:literal) => {\n"
      "        #[derive(Clone, Debug)]\n"
      "        pub struct $rust;\n"
      "        #[Scalar(name = $gql)]\n"
      "        impl ScalarType for $rust {\n"
      "            fn parse(value: Value) -> InputValueResult<Self> {\n"
      "                match &value { Value::String(_) | Value::Null => Ok($rust), _ => Err(InputValueError::expected_type(value)), }\n"
      "            }\n"
      "            fn to_value(&self) -> Value { Value::Null }\n"
      "        }\n"
      "    };\n"
      "}\n")
    W("gen_string_scalar!(GenDecimal, \"Decimal\");\n")
    W("gen_string_scalar!(GenPositiveDecimal, \"PositiveDecimal\");\n")
    W("gen_string_scalar!(GenJSONString, \"JSONString\");\n")
    W("#[derive(Clone, Debug)]\npub struct GenWeightScalar;\n")
    W("#[Scalar(name = \"WeightScalar\")]\nimpl ScalarType for GenWeightScalar {\n"
      "    fn parse(value: Value) -> InputValueResult<Self> {\n"
      "        match &value { Value::String(_) | Value::Number(_) | Value::Null => Ok(GenWeightScalar), _ => Err(InputValueError::expected_type(value)), }\n"
      "    }\n"
      "    fn to_value(&self) -> Value { Value::Null }\n}\n")
    SCALAR_MAP["Decimal"] = "GenDecimal"
    SCALAR_MAP["PositiveDecimal"] = "GenPositiveDecimal"
    SCALAR_MAP["JSONString"] = "GenJSONString"
    SCALAR_MAP["WeightScalar"] = "GenWeightScalar"

    # ---- enums ----
    for ename in sorted(M["enums"]):
        if ename in KEPT_ENUMS:
            continue
        W(f"\n#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]\npub enum {ename} {{")
        for v in reg["enums"][ename]:
            W(f"\n    #[graphql(name = \"{v}\")]\n    {pascal(v)},")
        W("\n}\n")

    # ---- inputs (exact shapes) ----
    for iname in sorted(M["inputs"]):
        if iname in KEPT_INPUTS:
            continue
        W(f"\n#[derive(InputObject, Clone, Debug)]\n#[graphql(name = \"{iname}\")]\npub struct {iname} {{")
        for fname, finfo in reg["inputs"][iname].items():
            rname = snake(fname)
            W(f"\n    #[graphql(name = \"{fname}\")]\n    pub {rname}: {in_rust(finfo['type'])},")
        W("\n}\n")

    # ---- structs ----
    def legacy_ty(t, fname):
        for lf, lr in LEGACY_FIELDS.get(t, []):
            if lf == fname:
                return lr
        return None

    def out_field_ty(t, fname):
        """Rust output type for a struct field (schema-driven or legacy)."""
        leg = legacy_ty(t, fname)
        if leg is not None:
            return leg
        return out_rust(t, fname, types[t]["fields"][fname]["type"])

    def struct_fields(t):
        """(fname, has_args, forced) in schema order, schema-known only."""
        order = [f for f in types[t]["fields"]]
        sel = {f for f in g.usage.get(t, {}) if f in types[t]["fields"]}
        forced = set()
        for iname, info in M["ifaces"].items():
            if t in info["implementors"]:
                forced.update(info["fields"])
        out = [(f, g.usage.get(t, {}).get(f, False), f in forced)
               for f in order if f in sel or f in forced]
        if t in LEGACY_FIELDS:
            have = {f for f, _, _ in out}
            for fname, _rty in LEGACY_FIELDS[t]:
                if fname not in have:
                    out.append((fname, False, False))
        return out or fallback_fields(t)

    def fallback_fields(t):
        for cand in ("id", "cursor", "node"):
            if cand in types[t]["fields"]:
                M["reports"].append(f"{t}: zero selected fields, fallback {cand}")
                return [(cand, False, False)]
        f0 = next(iter(types[t]["fields"]))
        M["reports"].append(f"{t}: zero selected fields, fallback {f0}")
        return [(f0, False, False)]

    for t in sorted(STRUCTS):
        plain = [(f, ha) for f, ha, _ in struct_fields(t) if not ha]
        methods = [(f, split_args(types[t]["fields"][f]["args"]))
                   for f, ha, _ in struct_fields(t) if ha]
        if not plain:
            # Method-only shape (e.g. AppManifestBrandLogo.default): unit
            # struct + Object impl (a field-less SimpleObject is illegal).
            W(f"\n#[derive(Clone, Default)]\npub struct {t};\n")
            W(f"\n#[Object(name = \"{t}\")]\nimpl {t} {{")
            for fname, args in methods:
                rname = snake(fname)
                sig = ", ".join(
                    f"#[graphql(name = \"{an}\")] {argid(an)}: {in_rust(at)}"
                    for an, at in args)
                ret = out_field_ty(t, fname)
                W(f"\n    async fn {rname}(&self{', ' + sig if sig else ''}) -> {ret} {{")
                W(f"\n        {method_default(ret)}")
                W("\n    }")
            W("\n}\n")
            continue
        # NOTE: `complex` is required for ComplexObject methods to register.
        backers = []
        for iname, info in M["ifaces"].items():
            if t not in info["implementors"]:
                continue
            for f in info["fields"]:
                if f not in types[t]["fields"]:
                    continue
                is_method = any(f == mf for mf, _ in methods)
                backers.append((f, is_method))
        complex = ", complex" if (methods or backers) else ""
        W(f"\n#[derive(SimpleObject, Clone)]\n#[graphql(name = \"{t}\"{complex})]\npub struct {t} {{")
        for fname, _ha in plain:
            rname = snake(fname)
            W(f"\n    #[graphql(name = \"{fname}\")]\n    pub {rname}: {out_field_ty(t, fname)},")
        W("\n}\n")
        # interface backers: gen_iface_* forward to the real field clone.
        # (Interface fields dispatch via method= to these. Arg-methods are
        # awaited with empty defaults.)
        if methods or backers:
            W(f"\n#[ComplexObject]\nimpl {t} {{")
            for fname, args in methods:
                rname = snake(fname)
                sig = ", ".join(
                    f"#[graphql(name = \"{an}\")] {argid(an)}: {in_rust(at)}"
                    for an, at in args)
                ret = out_field_ty(t, fname)
                W(f"\n    #[graphql(name = \"{fname}\")]\n    async fn {rname}(&self{', ' + sig if sig else ''}) -> {ret} {{")
                W(f"\n        {method_default(ret)}")
                W("\n    }")
            for f, is_method in backers:
                rname = snake(f)
                ret = out_rust(t, f, types[t]["fields"][f]["type"])
                if is_method:
                    # Same default the stub method itself returns (no ctx
                    # plumbing; keep in sync if the method gains real logic).
                    W(f"\n    pub async fn gen_iface_{rname}(&self) -> {ret} {{")
                    W(f"\n        {method_default(ret)}")
                    W("\n    }")
                else:
                    W(f"\n    pub async fn gen_iface_{rname}(&self) -> {ret} {{")
                    W(f"\n        self.{rname}.clone()")
                    W("\n    }")
            W("\n}\n")

    # ---- unions ----
    for u in sorted(UNIONS):
        vs = M["unions"][u]
        W(f"\n#[derive(Union, Clone)]\n#[graphql(name = \"{u}\")]\npub enum {u} {{")
        for v in vs:
            vp = KEPT_PATHS.get(v, v)
            W(f"\n    {pascal(v)}({vp}),")
        W("\n}\n")

    # ---- interfaces ----
    # Fields dispatch via method="gen_iface_*" to backer methods emitted on
    # every implementor (direct field access would demand From<&T> bounds
    # std doesn't provide for Option/Vec/object types).
    for iname in sorted(IFACES):
        info = M["ifaces"][iname]
        if info["fields"]:
            fattrs = ", ".join(
                f'field(name = "{f}", method = "gen_iface_{snake(f)}", '
                f'ty = "{out_rust(iname, f, types[iname]["fields"][f]["type"])}")'
                for f in info["fields"])
            fattrs = f"#[graphql({fattrs})]\n"
        else:
            fattrs = ""
            M["reports"].append(f"iface {iname}: zero fields")
        W(f"\n#[derive(Interface, Clone)]\n{fattrs}"
          f"#[graphql(name = \"{iname}\")]\npub enum {iname} {{")
        for v in info["implementors"]:
            vp = KEPT_PATHS.get(v, v)
            W(f"\n    {pascal(v)}({vp}),")
        W("\n}\n")

    # ---- roots ----
    def arg_sig(args):
        return ", ".join(
            f"#[graphql(name = \"{an}\")] {argid(an)}: {in_rust(at)}" for an, at in args)

    def leaf(base):
        if base in reg["enums"]:
            return "String"
        if base in SCALAR_MAP:
            return SCALAR_MAP[base]
        if base in KEPT_PATHS:
            return KEPT_PATHS[base]
        return base

    def conn_default(base):
        path = KEPT_PATHS.get(base, base)
        if base in STRUCTS:
            sfields = {snake(f) for f, _, _ in struct_fields(base)}
        else:
            sfields = {"total_count", "edges", "page_info"}
        parts = []
        if "total_count" in sfields:
            parts.append("total_count: None")
        if "edges" in sfields:
            parts.append("edges: vec![]")
        if "page_info" in sfields:
            lit = ("PageInfo { has_next_page: false, has_previous_page: false, "
                   "start_cursor: None, end_cursor: None }")
            # kept connections hold PageInfo by value; generated hold Option
            parts.append(f"page_info: {lit if base in KEPT_PATHS else f'Some({lit})'}")
        if not parts:
            M["reports"].append(f"conn {base}: no standard fields")
            return "None"
        return f"Some({path} {{ {', '.join(parts)} }})"

    def root_ret(gql):
        base, lst = unwrap(gql), is_list(gql)
        if lst:
            return f"Vec<{leaf(base)}>"
        return f"Option<{leaf(base)}>"

    def root_default(gql):
        base, lst = unwrap(gql), is_list(gql)
        if lst:
            return "vec![]"
        if "Connection" in base:
            return conn_default(base)
        return "None"

    def payload_default(base):
        if base not in STRUCTS:
            return None
        parts = []
        for fname, ha, _ in struct_fields(base):
            if ha:
                M["reports"].append(f"payload {base}.{fname}: method skipped in ctor")
                continue
            rname = snake(fname)
            leg = legacy_ty(base, fname)
            ft = None if leg is not None else types[base]["fields"][fname]["type"]
            parts.append(f"{rname}: {'vec![]' if (ft is not None and is_list(ft)) or (leg is not None and leg.startswith('Vec')) else 'None'}")
        return f"Some({base} {{ {', '.join(parts)} }})"

    qroots = [(n, i) for k, n, i in M["roots"] if k == "query"]
    mroots = [(n, i) for k, n, i in M["roots"] if k == "mutation"]
    W("\n#[derive(Default)]\npub struct GenQuery;\n\n#[Object]\nimpl GenQuery {")
    for name, info in qroots:
        rname = snake(name)
        args = split_args(info["args"])
        ret = root_ret(info["type"])
        W(f"\n    #[graphql(name = \"{name}\")]\n    async fn {rname}(&self{', ' + arg_sig(args) if args else ''}) -> {ret} {{")
        W(f"\n        {root_default(info['type'])}")
        W("\n    }")
    W("\n}\n")
    W("\n#[derive(Default)]\npub struct GenMutation;\n\n#[Object]\nimpl GenMutation {")
    for name, info in mroots:
        rname = snake(name)
        args = split_args(info["args"])
        base = unwrap(info["type"])
        ctor = payload_default(base)
        if ctor is None:
            M["reports"].append(f"mutation {name}: payload {base} kept/union, returns None")
            default = "None"
        else:
            default = ctor
        W(f"\n    #[graphql(name = \"{name}\")]\n    async fn {rname}(&self{', ' + arg_sig(args) if args else ''}) -> Option<{base if base not in KEPT_PATHS else KEPT_PATHS[base]}> {{")
        W(f"\n        {default if ctor is not None else 'None'}")
        W("\n    }")
    W("\n}\n")

    open(OUT, "w").write("\n".join(O) + "\n")
    print(f"WROTE {OUT} ({sum(len(x) for x in O)//1024} KiB)")
    for r in sorted(set(M["reports"]))[:40]:
        print(f"  GEN-NOTE: {r}")
    return None


def field_type_of(parent, field, types, reg):
    if parent == "Query":
        return reg["query"].get(field, {}).get("type")
    if parent == "Mutation":
        return reg["mutation"].get(field, {}).get("type")
    t = types.get(parent)
    if not t:
        return None
    return t["fields"].get(field, {}).get("type")


def selected_fields(t, g, types):
    if t not in types:
        return {}
    return {f: g.usage.get(t, {}).get(f, False)
            for f in g.usage.get(t, {}) if f in types[t]["fields"]}


def collect_conds(sels, typename, types, reg, out, seen):
    """Map (parent, field) -> inline-conditioned types used under it."""
    if typename in ("Query", "Mutation"):
        fields = reg["query"] if typename == "Query" else reg["mutation"]
    elif typename in types:
        fields = types[typename]["fields"]
    else:
        # union/interface parent: attribute conds to (typename, '*')
        for name, sub, cond, _ in sels:
            if name == "...on":
                out.setdefault((typename, "*"), set()).add(cond)
                collect_conds(sub, cond, types, reg, out, seen)
            elif name.startswith("..."):
                f = FRAGS.get(name[3:])
                if f and f[0] != typename:
                    out.setdefault((typename, "*"), set()).add(f[0])
                if f:
                    collect_conds(f[1], f[0], types, reg, out, seen)
            elif sub:
                collect_conds(sub, unwrap(fields[name]["type"]) if name in fields else typename,
                              types, reg, out, seen)
        return
    for name, sub, cond, _ in sels:
        if name == "...on":
            out.setdefault((typename, "*"), set()).add(cond)
            collect_conds(sub, cond, types, reg, out, seen)
        elif name.startswith("..."):
            f = FRAGS.get(name[3:])
            if f:
                collect_conds(f[1], f[0], types, reg, out, seen)
        elif sub and name in fields:
            collect_conds(sub, unwrap(fields[name]["type"]), types, reg, out, seen)
            # direct inline conds one level down
            for s2, _, c2, _ in sub:
                if s2 == "...on":
                    out.setdefault((typename, name), set()).add(c2)


def find_boxes(all_gen, abstractions, deps, member_edges):
    """DFS cycle-break: return {(parent, field)} struct-field edges to Box.
    member_edges: {union_or_iface: [member structs]} for cycle traversal."""
    graph = {}
    for (p, f), tgt in deps.items():
        graph.setdefault(p, []).append((f, tgt))
    for abs_name, members in member_edges.items():
        for m in members:
            graph.setdefault(abs_name, []).append((None, m))
    boxes, visited, stack = set(), set(), []

    def dfs(node):
        visited.add(node)
        stack.append(node)
        for f, tgt in graph.get(node, []):
            if tgt not in all_gen and tgt not in abstractions:
                continue
            if tgt in stack:
                # Back edge closes a cycle. Box THIS edge when it's a struct
                # field (guaranteed break); else first struct edge on path.
                if f is not None and node in all_gen:
                    boxes.add((node, f))
                else:
                    idx = stack.index(tgt)
                    for n in stack[idx:]:
                        if n not in all_gen:
                            continue
                        for ff, tt in graph.get(n, []):
                            if ff is not None and (tt in all_gen or tt in abstractions):
                                boxes.add((n, ff))
                                break
                        else:
                            continue
                        break
            elif tgt not in visited:
                dfs(tgt)
        stack.pop()

    for n in sorted(all_gen):
        if n not in visited:
            dfs(n)
    return boxes


def main():
    ir = json.load(open(IR))
    reg = {"query": ir["query"], "mutation": ir["mutation"], "types": ir["types"],
           "inputs": ir["inputs"], "enums": ir["enums"], "unions": ir.get("unions", {})}
    global FRAGS
    ops, FRAGS = load_docs()
    g = Gen(reg)
    g.analyze(ops)
    if "--plan" in sys.argv or "--emit" in sys.argv:
        return plan(g, reg, ops, emit="--emit" in sys.argv)
    print("use --plan or --emit")


if __name__ == "__main__":
    main()
