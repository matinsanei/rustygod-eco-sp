"""Systematic check: every gen.rs input field's nullability vs Saleor schema.graphql.
Usage: python3 scripts/check_input_nullability.py  (exit 1 on mismatch)
Compares declared `field: Type` in schema.graphql against emitted Rust:
  required (trailing `!`)  -> must be non-Option (Vec<T>/T)
  optional (no trailing !) -> must be Option<..>
Skips intentional deviations (KEPT_INPUTS, slimmed AddressInput, LEGACY_FIELDS).
"""
import re, sys

SCHEMA = "/home/matin/Desktop/dev/saleor/saleor-core/saleor/graphql/schema.graphql"
GEN = "crates/graphql/src/gen.rs"
CODEGEN = "scripts/schema_codegen.py"

# input types we deliberately do NOT generate 1:1
SKIP_TYPES = set()
# (type, field) pairs with intentional deviations: (reason)
SKIP_FIELDS = {
    ("AddressInput", None),  # slimmed on purpose (notes in codegen)
}

def snake(name):
    s = re.sub(r"(.)([A-Z][a-z]+)", r"\1_\2", name)
    return re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", s).lower()

def schema_inputs():
    src = open(SCHEMA).read()
    out = {}
    for m in re.finditer(r"input (\w+)[^{]*\{(.*?)\n\}", src, re.S):
        t, body = m.group(1), m.group(2)
        fields = {}
        for line in body.splitlines():
            line = line.strip()
            mm = re.match(r"(\w+)\s*:\s*([\[\]\w!]+)", line)
            if mm:
                fields[mm.group(1)] = mm.group(2)
        out[t] = fields
    return out

def gen_inputs():
    src = open(GEN).read()
    cg = open(CODEGEN).read()
    kept = set(re.findall(r'"(\w+)":\s*"crate::', cg))  # KEPT_INPUTS keys approx
    out = {}
    for m in re.finditer(r"pub struct (\w+) \{(.*?)\n\}\n", src, re.S):
        t, body = m.group(1), m.group(2)
        if t in kept:
            continue
        fields = {}
        cur_name = None
        for line in body.splitlines():
            s = line.strip()
            mm = re.match(r'#\[graphql\(name = "(\w+)"\)\]', s)
            if mm:
                cur_name = mm.group(1)
                continue
            mm = re.match(r"pub (\w+):\s*(.+?),?$", s)
            if mm and cur_name:
                fields[cur_name] = mm.group(2).strip().rstrip(",")
                cur_name = None
        if fields:
            out[t] = fields
    return out

def main():
    schema = schema_inputs()
    gen = gen_inputs()
    bad = []
    for t, fields in sorted(gen.items()):
        if t in SKIP_TYPES or t not in schema:
            continue
        for f, rust_ty in fields.items():
            if (t, None) in SKIP_FIELDS:
                continue
            if f not in schema[t]:
                continue  # LEGACY_FIELDS / injected — declared elsewhere
            decl = schema[t][f]
            want_required = decl.endswith("!")
            got_optional = rust_ty.startswith("Option<")
            if want_required and got_optional:
                bad.append(f"{t}.{f}: Saleor {decl} REQUIRED but gen Option<{rust_ty}>")
            elif not want_required and not got_optional:
                bad.append(f"{t}.{f}: Saleor {decl} optional but gen required {rust_ty}")
    if bad:
        print(f"MISMATCHES: {len(bad)}")
        for b in bad[:60]:
            print("  " + b)
        sys.exit(1)
    print(f"OK: {sum(len(v) for v in gen.values())} input fields match Saleor nullability")

main()
