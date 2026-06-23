//! System variables — the error-model slice (M1-T13).
//!
//! Only the three **error-state** read-only system variables live here:
//! `ERRNUM`/`ERRLINE`/`ERRPRG`. They are the *only* programmatic window onto a runtime
//! error (SmileBASIC has no error trapping), readable after a halt and never assignable.
//! The general, writable sysvar surface (`MAINCNT`, `TABSTEP`, …) is M6-T3 and is not
//! modelled here.
//!
//! Per `spec/concepts/error-model.md` and `spec/reference/sysvars.yaml`
//! (`ERRNUM` @0x2ef53c, `ERRLINE` @0x2ef1bc, `ERRPRG` @0x2ef3d4 — all `writable=false`).

/// A read-only error-state system variable. The compiler resolves a bare-name read of
/// one of these to [`Op::PushSysvar`](crate::bytecode::Op::PushSysvar); an attempt to
/// assign to one is a Syntax error (errnum 3), matching `writable=false`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrSysvar {
    /// `ERRNUM` — the errnum that halted the program (0 = none).
    Errnum,
    /// `ERRLINE` — the 1-based source line the error occurred on.
    Errline,
    /// `ERRPRG` — the program SLOT (0..3) the error occurred in.
    Errprg,
}

impl ErrSysvar {
    /// Map a canonical (uppercased, suffix-kept) identifier to its error sysvar, if any.
    /// Only the exact suffix-less names match; `ERRNUM$` etc. are ordinary variables.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "ERRNUM" => Some(Self::Errnum),
            "ERRLINE" => Some(Self::Errline),
            "ERRPRG" => Some(Self::Errprg),
            _ => None,
        }
    }

    /// The canonical name as it appears in source.
    pub fn canonical(self) -> &'static str {
        match self {
            Self::Errnum => "ERRNUM",
            Self::Errline => "ERRLINE",
            Self::Errprg => "ERRPRG",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip() {
        for sv in [ErrSysvar::Errnum, ErrSysvar::Errline, ErrSysvar::Errprg] {
            assert_eq!(ErrSysvar::from_name(sv.canonical()), Some(sv));
        }
    }

    #[test]
    fn non_sysvars_and_suffixed_names_are_not_matched() {
        assert_eq!(ErrSysvar::from_name("A"), None);
        assert_eq!(ErrSysvar::from_name("ERRNUM$"), None);
        assert_eq!(ErrSysvar::from_name("MAINCNT"), None);
    }
}
