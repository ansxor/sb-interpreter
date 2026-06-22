# S — Spec build-out (PRD)

> Status: 🔥 active · Depends on: M0; uses O (oracle) · Read `prd/README.md` first. Tasks: `PRD.md` (S-T*).

## Context / why

The spec is the **contract** the whole interpreter is verified against. The first attempt
auto-generated specs from `sb-docs` alone — documentation re-rendered as YAML, with no
typed signatures, no error conditions, and no test cases. Those were **deleted** because
they'd anchor everything on a thin, single-source artifact. This milestone rebuilds the
spec suite **for real**, from **all four sources**, so "faithful" is grounded in evidence.

This is the priority milestone. Interpreter implementation (M1–M7) is gated on the
relevant category's specs existing.

## The four sources (every spec draws on all that apply)

1. **Docs** (`sb-docs/smilebasic-3/`) — syntax, args, ranges, prose semantics → `documented`.
2. **Disassembly** (`sb-disassembly/listings/cia_3.6.0.lst`) — exact numeric/string/edge
   behavior; names are UTF-16, runtime = file offset + `0x100000` → `disassembled`.
3. **osb** (`osb/SMILEBASIC/*.d`, 3.5.0) — behavioral **cross-check only**, never authoritative.
4. **Oracle** (real SB 3.6.0 in Azahar, see `prd/oracle.md`) — ground truth → `hw_verified`.

## The contract (spec schema v2)

This is the shape every `spec/instructions/<id>.yaml` must reach (the original target):

```yaml
id: FLOOR
kind: function                 # statement | function | operator | system_var
category: Mathematics
version: { introduced: "3.0.0", verified_on: "3.6.0" }
signatures:                    # list models overloads
  - args:    [{ name: value, type: number, range: "any" }]   # number = int|double
    returns: { type: integer }
summary: "Largest integer not greater than value (rounds toward -inf)."
semantics:
  - "Integer input returns unchanged (no coercion error)."
  - "Result type is always Integer, even for Double input."
  - "FLOOR(-2.1) == -3."
errors:
  - { errnum: 8, name: TypeMismatch, when: "value is a string" }   # official table = 8
sources:
  - { type: documented,   ref: "sb-docs/smilebasic-3/floor.md" }
  - { type: disassembled, ref: "cia_3.6.0.lst FLOOR handler @0x...", confidence: hypothesis }
  - { type: hw_verified,  ref: "oracle run <date>" }
confidence: hw_verified        # documented < community < observed < disassembled < hw_verified
tests:
  - { name: positive_fraction, code: "PRINT FLOOR(3.7)",  expect: { stdout: "3" } }
  - { name: negative_rounds_down, code: "PRINT FLOOR(-2.1)", expect: { stdout: "-3" } }
  - { name: integer_passthrough, code: "PRINT FLOOR(5)",  expect: { stdout: "5" } }
  - { name: string_errors, code: 'A=FLOOR("x")', expect: { error: { errnum: 8 } } }
```

`tests` may stay inline OR live in the `spec/tests/<id>.yaml` overlay (oracle harvest
writes there). Either way the conformance suite runs them against `sb-core`.

## Authoring process (per instruction)

1. Read the doc page; draft `id/kind/category/signatures/summary/semantics`.
2. Find the handler via the **sb-disasm skill** — `disasm.py dispatch <NAME>` pins the handler
   address authoritatively (operators/special forms: `handler`/`find`+`xref` fallback). You MUST
   then `show`/`showmany` the BODY and confirm exact behavior, ranges, rounding, errors. A
   `type: disassembled` source must quote real body detail (errnum site / range guard / constant
   / ≥2 addresses) — an address + prose is not `disassembled`, it's `hypothesis`, and the spec
   gate (`cargo test -p sb-spec`) rejects a faked one.
3. Cross-check against osb's implementation; note any 3.5.0-vs-3.6.0 divergence.
4. Write **test cases**: at least normal + boundary + error per signature, with `expect`.
5. **Verify expects against the oracle** (harvest) → set those to `hw_verified`. If the
   oracle isn't available in this run, set `documented`/`disassembled` and append the case
   to `HARVEST_QUEUE.md` for a later harvest pass.
6. Set the top-level `confidence` to the **lowest** of the spec's load-bearing claims.

**Mine the sbsave corpus.** `harness/corpus/sbsave/files/*/TXT/` holds 3,329 real programs.
`grep -rl '\bFLOOR\b' harness/corpus/sbsave/files` surfaces real usages — common argument
patterns, edge cases, and idioms the docs never show (e.g. `MODULE`, `VAR X#=0`). Turn
surprising ones into test cases. sbsave is test **input**, not ground truth — verify expects
via the oracle. These programs are also end-to-end inputs for the final interpreter (run on
sb-core + the oracle and diff). See `harness/corpus/sbsave/README.md`.

### Loop vs. oracle division
- The loop authors specs from docs + disassembly + osb. **If Azahar is running**, it harvests
  `expect:` values directly via the **`sb-oracle` skill** (`run_case.py prog '<expr>'`) and
  sets `hw_verified`, committing them as frozen fixtures.
- If the oracle isn't up (or a case needs framebuffer/audio — not yet in the skill), it sets
  `documented`/`disassembled` and queues the case in `HARVEST_QUEUE.md` for a later pass.
- Either way the deterministic gate (`cargo test`) replays committed fixtures — never the
  emulator. See `prd/oracle.md`.

## Concept specs (architecture/models — the other kind)

Cross-cutting models with no call shape go in `spec/concepts/<slug>.md` (Markdown +
frontmatter; see `spec/concepts/README.md`), **not** the instruction schema. They carry
`sources` + `confidence` and are the contract for their implementation milestone:

- `execution-model` — lexer/parser/compiler/VM, 4 slots + COMMON, frame layout → M1
- `screen-and-color-model` — layers, Z-order, RGBA5551 → M2  *(exemplar written)*
- `sprite-bg-model` — attributes, animation, collision, tilemaps → M3
- `frame-and-timing-model` — VSYNC/WAIT/MAINCNT, 60 fps → M4
- `mml-grammar` — the full MML language → M5
- `file-and-extdata-format` — projects, resources, extdata layout → M6
- `error-model` — errnum/ERRLINE, halt/CONT semantics

## Tasks (sliced — the canonical slice list lives in `PRD.md`)

The 13 instruction categories (counts from `sb-docs/smilebasic-3/README.md`) are split into
**slices of ≤6 instructions** (`S-T1a`, `S-T1b`, …) in `PRD.md` — that's the single source of
truth for what's left. Slicing keeps each Ralph run a finishable unit: author every instruction
in the slice to the v2 contract (typed sigs, semantics, errors, cases), then harvest. A slice
is done = every instruction specced + cases present + oracle-verifiable cases harvested or queued.

**Persist before harvest.** Write the full spec from docs + disassembly + osb first (confidence
`disassembled`) — that's commit-able on its own. THEN harvest `expect:` via the oracle into an
OUTFILE (`run_case.py batch cases.txt out.tsv` — incremental + resumable) and raise the
confirmed sources to `hw_verified`. The oracle is slow and a run can be cut off; never gate the
written spec behind it. Anything unharvested stays `disassembled` + queued in `HARVEST_QUEUE.md`.

- **S-T0 — Spec schema v2 + authoring guide.** *(done)* `spec/SCHEMA.md` at the contract above;
  `sb-spec` serde structs (typed `signatures`, `errors`, ranges) + loader/coverage; the `FLOOR`
  exemplar as the reference.
- **S-T1…S-T13** — the instruction categories, sliced in `PRD.md`. Work them in order so
  implementation (M1–M7) can start on a category as soon as its slices land. Graphics (S-T7),
  Sound (S-T10), and any visual/audio output stay `disassembled` until O-T6/O-T7 land — spec
  them from docs + disassembly now, queue the pixel/audio expects.
- **S-T14 — Verify reference tables** *(sliced a/b/c)* — cross-check `spec/reference/{errors,
  sysvars,constants}.yaml` against the disassembly (error strings @≈0x1E965C; constant table)
  and the oracle; raise confidence. (Kept from M0 but doc-derived.)

## Acceptance (per category task)
- Every instruction in the category has a `spec/instructions/<id>.yaml` at the v2 contract.
- Typed signatures (with ranges/defaults), semantics, and error conditions present.
- ≥1 test case per signature covering normal + boundary + error, each with `expect`.
- `confidence` set honestly per source; oracle-verifiable cases harvested or in `HARVEST_QUEUE.md`.
- `cargo test -p sb-spec` loads them; `sb-spec-coverage` reflects the new confidence mix.

## Verification
```bash
cargo test -p sb-spec
cargo run -p sb-spec --bin sb-spec-coverage     # confidence distribution
# supervised harvest (oracle up): python3 harness/harvest/harvest.py
```
