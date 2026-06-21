# spec/ — SmileBASIC 3.6.0 behavior, as data

The authoritative, machine-checkable description of every SmileBASIC 3.6.0 instruction,
built from **all four sources** (docs + disassembly + osb cross-check + oracle). See
[`SCHEMA.md`](SCHEMA.md) for the format and [`../prd/specs.md`](../prd/specs.md) for the
authoring process and confidence ladder.

- `instructions/` — one spec per instruction (v2 contract). **Being rebuilt** in milestone
  S; the earlier doc-only generated specs were deleted as single-source.
- `reference/` — `errors`, `sysvars`, `constants` tables (doc-derived; cross-checked vs
  the disassembly + oracle in S-T14).
- `tests/` — optional per-instruction test overlays (oracle harvest writes `hw_verified`
  cases here).

```bash
cargo run -p sb-spec --bin sb-spec-coverage    # confidence distribution
```
