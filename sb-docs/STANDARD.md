# sb-docs conversion standard

Every page in `sb-docs/` is scraped from <https://smilebasicsource.com/documentation>
and converted from SmileBASIC Source's in-house markup ("12y" / "12y2") into this one
standard Markdown shape. This file is the contract: every doc file follows it exactly.

## Source of truth

- Index: <https://smilebasicsource.com/documentation>
- Each command/guide is a forum thread: `…/forum/thread/docs-<system>-<name>`
- Page content lives server-rendered in `<div class="content" data-markup="…">` as raw
  "12y" or "12y2" markup (a Markdown-ish dialect). We convert that raw markup, not the
  rendered HTML.

## File layout

```
sb-docs/
  STANDARD.md                  ← this file
  README.md                    ← generated index, grouped by system
  petit-computer/<name>.md     ← slug docs-ptc-<name>   (Petit Computer / PTC, original)
  smilebasic-4/<name>.md       ← slug docs-sb4-<name>   (SmileBASIC 4)
  smilebasic-3/<name>.md       ← slug docs-sb3-<name>   (SmileBASIC 3 / 3DS)
  smilebasic-3/reference/<n>.md ← SB3 reference tables (constants, errors, …)
  smilebasic-3/README.md       ← SB3 index, grouped by manual category
```

- For the web-scraped systems, `<name>` is the slug with the `docs-ptc-` / `docs-sb4-`
  prefix stripped (`docs-sb4-spset` → `smilebasic-4/spset.md`).
- One source page = one `.md` file. Filenames are lowercase, hyphenated.

## Frontmatter (YAML, required, exact key order)

```yaml
---
title: SPSET                                              # display name (from <h1>)
slug: docs-sb4-spset                                      # full source slug
system: SmileBASIC 4                                      # "SmileBASIC 4" | "Petit Computer"
type: command                                             # command | guide | reference
source: https://smilebasicsource.com/forum/thread/docs-sb4-spset
content_id: 19530                                         # SBS numeric thread id
created: 2020-11-30                                       # thread creation date (YYYY-MM-DD)
scraped: 2026-06-21                                       # date this file was generated
---
```

- `system`: `Petit Computer` for `ptc` slugs, `SmileBASIC 4` for `sb4` slugs,
  `SmileBASIC 3` for `sb3` slugs.
- `type`: `guide` if the slug ends in `-guide`; `reference` for the catalog pages
  (`keywords`, `operators`, `operator`, `constants`, `function`, `function-list`,
  `environment-support-table`); otherwise `command`.
- The frontmatter block is pre-computed and handed to the converter verbatim — emit it
  unchanged.

## SmileBASIC 3 (source: `InstructionList.pdf`)

SmileBASIC 3 (the 3DS release) is **not** on smilebasicsource.com — it comes from the
official English `InstructionList.pdf` (40-page landscape, a 3-column ruled table:
*label · key · value*). It is extracted **deterministically** from the PDF's ruled-cell
grid (not via the web-markup pipeline, and not via an LLM — a rigid table parses more
faithfully in code). Only the heterogeneous reference sections (MML, errors, system
variables, constants) are formatted by an LLM pass.

SB3 frontmatter adds `category` (the manual's section, e.g. `Sprites`) and `forms` (how
many syntax forms the instruction has), and uses `source: InstructionList.pdf`:

```yaml
---
title: SPSET
slug: docs-sb3-spset
system: SmileBASIC 3
type: command            # command | reference
category: Sprites
source: InstructionList.pdf
forms: 6
scraped: 2026-06-21
---
```

Body shape:
- `# <NAME>` then a `> **Category:** …` line.
- One syntax **form** per `(N)` in the manual. A single-form instruction puts its fields
  directly at `##`; a multi-form instruction gets a `## <NAME> (N)` per form with fields at
  `###`.
- Per-form fields mirror the manual's labels: a description (bulleted), **Format** (syntax
  in a ```` ```sb3 ```` fence), **Arguments** (a `| Argument | Description |` table,
  multi-line cells joined with `<br>`), **Examples** (```` ```sb3 ````), and any of
  *Return Values / Supplement / Notes / See Also* (table when key→value, else prose).
- Filenames strip the `(N)` and the `$ % #` type suffixes (`MID$` → `mid.md`); forms are
  merged into the one file.

### SmileBASIC 3 guide (source: `e-manual.pdf`)

The *Handy Instruction Manual* (`e-manual.pdf`) is the tutorial/concept companion to the
instruction list — prose + screenshots, organized as 26 numbered topics (9–35, no 31).
These live in `smilebasic-3/manual/` as `type: guide` pages (`slug: docs-sb3-manual-<slug>`,
with a `topic:` number). They are extracted deterministically (topic = the run of PDF text
blocks under each size-12 numbered header; `9.4`pt blocks → `##` sub-headings, `8`pt → body)
and then formatted by an LLM pass that **only reflows and structures — it preserves wording
verbatim**: PDF line-breaks rejoined into paragraphs, `Action! - INSTR` runs → bullet lists,
SmileBASIC snippets → ```` ```sb3 ```` fences, instruction names → inline code. Screenshots
are dropped (we can't embed the images); all surrounding text is kept.

## Body

The first `# <TITLE>` heading comes right after the frontmatter, then the converted body.

### 12y → Markdown rules (v1, the star dialect)

| 12y source | Markdown out |
|---|---|
| `* Heading` (line start) | `## Heading` |
| `** Heading` | `### Heading` |
| `*** Heading` | `#### Heading` |
| `*word*` (inline, mid-line) | `*word*` (italic — already valid, keep) |
| `` ```sb4 ``, `` ```sb ``, `` ```sbsyntax ``, `` ```mml ``, `` ```sbfunction `` | keep the fence + language tag unchanged |
| `` `code` `` | keep unchanged |
| `> text` (line start) | `> text` (blockquote, keep) |
| `\` at end of a table-cell line | join with `<br>` |

### 12y2 → Markdown rules (v2, the `#`/`[#]` dialect)

| 12y2 source | Markdown out |
|---|---|
| `# Heading` (line start) | `## Heading` — demote one level so the file has a single `# TITLE` H1 |
| `## Heading` | `### Heading` (demote one level) |
| `- item` lists | keep as-is |
| `` ```sb ``/`` ```sbsyntax `` etc. | keep unchanged |

> Both dialects end up the same: the file's only `#` is the title; every section heading
> from the body sits at `##` or deeper.

### Tables (both dialects)

12y header row: `|* Input | Description |`  →  standard Markdown header + `---` separator.
12y2 header row: `| Input | Description |[#]`  →  strip the trailing `[#]`, emit `---` separator.

```
|* Input | Description |          | Input | Description |
| `a%` | first |          →       | --- | --- |
| `b%` | second |                 | `a%` | first |
                                  | `b%` | second |
```

- **Rowspans** `#rs=N`: 12y writes the shared text once with `#rs=N` and leaves the
  next `N-1` rows' cell empty. Markdown has no rowspan — **repeat the shared value in
  each spanned row** so the table stays valid and readable.
- **Cell line-continuation** `\`: a cell whose line ends in `\` continues on the next
  line; join the pieces with `<br>`.
- **Nested tables** `{|* … |}` inside a cell: flatten to a `<br>`-separated `key — value`
  list inside that cell. Keep the inline `` `code` `` intact.

### Links (normalize ALL internal links to absolute canonical URLs)

SBS internal-link forms and how to resolve them:

| source form | meaning |
|---|---|
| `sbs:page/19530[Text]` | numeric thread id |
| `sbs:docs/ptc-sprite[Text]` | doc slug (missing the `docs-` prefix) |
| `sbs:page/docs-ptc-function{Text}` | full slug |
| `\link[sbs:page/docs-ptc-bgfill]{Text}` | full slug |
| `\link[sbs/page:docs-ptc-background]{Text}` | full slug, `/`+`:` swapped |
| `\link[sbs:docs/sb4-tprint]{Text}` | doc slug (missing `docs-` prefix) |

Resolve to `[Text](URL)`:

- Numeric id `N` → `https://smilebasicsource.com/forum/thread/N`
- A slug → ensure it starts with `docs-` (prepend `docs-` if it begins with `ptc-`/`sb4-`),
  then → `https://smilebasicsource.com/forum/thread/docs-…`
- `Text` keeps any inline `` `code` `` it contained.

Plain external links (`https://…`) stay as normal Markdown links.

### Text hygiene

- Decode HTML entities: `&quot;`→`"`, `&gt;`→`>`, `&lt;`→`<`, `&amp;`→`&`, `&#39;`→`'`.
- Preserve all SmileBASIC code **verbatim** inside code fences — never reflow, re-case,
  or "fix" it.
- Preserve paragraph breaks. Don't invent content, headings, or commentary that isn't
  in the source. Don't drop examples, notes, or table rows.
- Trailing whitespace trimmed; file ends with a single newline.

## Goal

A reader should get the same information as the live page, in clean portable Markdown,
with every example and table intact and every internal reference followable.
