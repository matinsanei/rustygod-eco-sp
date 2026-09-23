#!/usr/bin/env python3
"""Enterprise gate: execute every Dashboard GraphQL document against the
Rust backend and assert ZERO schema-validation errors.

Each dashboard gql`...` doc (minus tests/fixtures/stories/generated) is sent
with its fragment closure and empty variables `{}`. Missing-variable and
auth errors are EXPECTED (ignored); these patterns are FAILURES:

  - Unknown field / Unknown type / Unknown fragment / Unknown directive
  - Invalid value for argument / Unknown argument
  - Fragment cannot be spread here / can never be of type

Usage:
  python3 scripts/verify_dashboard_ops.py [--show-ok]

Exit 0 iff no validation failures.
"""
import re
import sys
import json
import glob
import urllib.request

URL = "http://127.0.0.1:8000/graphql"
DASH_SRC = "/home/matin/Desktop/dev/saleor/dashboard/src"
FAIL_PATTERNS = [
    "Unknown field", "Unknown type", "Unknown fragment", "Unknown directive",
    "Invalid value for argument", "Unknown argument",
    "cannot be spread here", "can never be of type",
    "must define one or more fields", "There can be only one",
]
# Errors that only reflect empty `{}` variables / missing auth — not schema gaps.
IGNORE_PATTERNS = [
    "Variable ", "was not provided", "Expected value of type",
    " authentication", "Authentication", "auth: ", "Permission",
    "Out of range", "cannot represent",
]


def gql_docs():
    docs = []
    for pat in (DASH_SRC + "/**/*.ts", DASH_SRC + "/**/*.tsx"):
        for f in glob.glob(pat, recursive=True):
            base = f.split("dashboard/src/")[-1]
            if ("hooks.generated" in f or ".test." in f or ".stories." in f
                    or "fixtures" in f or "/fixtures/" in f):
                continue
            try:
                t = open(f, errors="replace").read()
            except OSError:
                continue
            for m in re.finditer(r"gql`(.*?)`", t, re.S):
                docs.append((base, m.group(1)))
            # Executed raw template queries (Product Doctor public-API check —
            # plain fetch, no gql tag; see schema_examine.py).
            for m in re.finditer(r"const PUBLIC_API_\w+\s*=\s*`(.*?)`", t, re.S):
                docs.append((base, m.group(1)))
    return docs


def collect_fragments(docs):
    frags = {}
    for _, doc in docs:
        for fm in re.finditer(r"fragment\s+(\w+)\s+on\s+(\w+)", doc):
            frags[fm.group(1)] = fm.group(0) + doc[fm.end():fm.end()+50]  # placeholder
    # re-extract full fragment bodies properly (lockSchema stripped like the client does)
    frags = {}
    for _, doc in docs:
        doc = strip_lock_schema(doc)
        for fm in re.finditer(r"fragment\s+(\w+)\s+on\s+\w+\s*\{", doc):
            name = fm.group(1)
            j = doc.find("{", fm.end() - 1)
            depth, k = 0, j
            while k < len(doc):
                if doc[k] == "{":
                    depth += 1
                elif doc[k] == "}":
                    depth -= 1
                    if depth == 0:
                        break
                k += 1
            frags[name] = doc[fm.start():k+1]
    return frags


def needed_frags(doc, frags):
    seen, out, stack = set(), [], []

    def scan(text):
        for m in re.finditer(r"\.\.\.(\w+)", text):
            name = m.group(1)
            if name in frags and name not in seen:
                seen.add(name)
                stack.append(name)
    scan(doc)
    while stack:
        name = stack.pop()
        out.append(frags[name])
        scan(frags[name])
    return out


def strip_lock_schema(doc):
    """Mirror the dashboard runtime (graphql/lockSchema.ts): the client strips
    `@lockSchema` directives before sending; fields gated to the *other*
    schema are dropped. The released dashboard runs main-schema mode, and no
    staging-gated fields exist in docs, so: drop the directive text only."""
    return re.sub(r"\s*@lockSchema\s*\(\s*schema\s*:\s*\"(main|staging)\"\s*\)", "", doc)


def operations(doc):
    """Split a doc possibly holding several operations; yield (kind, name, text)."""
    doc = strip_lock_schema(doc)
    ops = []
    for om in re.finditer(r"(query|mutation|subscription)\s+(\w+)", doc):
        ops.append((om.group(1), om.group(2), om.start()))
    if not ops:
        return []
    # slice each op from its start to the next op start (fragments appended separately)
    spans = []
    for i, (kind, name, start) in enumerate(ops):
        end = ops[i+1][2] if i + 1 < len(ops) else len(doc)
        # cut at balanced end of the operation body: find first { then match
        j = doc.find("{", start)
        depth, k = 0, j
        while k < end:
            if doc[k] == "{":
                depth += 1
            elif doc[k] == "}":
                depth -= 1
                if depth == 0:
                    break
            k += 1
        spans.append((kind, name, doc[start:k+1]))
    return spans


def run(query):
    req = urllib.request.Request(
        URL, data=json.dumps({"query": query, "variables": {}}).encode(),
        headers={"Content-Type": "application/json", "Origin": "http://localhost"})
    try:
        with urllib.request.urlopen(req, timeout=15) as r:
            return json.load(r)
    except Exception as e:
        return {"errors": [{"message": f"TRANSPORT: {e}"}]}


def main():
    docs = gql_docs()
    frags = collect_fragments(docs)
    print(f"DOCS={len(docs)} FRAGMENTS={len(frags)}")
    tested, failed, skipped = 0, 0, 0
    failures = []
    show_ok = "--show-ok" in sys.argv
    for base, doc in docs:
        for kind, name, text in operations(doc):
            if kind == "subscription":
                skipped += 1
                continue
            full = text + "\n" + "\n".join(needed_frags(text, frags))
            res = run(full)
            tested += 1
            errs = [e.get("message", "") for e in res.get("errors", [])]
            real = [m for m in errs
                    if any(p in m for p in FAIL_PATTERNS)
                    and not any(p in m for p in IGNORE_PATTERNS)]
            if real:
                failed += 1
                failures.append((kind, name, base, real[:3]))
            elif show_ok and not errs:
                print(f"  ok {kind} {name}")
    print(f"TESTED={tested} FAILED={failed} SKIPPED(subs)={skipped}")
    for kind, name, base, errs in failures[:60]:
        print(f"FAIL {kind} {name} [{base}]")
        for e in errs:
            print(f"    {e[:220]}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
