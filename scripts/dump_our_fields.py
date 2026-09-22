"""Dump hand-written GraphQL type fields for codegen's KEPT-coverage check.
Usage: python3 scripts/dump_our_fields.py  (writes scripts/our_fields.json)

Scans crates/graphql/src/*.rs (excluding gen.rs). Keys are GRAPHQL type
names (from struct-level `#[graphql(name = ...)]`, else the Rust name).
Values: field/method graphql names + camelCase variants of bare idents.
"""
import re, glob, json, os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC = os.path.join(ROOT, "crates", "graphql", "src")


def snake_to_camel(s):
    parts = s.split("_")
    return parts[0] + "".join(p[:1].upper() + p[1:] for p in parts[1:])


def main():
    rust_to_gql = {}
    bodies = {}  # gql name -> list of (kind, body)
    for f in glob.glob(os.path.join(SRC, "*.rs")):
        if f.endswith("gen.rs"):
            continue
        src = open(f, errors="replace").read()
        lines = src.splitlines()
        for m in re.finditer(
            r"(?:pub\s+)?struct\s+(\w+)(?:<[^>]*>)?\s*(?:\{([^}]*)\}|;)", src
        ):
            name, body = m.group(1), m.group(2) or ""
            start_line = src[: m.start()].count("\n")
            gname = name
            for pl in lines[max(0, start_line - 3) : start_line]:
                am = re.match(r'\s*#\[graphql\(name = "(\w+)"', pl)
                if am:
                    gname = am.group(1)
            rust_to_gql[name] = gname
            bodies.setdefault(gname, []).append(("struct", body))
        for m in re.finditer(r"impl\s+(\w+)\s*\{", src):
            name = m.group(1)
            start_line = src[: m.start()].count("\n")
            for pl in lines[max(0, start_line - 3) : start_line]:
                am = re.match(r'\s*#\[Object\(name = "(\w+)"', pl)
                if am:
                    rust_to_gql[name] = am.group(1)
            rest = src[m.end() :]
            end = rest.find("\n}\n")
            bodies.setdefault(name, []).append(("impl", rest[: end if end != -1 else 0]))
    out = {}
    for key, parts in bodies.items():
        gname = rust_to_gql.get(key, key)
        fields = set(out.get(gname, []))
        for kind, body in parts:
            # field attrs can sit on the same line as the field
            # (single-line structs), so scan with finditer, not per-line.
            pos = 0
            for m in re.finditer(
                r'#\[graphql\(name = "(\w+)"[^\]]*\]\s*(?:pub\s+(r#\w+|\w+)\s*:|(?:pub\s+)?async\s+fn\s+(r#\w+|\w+))'
                r"|(?:pub\s+(r#\w+|\w+)\s*:)|(?:(?:pub\s+)?async\s+fn\s+(r#\w+|\w+))",
                body,
            ):
                groups = [g[2:] if g and g.startswith("r#") else g for g in m.groups()]
                attr, pub_after_attr, fn_after_attr, bare_pub, bare_fn = groups
                if kind == "struct" and (pub_after_attr or bare_pub):
                    ident = pub_after_attr or bare_pub
                    fields.add(attr or ident)
                    if not attr:
                        fields.add(snake_to_camel(ident))
                elif kind == "impl" and (fn_after_attr or bare_fn):
                    ident = fn_after_attr or bare_fn
                    if ident.startswith("_"):
                        continue
                    fields.add(attr or ident)
                    if not attr:
                        fields.add(snake_to_camel(ident))
        out[gname] = sorted(fields)
    dest = os.path.join(ROOT, "scripts", "our_fields.json")
    json.dump(out, open(dest, "w"), indent=1, sort_keys=True)
    print(f"WROTE {dest} ({len(out)} types)")


main()
