# spec/concepts/ — architecture & behavior specs (the other kind of spec)

Not everything is a per-instruction function. Cross-cutting **models** — the screen/layer
compositing, the VM/slot execution model, the MML grammar, the file/extdata format, the
frame/timing model, the error model — don't fit the `id/signatures` instruction contract.
They live here as **concept specs**: prose-first **Markdown** with light YAML frontmatter.

Concept specs still carry `sources` + `confidence`, so the same ladder
(`documented < community < observed < disassembled < hw_verified`) applies to a *model*,
not just a function. They cross-reference the instruction specs they govern.

## Format

```markdown
---
title: Screen & color model
slug: screen-and-color-model
area: graphics            # execution | graphics | sprites-bg | audio | input | files | system
kind: concept            # marks this as a concept spec (vs an instruction spec)
sources:
  - { type: documented, ref: "sb-docs/smilebasic-3/manual/screen-layout.md" }
  - { type: disassembled, ref: "..." , confidence: hypothesis }
confidence: documented
related: [GPAGE, XSCREEN, DISPLAY, SPSET, BGSCREEN]   # instruction ids this model governs
---

# Screen & color model

(prose, tables, diagrams, exact numbers — whatever the model needs)
```

## Conventions
- One model per file; `slug` matches the filename.
- Put **exact, verifiable facts** (dimensions, bit layouts, Z ranges, byte offsets) in
  tables and cite the source. Inferences not yet confirmed → mark the source
  `confidence: hypothesis` and queue confirmation in beads (bd search "oracle").
- These are documentation-grade (not executed by `sb-spec`), but they are the *contract*
  for the corresponding implementation milestone, exactly like instruction specs.

See `prd/specs.md` for how concept specs fit the build-out, and `_TEMPLATE.md` to start one.
