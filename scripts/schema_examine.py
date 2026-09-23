#!/usr/bin/env python3
"""Systematic examination of Saleor's schema.graphql vs Dashboard needs.

Saleor's schema.graphql is uniform codegen output, so plain-text parsing
(regex + brace matching, no GraphQL lib needed) is exact:

  1. Registry: every `type Query/Mutation` root (name + args), every
     `type/input/enum/scalar/union/interface` with fields + type refs.
  2. Dashboard usage: all gql`...` docs in dashboard/src (except generated
     hooks) -> operations (root field) + per-type selected fields
     (fragment spreads resolved globally).
  3. Gap report: dashboard roots missing from OUR backend (read from
     crates/graphql query/mutation method names), grouped by dashboard page.

Usage:
  python3 scripts/schema_examine.py [--json-out /tmp/schema_ir.json]

Only READS the Saleor/dashboard repos + our crates/graphql sources.
"""
import json
import re
import sys
import glob
import os

SALEOR_SCHEMA = os.environ.get(
    "SALEOR_SCHEMA",
    # The dashboard's own vendored schema (schema-main.graphql, main-schema
    # mode = what the released container runs) — NOT saleor-core's 3.24
    # schema.graphql, which already dropped fields the dashboard selects
    # (filterableInStorefront et al). Codegen input must be the exact schema
    # the dashboard's hooks were generated against: zero version-skew class.
    "/home/matin/Desktop/dev/saleor/dashboard/schema-main.graphql",
)
DASH_SRC = "/home/matin/Desktop/dev/saleor/dashboard/src"
OUR_GQL = "/home/matin/Desktop/dev/rustygod-saleor/crates/graphql/src"


def split_top_level(body):
    """Split a type body into top-level entries (field defs), respecting
    parens/braces/brackets and description strings."""
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


def parse_field(entry):
    """'name(arg: Type): RetType @dir' -> (name, args_str, ret_type)."""
    m = re.match(r"([A-Za-z_][A-Za-z0-9_]*)\s*(\(.*?\))?\s*:\s*([A-Za-z_\[\]!][A-Za-z0-9_\[\]!]*)", entry, re.S)
    if not m:
        return None
    return m.group(1), (m.group(2) or ""), m.group(3)


def parse_schema(path):
    src = open(path).read()
    reg = {"query": {}, "mutation": {}, "types": {}, "inputs": {}, "enums": {}, "unions": {}, "scalars": set()}
    # top-level definitions: kind + name + body up to matching closing brace
    for m in re.finditer(r"^(type|input|enum|scalar|union|interface)(?:\s+extend)?\s+([A-Za-z_][A-Za-z0-9_]*)", src, re.M):
        kind, name = m.group(1), m.group(2)
        if kind == "scalar":
            reg["scalars"].add(name); continue
        if kind == "union":
            line = src[m.end():src.find("\n", m.end())]
            reg["unions"][name] = re.findall(r"[A-Za-z_][A-Za-z0-9_]*", line.split("=", 1)[-1])
            continue
        # body = first {...} block after the definition header line
        # (headers carry directives like @doc(...) — never skip on parens).
        # Braces inside descriptions/strings must not count.
        j = src.find("{", m.end())
        if j < 0:
            continue
        depth, k, instr3, instr1 = 0, j, False, False
        while k < len(src):
            c = src[k]
            if not instr1 and src.startswith('"""', k):
                instr3 = not instr3
                k += 3
                continue
            if not instr3 and c == '"' and not instr1:
                # skip single-line string
                k += 1
                while k < len(src) and src[k] != '"' and src[k] != "\n":
                    k += 2 if src[k] == "\\" else 1
                k += 1
                continue
            if not instr3 and not instr1:
                if c == "{":
                    depth += 1
                elif c == "}":
                    depth -= 1
                    if depth == 0:
                        break
            k += 1
        body = src[j+1:k]
        if kind == "enum":
            clean = re.sub(r'""".*?"""', "", body, flags=re.S)
            vals = list(dict.fromkeys(
                v for v in re.findall(r"^\s*([A-Z][A-Z0-9_]*)\b", clean, re.M)))
            reg["enums"][name] = vals; continue
        if kind == "union":
            reg["unions"][name] = re.findall(r"[A-Za-z_][A-Za-z0-9_]*", body); continue
        fields = {}
        for entry in split_top_level(body):
            pf = parse_field(entry)
            if pf:
                fields[pf[0]] = {"args": pf[1], "type": pf[2]}
        if name == "Query":
            reg["query"] = fields
        elif name == "Mutation":
            reg["mutation"] = fields
        elif kind == "input":
            reg["inputs"][name] = fields
        else:
            reg["types"][name] = {"kind": kind, "fields": fields}
    return reg


# ---- Dashboard documents: tiny selection-set parser ----
TOKEN = re.compile(r"\.\.\.|[A-Za-z_][A-Za-z0-9_]*|[{()}:,!=$@\"\[\]]|\$[A-Za-z_][A-Za-z0-9_]*")


def tokenize(doc):
    # strip ${...} interpolations (fragment refs)
    doc = re.sub(r"\$\{[^{}]*\}", " ", doc)
    # strip strings
    doc = re.sub(r'"(?:[^"\\]|\\.)*"', '""', doc)
    return TOKEN.findall(doc)


def parse_selection(toks, pos=0):
    """Returns (selections, new_pos). selections: list of (name, sublist|None, spread_on|None)."""
    sels = []
    assert toks[pos] == "{", toks[pos:pos+3]
    pos += 1
    while pos < len(toks) and toks[pos] != "}":
        t = toks[pos]
        if t == "...":
            pos += 1
            if toks[pos] == "on":
                pos += 2  # ... on Type {
                sub, pos = parse_selection(toks, pos + 1) if toks[pos+1] == "{" else ([], pos+1)
                sels.append(("...on", sub, toks[pos-1] if False else None))
            else:
                sels.append(("..." + toks[pos], None, None)); pos += 1
        elif re.match(r"[A-Za-z_]", t):
            name = t; pos += 1
            if pos < len(toks) and toks[pos] == ":":
                pos += 2  # alias: real
                name = toks[pos-1] if False else name
                name = t  # keep alias; field identity resolved by alias anyway
                # actually next token is field name
                name = toks[pos]; pos += 1
            # skip args (...) and directives @..(..)
            while pos < len(toks) and toks[pos] in ("(", "@"):
                if toks[pos] == "@":
                    pos += 2
                else:
                    d = 1; pos += 1
                    while d:
                        if toks[pos] == "(": d += 1
                        elif toks[pos] == ")": d -= 1
                        pos += 1
            if pos < len(toks) and toks[pos] == "{":
                sub, pos = parse_selection(toks, pos)
                sels.append((name, sub, None))
            else:
                sels.append((name, None, None))
        else:
            pos += 1
    return sels, pos + 1


def parse_dashboard():
    ops, frags = [], {}
    for f in glob.glob(DASH_SRC + "/**/*.ts", recursive=True) + glob.glob(DASH_SRC + "/**/*.tsx", recursive=True):
        if "hooks.generated" in f:
            continue
        try:
            t = open(f, errors="replace").read()
        except OSError:
            continue
        for m in re.finditer(r"gql`(.*?)`", t, re.S):
            doc = m.group(1)
            for fm in re.finditer(r"fragment\s+(\w+)\s+on\s+(\w+)\s*\{", doc):
                fname, ftype = fm.groups()
                j = doc.find("{", fm.end()-1)
                toks = tokenize(doc[j:])
                try:
                    sels, _ = parse_selection(toks)
                except (AssertionError, IndexError):
                    continue
                frags[fname] = (ftype, sels)
            # NOTE: one gql block can hold MANY operations (ConditionalFilter
            # packs ~25 queries per block). Taking only the first silently
            # dropped 24 ops (warehouse/page-type slugs, pages search, ...).
            page = f.split("dashboard/src/")[-1].split("/")[0]
            for om in re.finditer(r"^\s*(query|mutation|subscription)\s+(\w+)", doc, re.M):
                kind, name = om.groups()
                j = doc.find("{", om.end())
                toks = tokenize(doc[j:])
                try:
                    sels, _ = parse_selection(toks)
                except (AssertionError, IndexError):
                    continue
                ops.append({"kind": kind, "name": name, "root": sels[0][0] if sels else None,
                            "sel": sels, "page": page})
        # Raw (untagged) template queries that ARE executed at runtime.
        # Product Doctor's public-API verification fires a plain fetch with a
        # template string (no gql tag) — the harness was blind to it and the
        # page failed on `availableForPurchaseAt` while TESTED stayed green.
        for m in re.finditer(r"const PUBLIC_API_\w+\s*=\s*`(.*?)`", t, re.S):
            doc = m.group(1)
            page = f.split("dashboard/src/")[-1].split("/")[0]
            for om in re.finditer(r"^\s*(query|mutation|subscription)\s+(\w+)", doc, re.M):
                kind, name = om.groups()
                j = doc.find("{", om.end())
                toks = tokenize(doc[j:])
                try:
                    sels, _ = parse_selection(toks)
                except (AssertionError, IndexError):
                    continue
                ops.append({"kind": kind, "name": name, "root": sels[0][0] if sels else None,
                            "sel": sels, "page": page})
    return ops, frags


def selected_fields_per_type(ops, frags, reg):
    """Resolve fragment spreads; map selected fields to parent type names.
    Root type: Query/Mutation. Nested: by schema field return type (unwrapped)."""
    usage = {}  # typename -> set(fields)

    def unwrap(t):
        return re.sub(r"[\[\]!]", "", t)

    def walk(sels, typename):
        if typename not in ("Query", "Mutation") and typename not in reg["types"]:
            return
        fields = reg["query"] if typename == "Query" else reg["mutation"] if typename == "Mutation" else reg["types"][typename]["fields"]
        for name, sub, _ in sels:
            if name == "...on":
                continue
            if name.startswith("..."):
                f = frags.get(name[3:])
                if f:
                    walk(f[1], f[0])
                continue
            if name.startswith("__"):
                continue
            usage.setdefault(typename, set()).add(name)
            if sub and name in fields:
                walk(sub, unwrap(fields[name]["type"]))

    for op in ops:
        walk(op["sel"], "Query" if op["kind"] == "query" else "Mutation" if op["kind"] == "mutation" else "Query")
    return {k: sorted(v) for k, v in usage.items()}


def our_roots():
    """Method names in our Query/Mutation impls -> GraphQL camelCase roots."""
    import re as _re
    roots = set()
    for fn in glob.glob(OUR_GQL + "/*.rs"):
        if os.path.basename(fn) == "gen.rs":
            continue  # generated roots must not mask themselves as "ours"
        src = open(fn).read()
        for m in _re.finditer(r"async fn (\w+)\s*\(", src):
            name = m.group(1)
            # snake -> camel (async-graphql default rename_all camelCase)
            parts = name.split("_")
            roots.add(parts[0] + "".join(p.title() for p in parts[1:]))
    return roots


def main():
    reg = parse_schema(SALEOR_SCHEMA)
    ops, frags = parse_dashboard()
    usage = selected_fields_per_type(ops, frags, reg)
    ours = our_roots()
    dash_roots = {o["root"] for o in ops if o["root"]}

    print(f"SCHEMA: query_roots={len(reg['query'])} mutation_roots={len(reg['mutation'])} "
          f"types={len(reg['types'])} inputs={len(reg['inputs'])} enums={len(reg['enums'])} "
          f"unions={len(reg['unions'])} scalars={sorted(reg['scalars'])}")
    print(f"DASHBOARD: ops={len(ops)} fragments={len(frags)} unique_roots={len(dash_roots)} "
          f"types_touched={len(usage)}")
    print(f"OURS: root_resolvers={len(ours)}")

    missing_q = sorted([r for r in dash_roots if r in reg["query"] and r not in ours])
    missing_m = sorted([r for r in dash_roots if r in reg["mutation"] and r not in ours])
    print(f"\nMISSING QUERY ROOTS ({len(missing_q)}): {missing_q}")
    print(f"\nMISSING MUTATION ROOTS ({len(missing_m)}): {missing_m}")

    # per-page grouping of missing roots
    by_page = {}
    for o in ops:
        if o["root"] in missing_q or o["root"] in missing_m:
            by_page.setdefault(o["page"], set()).add(f"{o['kind']} {o['name']} -> {o['root']}")
    print("\nMISSING BY DASHBOARD PAGE:")
    for page in sorted(by_page):
        print(f"  [{page}] ({len(by_page[page])})")
        for line in sorted(by_page[page])[:12]:
            print(f"    {line}")

    if "--json-out" in sys.argv:
        out = sys.argv[sys.argv.index("--json-out") + 1]
        json.dump({"query": reg["query"], "mutation": reg["mutation"],
                   "types": reg["types"],
                   "inputs": reg["inputs"], "enums": reg["enums"],
                   "unions": reg["unions"], "scalars": sorted(reg["scalars"]),
                   "usage": usage, "ours": sorted(ours),
                   "missing_query": missing_q, "missing_mutation": missing_m},
                  open(out, "w"), indent=1)
        print(f"\nIR written to {out}")
    # patterns
    conns = [t for t in reg["types"] if t.endswith("CountableConnection")]
    print(f"\nPATTERNS: *CountableConnection={len(conns)}, "
          f"types_with_errors_field={sum(1 for t in reg['types'].values() if 'errors' in t['fields'])}, "
          f"types_with_metadata={sum(1 for t in reg['types'].values() if 'metadata' in t['fields'])}")


if __name__ == "__main__":
    sys.exit(main())
