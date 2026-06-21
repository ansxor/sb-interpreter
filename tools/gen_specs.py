#!/usr/bin/env python3
"""Generate the `documented`-layer YAML specs from the sb-docs Markdown mirror.

Input:  sb-docs/smilebasic-3/*.md            (248 instruction pages)
        sb-docs/smilebasic-3/reference/*.md  (errors, sysvars, constants tables)
Output: spec/instructions/<stem>.yaml        (one per instruction)
        spec/reference/{errors,sysvars,constants}.yaml

This emits ONLY the documented layer (confidence: documented). Verified tests and
oracle-harvested `expect:` values live in the separate `spec/tests/` overlay, which the
Rust loader merges — so re-running this generator never clobbers ground truth.

Scalars are emitted as JSON (a subset of YAML) to avoid quoting/escaping bugs.
Deterministic: stable ordering, no timestamps. Safe to re-run.
"""
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DOCS = ROOT / "sb-docs" / "smilebasic-3"
OUT_INSTR = ROOT / "spec" / "instructions"
OUT_REF = ROOT / "spec" / "reference"

OPERATORS = {"AND", "OR", "XOR", "NOT", "DIV", "MOD"}

# ---------- small markdown helpers ----------


def split_frontmatter(text):
    if not text.startswith("---"):
        return {}, text
    end = text.find("\n---", 3)
    fm_block = text[3:end].strip()
    body = text[end + 4:].lstrip("\n")
    fm = {}
    for line in fm_block.splitlines():
        if ":" in line:
            k, v = line.split(":", 1)
            fm[k.strip()] = v.strip()
    return fm, body


def split_sections(lines, level):
    """Split into (preamble_lines, [(title, body_lines), ...]) at heading `level`."""
    prefix = "#" * level + " "
    deeper = "#" * (level + 1) + " "
    pre, sections = [], []
    cur_title, cur = None, []
    for ln in lines:
        if ln.startswith(prefix) and not ln.startswith(deeper):
            if cur_title is None:
                pre = cur
            else:
                sections.append((cur_title, cur))
            cur_title = ln[len(prefix):].strip()
            cur = []
        else:
            cur.append(ln)
    if cur_title is None:
        pre = cur
    else:
        sections.append((cur_title, cur))
    return pre, sections


def code_fences(lines):
    out, in_fence, buf = [], False, []
    for ln in lines:
        if ln.strip().startswith("```"):
            if in_fence:
                out.append("\n".join(buf).strip("\n"))
                buf, in_fence = [], False
            else:
                in_fence = True
        elif in_fence:
            buf.append(ln)
    return [c for c in out if c.strip()]


def parse_first_table(lines):
    """Return (header_lowercased, [row_dict, ...]) for the first pipe-table found."""
    cells_rows = []
    for ln in lines:
        s = ln.strip()
        if s.startswith("|"):
            # Protect escaped pipes (`\|`, common in bitfield descriptions) so they
            # aren't treated as column delimiters.
            protected = s.replace("\\|", "\x00")
            cells_rows.append(
                [c.strip().replace("\x00", "|") for c in protected.strip("|").split("|")]
            )
        elif cells_rows:
            break
    if len(cells_rows) < 2:
        return [], []
    header = [h.lower() for h in cells_rows[0]]
    rows = [dict(zip(header, r)) for r in cells_rows[2:]]  # skip header + separator
    return header, rows


def unbacktick(s):
    return s.strip().strip("`").strip()


def clean_cell(s):
    return s.replace("\\|", "|").strip()


# ---------- instruction parsing ----------


def build_form(desc_lines, fields):
    prose, bullets = [], []
    for ln in desc_lines:
        s = ln.strip()
        if not s or s.startswith(("#", ">")):
            continue
        if s.startswith("- "):
            bullets.append(s[2:].strip())
        else:
            prose.append(s)
    fmts = code_fences(fields.get("format", []))
    _, arg_rows = parse_first_table(fields.get("arguments", []))
    _, ret_rows = parse_first_table(fields.get("return values", []))
    args = []
    for r in arg_rows:
        vals = list(r.values())
        name = unbacktick(vals[0]) if vals else ""
        desc = clean_cell(vals[1]) if len(vals) > 1 else ""
        if name:
            args.append({"name": name, "desc": desc})
    returns = []
    for r in ret_rows:
        vals = list(r.values())
        name = unbacktick(vals[0]) if vals else ""
        desc = clean_cell(vals[1]) if len(vals) > 1 else ""
        if name:
            returns.append({"name": name, "desc": desc})
    return {
        "format": fmts[0] if fmts else None,
        "description": " ".join(prose) if prose else None,
        "args": args,
        "returns": returns,
        "examples": code_fences(fields.get("examples", [])),
        "_bullets": bullets,
    }


def detect_kind(ident, forms):
    base = ident.upper()
    if base in OPERATORS:
        return "operator"
    for f in forms:
        fmt = f.get("format") or ""
        if re.search(r"=\s*" + re.escape(base) + r"\s*\(", fmt, re.I):
            return "function"
    return "statement"


def parse_instruction(path):
    fm, body = split_frontmatter(path.read_text(encoding="utf-8"))
    ident = fm.get("title", path.stem.upper())
    category = fm.get("category", "")
    lines = body.splitlines()

    form_re = re.compile(r"^" + re.escape(ident) + r"\s*\(\d+\)\s*$", re.I)
    pre2, level2 = split_sections(lines, 2)
    form_secs = [(t, b) for t, b in level2 if form_re.match(t)]

    see_also = None
    if form_secs:
        forms = []
        for _, b in form_secs:
            desc_lines, fields = ([], {})
            dl, subs = split_sections(b, 3)
            desc_lines = dl
            fields = {t.lower(): fb for t, fb in subs}
            forms.append(build_form(desc_lines, fields))
    else:
        fields = {t.lower(): fb for t, fb in level2}
        if "see also" in fields:
            sa = [ln.strip() for ln in fields["see also"] if ln.strip()]
            see_also = " ".join(sa) or None
        forms = [build_form(pre2, fields)]

    # Top-level summary + de-duplicated semantics across forms.
    summary = next((f["description"] for f in forms if f["description"]), None)
    semantics, seen = [], set()
    for f in forms:
        for b in f.pop("_bullets"):
            if b not in seen:
                seen.add(b)
                semantics.append(b)

    return {
        "id": ident,
        "kind": detect_kind(ident, forms),
        "category": category,
        "summary": summary,
        "semantics": semantics,
        "forms": forms,
        "see_also": see_also,
        "source_md": f"sb-docs/smilebasic-3/{path.name}",
    }


# ---------- YAML emission (scalars via json.dumps = valid YAML) ----------


def y(v):
    if v is None:
        return "null"
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, int):
        return str(v)
    return json.dumps(v, ensure_ascii=False)


def emit_instruction(spec):
    L = []
    L.append(f"# AUTO-GENERATED from {spec['source_md']} by tools/gen_specs.py — do not edit.")
    L.append("# Verified tests + oracle-harvested expects live in spec/tests/ (merged by sb-spec).")
    L.append(f"id: {y(spec['id'])}")
    L.append(f"kind: {spec['kind']}")
    L.append(f"category: {y(spec['category'])}")
    L.append('system: "SmileBASIC 3"')
    L.append(f"summary: {y(spec['summary'])}")
    if spec["semantics"]:
        L.append("semantics:")
        L += [f"  - {y(s)}" for s in spec["semantics"]]
    else:
        L.append("semantics: []")
    if spec["forms"]:
        L.append("forms:")
        for f in spec["forms"]:
            L.append(f"  - format: {y(f['format'])}")
            L.append(f"    description: {y(f['description'])}")
            if f["args"]:
                L.append("    args:")
                for a in f["args"]:
                    L.append(f"      - {{ name: {y(a['name'])}, desc: {y(a['desc'])} }}")
            else:
                L.append("    args: []")
            if f["returns"]:
                L.append("    returns:")
                for r in f["returns"]:
                    L.append(f"      - {{ name: {y(r['name'])}, desc: {y(r['desc'])} }}")
            if f["examples"]:
                L.append("    examples:")
                L += [f"      - {y(e)}" for e in f["examples"]]
    if spec["see_also"]:
        L.append(f"see_also: {y(spec['see_also'])}")
    L.append("sources:")
    L.append(f"  - {{ type: documented, ref: {y(spec['source_md'])} }}")
    L.append("confidence: documented")
    return "\n".join(L) + "\n"


# ---------- reference tables ----------


def parse_value(raw):
    raw = raw.strip().strip("`").strip()
    try:
        if raw.lower().startswith("&h"):
            return int(raw[2:], 16)
        if raw.lower().startswith("&b"):
            return int(raw[2:], 2)
        return int(raw)
    except ValueError:
        return None


def gen_constants():
    text = (DOCS / "reference" / "constants.md").read_text(encoding="utf-8")
    _, body = split_frontmatter(text)
    _, groups = split_sections(body.splitlines(), 2)
    out = ["# AUTO-GENERATED from sb-docs/.../reference/constants.md — do not edit.",
           "constants:"]
    n = 0
    for group, lines in groups:
        _, rows = parse_first_table(lines)
        for r in rows:
            vals = list(r.values())
            name = unbacktick(vals[0])
            raw = unbacktick(vals[1]) if len(vals) > 1 else ""
            bits = parse_value(raw)
            out.append(
                f"  - {{ name: {y(name)}, group: {y(group)}, raw: {y(raw)}, bits: {y(bits)} }}"
            )
            n += 1
    return "\n".join(out) + "\n", n


def gen_errors():
    text = (DOCS / "reference" / "error-table.md").read_text(encoding="utf-8")
    _, body = split_frontmatter(text)
    _, rows = parse_first_table(body.splitlines())
    out = ["# AUTO-GENERATED from sb-docs/.../reference/error-table.md — do not edit.",
           "errors:"]
    for r in rows:
        vals = list(r.values())
        num = parse_value(vals[0])
        name = clean_cell(vals[1]) if len(vals) > 1 else ""
        desc = clean_cell(vals[2]) if len(vals) > 2 else ""
        out.append(f"  - {{ num: {y(num)}, name: {y(name)}, desc: {y(desc)} }}")
    return "\n".join(out) + "\n", len(rows)


def gen_sysvars():
    text = (DOCS / "reference" / "system-variables.md").read_text(encoding="utf-8")
    _, body = split_frontmatter(text)
    _, rows = parse_first_table(body.splitlines())
    out = ["# AUTO-GENERATED from sb-docs/.../reference/system-variables.md — do not edit.",
           "system_variables:"]
    for r in rows:
        vals = list(r.values())
        name = unbacktick(vals[0])
        desc = clean_cell(vals[1]) if len(vals) > 1 else ""
        writable = "writable" in desc.lower()
        out.append(
            f"  - {{ name: {y(name)}, desc: {y(desc)}, writable: {y(writable)} }}"
        )
    return "\n".join(out) + "\n", len(rows)


# ---------- main ----------


def main():
    OUT_INSTR.mkdir(parents=True, exist_ok=True)
    OUT_REF.mkdir(parents=True, exist_ok=True)

    md_files = sorted(p for p in DOCS.glob("*.md") if p.name.lower() != "readme.md")
    ok, failed = 0, []
    for p in md_files:
        try:
            spec = parse_instruction(p)
            (OUT_INSTR / f"{p.stem}.yaml").write_text(emit_instruction(spec), encoding="utf-8")
            ok += 1
        except Exception as e:  # noqa: BLE001
            failed.append((p.name, repr(e)))

    for name, gen in (("constants", gen_constants), ("errors", gen_errors), ("sysvars", gen_sysvars)):
        content, n = gen()
        (OUT_REF / f"{name}.yaml").write_text(content, encoding="utf-8")
        print(f"  reference/{name}.yaml: {n} entries")

    print(f"instructions: {ok}/{len(md_files)} generated")
    if failed:
        print(f"FAILED ({len(failed)}):")
        for name, err in failed:
            print(f"  {name}: {err}")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
