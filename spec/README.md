# spec/ — SmileBASIC 3.6.0 behavior, as data

The authoritative, machine-checkable description of every SmileBASIC 3.6.0 instruction.
See [`SCHEMA.md`](SCHEMA.md) for the format and the confidence ladder.

- **248** instruction specs in `instructions/` (auto-generated from `sb-docs/` by
  `tools/gen_specs.py`; the documented layer).
- **Reference tables** in `reference/`: 45 error codes, 24 system variables, 79 built-in
  constants with exact bit values.
- **Test overlays** in `tests/` (hand-authored + oracle-harvested; not generated).

Loaded and validated by the `sb-spec` crate. Quick coverage check:

```bash
cargo run -p sb-spec --bin sb-spec-coverage
```
