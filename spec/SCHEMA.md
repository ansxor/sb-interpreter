# Spec schema

The `spec/` tree is the **single source of truth** for SmileBASIC 3.6.0 behavior. It
drives the implementation checklist, the deterministic conformance suite, and the
coverage dashboard. Loaded + validated by the `sb-spec` crate.

## Layout

```
spec/
  instructions/<stem>.yaml   # one per instruction — the DOCUMENTED layer (auto-generated)
  tests/<stem>.yaml          # hand-authored / oracle-harvested test overlay (NOT generated)
  reference/
    errors.yaml              # ERRNUM table (3..47)
    sysvars.yaml             # system variables
    constants.yaml           # built-in #constants with exact bit values
```

`<stem>` matches the source doc filename (e.g. `floor.md` → `floor.yaml`; type suffixes
`$`/`*` are stripped, same as the docs).

## Two-layer design (why instructions/ and tests/ are separate)

- `instructions/*.yaml` is **auto-generated** from `sb-docs/` by `tools/gen_specs.py`.
  Re-run that script anytime; it owns the documented fields. **Do not hand-edit.**
- `tests/*.yaml` is **never generated**. It holds verified conformance tests and the
  `expect:` values harvested from real SmileBASIC by the oracle (`harness/harvest/`).
  Keeping it separate means regenerating the documented layer can never clobber ground
  truth. The loader merges the overlay's `tests:` into the instruction spec.

## Confidence ladder

```
documented  <  community  <  observed  <  disassembled  <  hw_verified
```

Every spec starts at `documented` (from the official docs). It rises as we reverse-
engineer the disassembly (`disassembled`) and confirm against the emulator/hardware
oracle (`hw_verified`). The coverage report counts specs per level so "faithful" is a
number, not a vibe.

## instructions/<stem>.yaml

```yaml
id: "FLOOR"               # canonical name (keeps $ / * suffixes)
kind: function            # statement | function | operator | system_var
category: "Mathematics"
system: "SmileBASIC 3"
summary: "..."            # one-line description
semantics:                # behavioral bullet points (de-duplicated across forms)
  - "..."
forms:                    # one entry per syntax form (overloads)
  - format: "Variable = FLOOR( Numerical value )"
    description: "..."
    args:
      - { name: "Numerical value", desc: "Source numerical value" }
    returns:              # for OUT params / function results (optional)
      - { name: "IX", desc: "..." }
    examples:
      - "A=FLOOR(12.345)"
see_also: "ROUND: ..., CEIL: ..."   # optional
sources:
  - { type: documented, ref: "sb-docs/smilebasic-3/floor.md" }
confidence: documented
```

As confidence rises, add richer `sources` entries (with per-source `confidence:`) and
tighten `forms[].args` with types/ranges from the disassembly. Typed signatures and
per-arg `type`/`range` are added during M1 RE work, not by the generator.

## tests/<stem>.yaml

```yaml
tests:
  - name: positive_fraction
    code: 'PRINT FLOOR(3.7)'
    expect: { stdout: "3" }
  - name: string_errors
    code: 'A=FLOOR("x")'
    expect: { error: { errnum: 8 } }   # Type mismatch (official table) — NOT 20
```

`expect` supports `stdout` (exact console text) and `error: { errnum }`. Graphics/audio
expectations reference committed golden PNG/WAV fixtures (added in M2/M5). The
conformance suite runs each `code` through `sb-core` and asserts `expect`; the oracle
harvest fills `expect` from real SmileBASIC and flips the relevant source to
`hw_verified`.

## Regenerating

```bash
python3 tools/gen_specs.py        # rewrites spec/instructions/ + spec/reference/
cargo test -p sb-spec             # validates the whole corpus loads
cargo run -p sb-spec --bin sb-spec-coverage
```
