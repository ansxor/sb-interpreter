# Spec schema

The `spec/` tree is the **single source of truth** for SmileBASIC 3.6.0 behavior, built
from **all four sources** (docs + disassembly + osb cross-check + oracle). The full
contract, the four-source authoring process, and the per-category task breakdown live in
**`prd/specs.md`** — read that to author specs. This file is the quick reference.

> The earlier doc-only generated specs were deleted (they were single-source). There is no
> auto-generator; specs are authored from all sources. `instructions/` is being rebuilt
> (milestone S).

## Layout

```
spec/
  instructions/<stem>.yaml   # one per instruction — the v2 contract (see prd/specs.md)
  tests/<stem>.yaml          # optional test overlay (oracle harvest writes here)
  reference/
    errors.yaml              # ERRNUM table (3..47)   — to be cross-checked vs oracle (S-T14)
    sysvars.yaml             # system variables        — "
    constants.yaml           # built-in #constants     — "
```

## Confidence ladder (load-bearing)

```
documented  <  community  <  observed  <  disassembled  <  hw_verified
```

A spec's top-level `confidence` is the **lowest** of its load-bearing claims. The
autonomous loop may reach `documented`/`disassembled`; only the **oracle harvest**
(`prd/oracle.md`) sets `hw_verified`. Behaviors that need the oracle but can't be verified
in a given run go to `HARVEST_QUEUE.md`.

## The contract (v2)

See `prd/specs.md` for the annotated `FLOOR` exemplar. Every `instructions/<id>.yaml` has:
`id`, `kind`, `category`, `version`, **typed `signatures`** (arg types/ranges/defaults +
returns), `summary`, `semantics`, **`errors`** (errnum + condition), `sources` (per-source
refs with confidence), top-level `confidence`, and **`tests`** (code → `expect`).

`tests` may be inline or in `spec/tests/<stem>.yaml` (merged by `sb-spec`); either way the
conformance suite runs them against `sb-core`.

## Tooling

```bash
cargo test -p sb-spec                          # all specs load + are well-formed
cargo run -p sb-spec --bin sb-spec-coverage    # confidence distribution
```
