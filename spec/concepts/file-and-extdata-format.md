---
title: File & extdata format
slug: file-and-extdata-format
area: files
kind: concept
sources:
  - { type: documented, ref: "sb-docs/smilebasic-3/manual/managing-projects-files.md (projects, active-project model, rename/copy/delete)" }
  - { type: documented, ref: "sb-docs/smilebasic-3/{save,load,files,chkfile,project}.md (resource namespaces PRG0-3/GRP0-5/GRPF/TXT/DAT; FILES filters; CHKFILE; PROJECT)" }
  - { type: documented, ref: "sb-docs/smilebasic-3/{gsave,gload}.md (image array color-conversion flag: 0=32-bit logical, 1=16-bit physical)" }
  - { type: disassembled, ref: "cia_3.6.0.lst SAVE handler @0x18e7d4: argcount guard `ldr r0,[r0,#0x4]`/`cmp r0,#0x1`/else `mov r0,#0x3` (errnum 3); resource string parsed by shared parser `bl 0x001d6d6c` (result type code checked `cmp r0,#0xe`/`bls`); unknown resource `mov r0,#0x4` @0x18e898 (errnum 4); type/page range guards @0x18e8e4 `cmp r2,#0x4`/`bcc` and @0x18e900 `cmp r2,#0x6`/`bge` -> `mov r0,#0xa` (errnum 10); resource-type switch table @0x18e930 `ldrcc pc,[pc,r0,lsl #0x2]` (cases 0..6)" }
  - { type: hw_verified, ref: "sb-oracle skill sb_extdata.py: extdata SB3 container (80-byte header + UTF-8/PCBN body + 20-byte HMAC-SHA1 footer; markers TXT/DAT/GRP; HMAC key) round-tripped both directions against real SB 3.6.0 (O-T3 write valid file SB accepts + O-T4 read result off disk)" }
  - { type: hw_verified, ref: "sb-oracle skill sb_grp.py: DAT/GRP body = 28-byte PCBN header (magic 'PCBN'+'0001', u32 LE width/height @12/@16) + 512x512 RGBA5551 LE row-major pixels; pixel-exact GRP0 capture (O-T6)" }
  - { type: community, ref: "tools/extract_sbsave.py: PETC smilebasicsource.com server container (type 0=TXT/1=DAT/2=PRJ; project directory at 0x54/0x58; internal name prefix T/B) validated against 915/915 scraped downloads" }
# On-disk container + GRP body are hw_verified (round-trip); resource parsing/errnums are
# disassembled; project model is documented; DAT-array tagging is queued. Top-level = the
# lowest load-bearing tier so the whole model isn't overclaimed as hw_verified.
confidence: disassembled
related:
  - SAVE
  - LOAD
  - FILES
  - DELETE
  - RENAME
  - CHKFILE
  - PROJECT
  - GSAVE
  - GLOAD
  - BGSAVE
  - BGLOAD
  - PRGGET$
  - PRGSET
---

# File & extdata format

The contract for M6's storage layer (the `Storage` trait + extdata-compatible layout) and
for oracle interop (O-T3). There are **two distinct format layers**, and they must not be
confused:

1. **The logical resource model** — how a SmileBASIC program *names* and *typed-accesses*
   storage at runtime: `SAVE "GRP0:NAME"`, `LOAD "TXT:FOO" OUT S$`, projects, the active
   project. This is the language-visible contract (docs + disassembly).
2. **The on-disk extdata container** — the exact bytes SmileBASIC writes to the 3DS extdata
   filesystem for one file: an 80-byte header, the resource body, and a 20-byte HMAC-SHA1
   footer. This is what the oracle reads/writes (cracked + `hw_verified` round-trip).

A third, *non-SmileBASIC* container — the **PETC server file** the corpus was scraped from —
is documented here too so the corpus loader and on-disk format aren't conflated.

---

## 1. Logical resource model

Every file is addressed by a string `"[ResourceType:]Name"`. The optional `ResourceType:`
prefix selects which typed namespace the operation targets; with no prefix the operation
targets the **current program slot** (for `SAVE`/`LOAD`) or defaults to `TXT` checks.

| Prefix | Namespace | Body type | Notes |
|---|---|---|---|
| *(none)* | current program SLOT | program text | `SAVE "TEST"` saves the running slot's source |
| `PRG0:`–`PRG3:` | program slot 0–3 | program text (UTF-8) | `PRG:` == `PRG0:`. Stored on disk as a **TXT** file |
| `GRP0:`–`GRP5:` | graphic page 0–5 | PCBN image | 512×512 RGBA5551 page |
| `GRPF:` | font image page | PCBN image | the font/sprite-sheet page (page index −1 in GSAVE) |
| `TXT:` | text resource | UTF-8 string | string ↔ file (`SAVE "TXT:F",S$` / `LOAD("TXT:F")`) |
| `DAT:` | binary data resource | PCBN numeric array | numeric array ↔ file (`SAVE "DAT:F",A` / `LOAD "DAT:F",A`) |

Key consequence (from `sb_extdata.py`, `hw_verified`): **programs are TXT files**. A program
named `P` is the on-disk file `TP` and loads via `LOAD "PRG0:P"`. There is no separate
"program" container type on disk — the `PRG*`/`TXT` distinction is purely the runtime
namespace; both map to the same `TXT` on-disk marker.

### Resource-name parsing (disassembled)

The `SAVE` handler `@0x18e7d4` shows the shape every file instruction shares:

- **Argument count** is checked first (`ldr r0,[r0,#0x4]` / `cmp r0,#0x1`); a malformed call
  takes the `mov r0,#0x3` path → **errnum 3** (Syntax error).
- The resource string is handed to a **shared resource-name parser** `@0x001d6d6c`, which
  splits `"TYPE:NAME"` and returns a **resource type code** (validated `cmp r0,#0xe` / `bls`,
  i.e. ≤ 0xE). An unrecognized resource → `mov r0,#0x4` `@0x18e898` → **errnum 4** (Illegal
  function call).
- The parsed type code is then range-checked against the page/index limits for that family
  (`@0x18e8e4` `cmp r2,#0x4`/`bcc`, `@0x18e900` `cmp r2,#0x6`/`bge`) → on overflow
  `mov r0,#0xa` `@0x18e8f8` → **errnum 10** (Out of range; e.g. `GRP6`).
- A **resource-type switch** `@0x18e930` (`ldrcc pc,[pc,r0,lsl #0x2]`, cases 0..6) dispatches
  to the per-type save routine. The 0..6 span corresponds to the typed namespaces above
  (TXT/DAT/program text + the GRP pages).

### FILES filters (documented)

`FILES ["filter"][, strArray$]` lists names to the console, or fills a 1-D string array
(auto-extended). Filter strings:

| Filter | Lists |
|---|---|
| *(none)* | all files in the current project |
| `"TXT:"` | texts **and** programs |
| `"DAT:"` | binary data (**including** graphics — GRP/DAT share the DAT folder) |
| `"//"` | the project list (all projects) |
| `"PROJECT/"` | contents of the named project |

`CHKFILE("[TXT:|DAT:]Name")` → `TRUE`/`FALSE` for existence (defaults to checking the
program/text namespace when no prefix is given).

---

## 2. On-disk extdata container (the cracked SB3 file format)

Each SmileBASIC file on the 3DS extdata filesystem is exactly:

```
[ 80-byte header ] [ body ] [ 20-byte footer ]
```

**`hw_verified`** — written by `sb_extdata.py` and accepted by real SB 3.6.0 (correct HMAC),
and read back from real-SB-saved files (O-T3 / O-T4).

### Header (80 bytes = 0x50)

| Offset | Size | Field | Value |
|---|---|---|---|
| 0x00 | 8 | type marker | TXT `01 00 00 00 00 00 01 00` · DAT `01 00 01 00 00 00 00 00` · GRP `01 00 01 00 00 00 02 00` |
| 0x08 | 4 | body length | u32 LE (byte length of the body, not incl. header/footer) |
| 0x0C | 4 | save date | `DF 07 0A 0F` (the fixed date SBFM/the injector writes; SB itself stamps the real RTC) |
| 0x10 | 0x40 | reserved | zero-filled to 80 bytes |

### Body

The raw resource payload — UTF-8 text for TXT, or a PCBN blob for DAT/GRP (see §3). Length
is exactly the `0x08` field.

### Footer (20 bytes = 0x14)

```
footer = HMAC-SHA1(KEY, header || body)
KEY    = nqmby+e9S?{%U*-V]51n%^xZMk8>b{?x]&?(NmmV[,g85:%6Sqd"'U")/8u77UL2
```

The footer is an **integrity check**, not encryption — the body is plaintext. A file with a
missing or wrong footer shows as `?NAME` in `FILES` and **will not load**; any writer
(including our storage layer when targeting the oracle) must compute the HMAC. Source of the
markers/prefixes/key: `nnn1590/lpp-3ds-sbfm` (`romfs/index.lua`), confirmed by round-trip.

### On-disk naming

The on-disk filename is **`TYPE_PREFIX + in-SB name`**: `TXT → "T"`, `DAT → "B"`, `GRP → "B"`
(GRP shares the `B`/data folder, matching the `DAT:` FILES filter). So in-SB `P` (a program)
→ on-disk `TP`; in-SB graphic `PIC` → on-disk `BPIC`. Under Azahar these live in the extdata
`user/###/` directory (see §5).

---

## 3. Resource body formats

### TXT body — UTF-8 program/text

Plain UTF-8 SmileBASIC source (program slots) or arbitrary string data (`TXT:` resources).
Decoded as `body[0x50 : 0x50 + bodylen]`. SmileBASIC's private-use glyphs survive as their
UTF-8 byte sequences (the corpus flags those entries `encoding:"sb-bytes"`).

### DAT / GRP body — PCBN binary

Graphics pages and numeric-array data share a binary container ("PCBN"). For a **GRP image**
(`hw_verified`, pixel-exact, O-T6):

| Offset (in body) | Size | Field |
|---|---|---|
| 0x00 | 4 | magic `"PCBN"` |
| 0x04 | 4 | version `"0001"` |
| 0x08 | 4 | type/flags (u16 `0x0003`, u16 `0x0002`) — not needed to decode |
| 0x0C | 4 | width  (u32 LE) — 512 for a GRP page |
| 0x10 | 4 | height (u32 LE) — 512 for a GRP page |
| 0x14 | 8 | checksum/date-ish + zero — ignored |
| 0x1C | w·h·2 | pixels: 16-bit **RGBA5551** LE, row-major, top-left origin |

RGBA5551 bit layout (MSB→LSB): `R:5 G:5 B:5 A:1` — alpha is **bit 0** (1 = opaque,
0 = transparent). 5→8-bit channel expansion is `v<<3` (matches the logical-color constants,
e.g. `#WHITE = &HFFF8F8F8`). **A GRP page always saves the full 512×512 buffer**, independent
of `XSCREEN` mode and of the visible 400×240 / 320×240 regions.

For a **DAT numeric array**, the PCBN container holds int / double / ushort element data
(per `GSAVE`'s color-conversion flag and `osb` `project.d loadDataFile`); a raw blob with no
`PCBN` magic is treated as a raw `int32` array. The `GSAVE`/`GLOAD` **color-conversion flag**
picks how pixels are stored when round-tripping a page through an array: `0` = convert to
32-bit logical colors, `1` = leave the 16-bit physical codes as-is.

> **Queued (O-T3):** the exact DAT element-type tagging (how int vs double vs ushort arrays,
> and array dimensions, are encoded in the PCBN header for `SAVE "DAT:"`/`LOAD "DAT:"`) is
> not yet byte-verified — only the GRP image layout is pixel-exact. See `bd:sb-interpreter-c9d`.

---

## 4. Projects & the active-project model

Files live inside a **project folder**. On the device, projects mirror
`PROJECTS/<name>/{TXT,DAT}/` — `TXT/` for text/programs, `DAT/` for binary/graphics (the same
two-folder split the on-disk `T`/`B` prefixes encode). The initial/default project is named
**`DEFAULT`**.

SmileBASIC tracks **three nested "current project" conditions** (docs):

1. **start-up** project (set via *Change Active Project* in the TOP MENU),
2. **non-execution-time** project (set by the `PROJECT` instruction — DIRECT mode only),
3. **execution-time** project (set during a run, e.g. by `EXEC`).

They are nested: setting an outer one cascades to the inner ones. When execution starts (via
`RUN`, a tool, or launching a program from the file viewer), the execution-time project is
**initialized from** the non-execution-time project. `PROJECT ""` resets the default to its
initial state (`DEFAULT`); `PROJECT OUT PJ$` reads the current project name (usable from a
program). Switching the active project hides — but does **not** delete — the previous
project's files; reassigning it restores access.

**M6 implication:** the storage layer keys every file access by `(project, type, name)`. The
in-memory test impl seeds the `sbsave` corpus tree (already `PROJECTS`-style) as a ready
project. `EXEC`/multi-slot semantics (M6-T6) must honor the execution-time-vs-non-execution
project split.

---

## 5. Azahar extdata layout (oracle interop)

For the oracle, SmileBASIC's extdata lives under:

```
~/Library/Application Support/Azahar/sdmc/Nintendo 3DS/<id>/<id>/extdata/00000000/000016DE/user/###/
```

`000016DE` is SmileBASIC's extdata id; the actual files sit in the `user/###/` directory,
each named by its `TYPE_PREFIX + in-SB name` (§2). The oracle skill writes a valid container
there (`write_file`) for program injection and reads results back off disk (`read_result`,
which slices `body[0x50 : 0x50+bodylen]` and ignores the footer). This is the
`extdata-compatible layout` M6-T1's `Storage` trait must be able to import/export.

---

## 6. PETC server container (corpus only — NOT the device format)

The `harness/corpus/sbsave/` corpus was scraped as **PETC server files** from
smilebasicsource.com — a *different* container from the device extdata format above. It is
documented here only so the two are not conflated; the deterministic interpreter never reads
it directly (the extractor normalizes it to the `files/<KEY>/{TXT,DAT}/` tree). Validated
against **915/915** scraped downloads (`community`, `tools/extract_sbsave.py`):

```
[ 80-byte SB3-style header ][ payload ][ 20-byte SHA1 footer ]
  0x02 u16  type: 0=TXT (program text), 1=DAT (PCBN binary), 2=PRJ (project package)
  0x08 u32  payload size
  0x0C      created date (u16 year, u8 month/day/hour/min/sec)
  0x14      author fields (creator, then current author)
```

A **type-2 PRJ** package bundles many files: a project size at `0x50`, file count `N` at
`0x54`, then an `N × { u32 fullSize, char[16] name }` directory at `0x58`, then `N` standalone
SB3 files concatenated, then the project's own SHA1 footer. Internal names carry the same
1-char type prefix (`T`→TXT, `B`→DAT); the on-device resource name is `name[1:]`.

> The PETC footer is plain **SHA1** of header+payload (no HMAC key) — distinct from the
> device extdata footer's **HMAC-SHA1**. Don't reuse one verifier for the other.

---

## 7. Error conditions (file ops)

From `spec/reference/errors.yaml` and the `SAVE` handler body (§1):

| errnum | Name | When |
|---|---|---|
| 3 | Syntax error | malformed file instruction (bad/missing args) — `SAVE` argcount guard |
| 4 | Illegal function call | unrecognized resource type in the name string |
| 10 | Out of range | resource index past its family's limit (e.g. `GRP6`, `PRG4`) |
| 35 | Illegal file format | file body is a format SmileBASIC can't read |
| 46 | Load failed | the file could not be read (missing / unreadable / bad footer) |

(`SAVE`/`LOAD` also pop a confirmation dialog; `SAVE`'s dialog cannot be suppressed, `LOAD`'s
can via the trailing `,FALSE` flag — these are M6 behaviors, not format facts.)

---

## Open questions (queued for the oracle — see `bd:sb-interpreter-c9d`)

- **DAT numeric-array PCBN tagging** — exact element-type (int/double/ushort) and dimension
  encoding for `SAVE "DAT:"`/`LOAD "DAT:"`; only GRP image layout is pixel-verified.
- **GRPF page** — whether the font page is the same 512×512 PCBN layout as GRP0-5 (assumed) or
  a distinct size.
- **Header date semantics** — what SB stamps at `0x0C` on a real save (the injector uses a
  fixed constant); whether SB validates it on load.
- **errnum 35 vs 46** — which file-corruption modes raise 35 (illegal format) vs 46 (load
  failed) on real hardware.
</content>
</invoke>
