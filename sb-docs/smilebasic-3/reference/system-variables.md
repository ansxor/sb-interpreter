---
title: System Variables
slug: docs-sb3-system-variables
system: SmileBASIC 3
type: reference
category: System Variable
source: InstructionList.pdf
scraped: 2026-06-21
---

# System Variables

System variables are variables reserved in the system and managed by SmileBASIC. Although they are primarily read-only, it is possible to populate values in some (they are writable).

| System Variable | Description |
|---|---|
| `CSRX` | Cursor position X |
| `CSRY` | Cursor position Y |
| `CSRZ` | Cursor position Z (depth) |
| `FREEMEM` | Amount of free user memory available (in KB) |
| `VERSION` | System version (&HXXYYZZZZ) |
| `TABSTEP` | TAB movement amount (writable) |
| `SYSBEEP` | System sound effects (writable, TRUE=allowed) |
| `ERRNUM` | Error number |
| `ERRLINE` | Line where an error occurred |
| `ERRPRG` | Program SLOT where an error occurred |
| `PRGSLOT` | Current program SLOT for the PRG instruction |
| `RESULT` | Dialog result (TRUE/FALSE/-1=Suspended) |
| `MAINCNT` | Number of frames since SmileBASIC was launched |
| `MICPOS` | Current sampling location |
| `MICSIZE` | Number of samples in the sampling buffer |
| `MPCOUNT` | Number of participants in a session |
| `MPHOST` | Host ID |
| `MPLOCAL` | User ID |
| `TRUE` | Always 1 |
| `FALSE` | Always 0 |
| `TIME$` | Time string (HH:MM:SS) |
| `DATE$` | Date string (YYYY/MM/DD) |
| `HARDWARE` | Hardware information (1=new3DS) |
| `CALLIDX` | Number called by SPFUNC and BGFUNC |
