//! Source-code-manipulation family (M6-T4): `PRGEDIT` / `PRGGET$` / `PRGSET` / `PRGINS` /
//! `PRGDEL` / `PRGNAME$` / `PRGSIZE` — runtime editing of a program SLOT's source text.
//!
//! SmileBASIC keeps four program SLOTs (0..3). The PRG* commands edit a slot's source as a
//! list of lines. An *edit target* (slot + current line) is selected by [`PRGEDIT`]; the four
//! mutators that act on the current line (`PRGGET$`/`PRGSET`/`PRGINS`/`PRGDEL`) require an
//! active target and otherwise raise errnum 38 (`Use PRGEDIT before any PRG function`). The
//! edit-target state is **session-persistent** in real SB (a shared global, [`prgset` spec]),
//! so the errnum-38 guard fires only from a *cold* state — before any PRGEDIT has run — which
//! the VM models as `prg_edit == None`.
//!
//! This module holds the pure, I/O-free pieces: the per-slot [`PrgSlot`] source model and the
//! line-splitting helpers. The command handlers live on the VM (`vm::Vm::call_prg` and
//! friends) where the slots, edit target and running-slot index are reachable, mirroring the
//! `call_files` layout (M6-T2).
//!
//! Confidence: the arg-count (errnum 4), slot/type range (errnum 10), count-0 (errnum 10) and
//! no-PRGEDIT (errnum 38) guards are `hw_verified` (sb-oracle, see the `prg*.yaml` specs). The
//! *content* behaviour — the returned line text, the line/char/free counts, and the running-
//! slot file name — is `community`/oracle-pending (no scalar golden in a warm session); the
//! slot capacity constant behind `PRGSIZE(slot,2)` is tracked in beads (bd:sb-interpreter-wir).

use crate::value::SbStr;

/// Line-feed code (`CHR$(10)`) — the line separator inside a program slot's source buffer.
pub(crate) const LF: u16 = 0x0A;

/// Per-slot program capacity in characters, the base for `PRGSIZE(slot,2)` (free characters).
/// SmileBASIC 3 slots hold a large fixed buffer; the exact value is oracle-pending (tracked in
/// beads: bd:sb-interpreter-wir). `PRGSIZE(slot,1) + PRGSIZE(slot,2)` is meant to equal this total (the
/// documented memory-usage idiom `PRGSIZE(3,1)/(PRGSIZE(3,1)+PRGSIZE(3,2))`).
pub(crate) const SLOT_CAPACITY: usize = 524_288;

/// One program SLOT's editable source: the file name last handled by LOAD/SAVE (read by
/// `PRGNAME$`) and the source lines. Each stored line has its trailing line-feed stripped —
/// `PRGGET$` returns a line without the `CHR$(10)` terminator (the disassembled handler
/// special-cases the trailing U+000A).
#[derive(Debug, Clone, Default)]
pub(crate) struct PrgSlot {
    pub name: SbStr,
    pub lines: Vec<SbStr>,
}

impl PrgSlot {
    /// Load this slot's source from a raw string, splitting into lines on LF (terminator
    /// model — a trailing LF does not add a final empty line).
    pub fn set_source(&mut self, src: &SbStr) {
        self.lines = split_terminated(src);
    }

    /// Number of source characters (`PRGSIZE` type 1): the line text plus one LF terminator
    /// per line. Oracle-pending exact model (queued).
    pub fn char_count(&self) -> usize {
        self.lines.iter().map(|l| l.len() + 1).sum()
    }

    /// Remaining free characters (`PRGSIZE` type 2): the capacity minus the used characters
    /// (saturating). Oracle-pending (queued).
    pub fn free_count(&self) -> usize {
        SLOT_CAPACITY.saturating_sub(self.char_count())
    }
}

/// Split a source buffer on LF treated as a *terminator*: `"A\nB"` and `"A\nB\n"` both yield
/// `["A","B"]`; `""` yields `[]`. Used to load a slot's source.
pub(crate) fn split_terminated(src: &SbStr) -> Vec<SbStr> {
    let mut lines = Vec::new();
    let mut cur: SbStr = Vec::new();
    for &u in src {
        if u == LF {
            lines.push(std::mem::take(&mut cur));
        } else {
            cur.push(u);
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

/// Split an inserted/substituted string on LF treated as a *separator*: N line-feeds yield
/// N+1 segments, so `""` becomes one blank line, `"a\nb"` two lines, and `CHR$(10)+"x"`
/// becomes `["", "x"]`. Used by `PRGINS`/`PRGSET`, where a string containing `CHR$(10)`
/// writes multiple lines and an empty string still adds one (blank) line.
pub(crate) fn split_separated(s: &SbStr) -> Vec<SbStr> {
    let mut out: Vec<SbStr> = vec![Vec::new()];
    for &u in s {
        if u == LF {
            out.push(Vec::new());
        } else {
            out.last_mut().unwrap().push(u);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(t: &str) -> SbStr {
        t.encode_utf16().collect()
    }
    fn flat(lines: &[SbStr]) -> Vec<String> {
        lines.iter().map(|l| String::from_utf16_lossy(l)).collect()
    }

    #[test]
    fn terminated_strips_trailing_lf() {
        assert_eq!(flat(&split_terminated(&s("A\nB\n"))), ["A", "B"]);
        assert_eq!(flat(&split_terminated(&s("A\nB"))), ["A", "B"]);
        assert_eq!(flat(&split_terminated(&s("A\n\nB"))), ["A", "", "B"]);
        assert!(split_terminated(&s("")).is_empty());
    }

    #[test]
    fn separated_keeps_empties() {
        assert_eq!(flat(&split_separated(&s(""))), [""]);
        assert_eq!(flat(&split_separated(&s("a\nb"))), ["a", "b"]);
        assert_eq!(flat(&split_separated(&s("\nx"))), ["", "x"]);
    }

    #[test]
    fn char_and_free_counts() {
        let mut slot = PrgSlot::default();
        slot.set_source(&s("AB\nCDE")); // 2+1 + 3+1 = 7
        assert_eq!(slot.lines.len(), 2);
        assert_eq!(slot.char_count(), 7);
        assert_eq!(slot.free_count(), SLOT_CAPACITY - 7);
    }
}
