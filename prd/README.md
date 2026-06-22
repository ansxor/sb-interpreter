# PRDs — SmileBASIC 3.6.0 interpreter

Per-milestone Product Requirement Documents, written for **handoff to other agents**.
Each milestone PRD is self-contained: context, scope, a task breakdown with stable IDs
(`M1-T3`), file pointers, references, and acceptance criteria. Read **this file first**
for shared conventions, then the milestone you're assigned.

Master plan: `~/.claude/plans/i-want-to-make-bright-plum.md`.

## Milestones

| PRD | Title | Status | Depends on |
|---|---|---|---|
| [M0](M0.md) | Scaffolding & spec pipeline | ✅ done | — |
| [M1](M1.md) | Core VM + a real window | ⬜ next | M0 |
| [M2](M2.md) | Graphics (GRP + compositor) | ⬜ | M1 |
| [M3](M3.md) | Sprites & BG | ⬜ | M2 |
| [M4](M4.md) | Input & timing | ⬜ | M1 |
| [M5](M5.md) | Audio (MML) | ⬜ | M1 |
| [M6](M6.md) | Files, projects, system, faithful stubs | ⬜ | M1 |
| [M7](M7.md) | Hardening (fuzzing, hw_verified push) | ⬜ | M2–M6 |
| [oracle](oracle.md) | Emulator-oracle bring-up (spikes) | ⬜ | M0 (parallel) |

M4/M5/M6 can proceed in parallel once M1 lands. The oracle track is independent of the
interpreter and can start immediately.

## The golden rule: fidelity is measured, not assumed

Every behavior we implement is described by a YAML spec under `spec/` and carries a
**confidence** level:

```
documented  <  community  <  observed  <  disassembled  <  hw_verified
```

- `documented` — from the official docs (where all 248 specs start).
- `disassembled` — confirmed by reading the 3.6.0 binary.
- `hw_verified` — confirmed against real SmileBASIC via the emulator oracle.

When you implement or verify a behavior, **raise its confidence and add tests**. "Done"
for a behavior means: implemented in `sb-core`, covered by a deterministic test, and the
spec's confidence reflects how it was verified.

### Implementation fidelity rule (read this twice)

**The spec is the contract. `osb/` is a *structural* reference only.** We are building
SmileBASIC **3.6.0**; `osb` is a third-party **3.5.0** interpreter that "may or may not be
accurate." Use it to learn how to *shape* a lexer/parser/VM — never as the definition of
behavior.

- Do **not** translate osb line-by-line, copy its comments, or write "port of osb."
- Do **not** reproduce osb's limitations or 3.5.0-isms (e.g. osb's ASCII-only identifiers —
  SmileBASIC is Japanese and supports full-width/kana names).
- Where osb disagrees with the docs or the disassembly, **the docs/disassembly win.**
- **Every behavior task must add a spec-derived conformance test** (`spec/tests/<id>.yaml`
  and/or `harness/corpus/cases/*.yaml`) using the concrete values the docs give.
- Set `confidence` honestly: `documented`, `disassembled`, or `hw_verified` — the last ONLY
  when confirmed via the **`sb-oracle` skill** (real SB 3.6.0) and committed as a fixture.
- Can't determine a 3.6.0 edge case from docs/disassembly? Implement the documented
  behavior, add a test, and append the open question to `HARVEST_QUEUE.md` for the oracle —
  do not silently inherit it from osb.

## Reference sources (and how to use them)

| Source | Path | Use for |
|---|---|---|
| **Specs** (source of truth) | `spec/instructions/*.yaml`, `spec/reference/*.yaml` | The contract. Start here for any instruction. |
| **Official docs** | `sb-docs/smilebasic-3/` (instructions, `reference/`, `manual/`) | Prose semantics, screen/sound models, MML. |
| **Disassembly** | `sb-disassembly/listings/cia_3.6.0.lst` | Exact numeric behavior (rounding, RNG, float→string, pixel math). |
| **osb (D, 3.5.0)** | `osb/SMILEBASIC/*.d` | Design template + behavioral cross-check. **Never authoritative** where it disagrees with the docs/disassembly (it targets 3.5.0). |
| **Real programs** | `harness/corpus/sbsave/` (`INDEX.json`) | 3,329 scraped real-world programs + 2,773 resources. Test *inputs* (parser fuel, e2e runs, oracle-diff candidates) — **never** expected values. See its README. |
| **Oracle** | the **`sb-oracle` skill** (`.claude/skills/sb-oracle/`) | Drives real SB 3.6.0 in Azahar → `hw_verified`. `run_case.py prog '<expr>'` (needs Azahar running). |

### Disassembly gotchas (read before grepping)
- The binary is loaded at base **`0x00100000`**; **runtime addr = file offset + 0x100000**.
- Command/function names are **UTF-16LE**. Grep `unicode u"PRINT"`, **not** `PRINT`.
- Anchors: command-name pool ≈ `0x2ED800–0x2EFAxx`; sorted keyword lookup table
  ≈ `0x2C8E00` (12-byte records); error strings ≈ `0x1E965C–0x1E99F4`; version banner
  `0x1E9AE0`. Biggest functions (parser/VM candidates): `FUN_00199EB8`,
  `FUN_0019D508`, `FUN_001331AC`. For name→handler wiring, open
  `sb-disassembly/ghidra_project/sb-3ds.gpr` in the Ghidra GUI and type the `.rodata`
  tables as pointers so xrefs light up. See `sb-disassembly/README.md`.

### osb file map (design reference)
`parser.d` (Lexical + Parser) · `node.d` (AST) · `compiler.d` (AST→bytecode) · `VM.d`
(`Code` opcodes + `run()`) · `type.d` (Value/Array) · `builtinfunctions.d` (builtins +
`static this()` registration ≈ line 4169) · `console.d` · `graphic.d` (`Graphic2`) ·
`sprite.d` · `bg.d` · `random.d` + `tinymt32.d` · `systemvariable.d` · `error.d`.

## Coding standards (enforced by CI — `.github/workflows/ci.yml`)

- `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean.
- Builds on **native and `wasm32-unknown-unknown`**. `sb-core` stays I/O- and GUI-free
  (no `std::fs`, no threads that don't exist on wasm). Platform concerns go in the
  `sb-platform-*` crates.
- All tests **deterministic + hermetic**: fixed RNG seeds, no emulator, no network. The
  emulator oracle and fuzzer run **only** in `harness/harvest/` (offline). See
  `harness/README.md`.
- Numeric fidelity: SmileBASIC Integer = `i32`, Double = `f64`. Match overflow/rounding
  to the disassembly, not to Rust defaults. When in doubt, write the case and harvest it.

## How to pick up a task

1. Read this file + the milestone PRD + the relevant `spec/` entries.
2. Implement against the spec. Cross-check behavior with osb; confirm exact numbers in
   the disassembly.
3. Add deterministic tests. For behaviors verified against the disassembly, add a
   `sources:` entry with `confidence: disassembled` to the spec; for oracle-verified,
   add a `spec/tests/<id>.yaml` overlay (or hand it to the oracle track to harvest).
4. Run the milestone verification commands. Keep CI green.
5. Update the milestone PRD's task checkbox and the status table above.

## Glossary

- **Slot** — one of SmileBASIC's 4 program slots (+ shared `COMMON DEF`).
- **GRP** — a graphics page (drawable bitmap layer).
- **MML** — Music Macro Language (BGM source).
- **Oracle** — real SmileBASIC 3.6.0 running in Citra/Azahar, queried via RPC.
- **Harvest / Replay** — Phase A (offline, capture fixtures) / Phase B (deterministic CI).
