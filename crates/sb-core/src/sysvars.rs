//! System variables (M6-T3) — the reserved names a bare identifier resolves to *before*
//! any user variable.
//!
//! SmileBASIC reserves 24 system-variable names (`spec/reference/sysvars.yaml`, located in
//! the binary's UTF-16LE keyword pool). They are not ordinary variables: a bare `MAINCNT`,
//! `VERSION`, `TIME$`, … reads system state, and assigning to a read-only one raises a Syntax
//! error (errnum 3). Two are *writable* — `TABSTEP` and `SYSBEEP` — and an assignment to them
//! takes effect on the VM's state.
//!
//! Three of the 24 are handled outside this enum because they have no runtime state of their
//! own: `TRUE`/`FALSE` are folded to the Integer literals `1`/`0` by the lexer, and `HARDWARE`
//! reads through the screen-config builtin (M4-T4). The remaining 21 are modelled here as
//! [`Sysvar`]; the VM maps each to a value in [`Vm::read_sysvar`](crate::vm::Vm) and routes the
//! two writable ones through [`Vm::write_sysvar`](crate::vm::Vm).

/// A reserved system variable (everything except the lexer-folded `TRUE`/`FALSE` and the
/// builtin-routed `HARDWARE`). The compiler resolves a bare-name read of one of these to
/// [`Op::PushSysvar`](crate::bytecode::Op::PushSysvar) ahead of any user variable; an
/// assignment to a [`writable`](Sysvar::writable) one compiles to
/// [`Op::StoreSysvar`](crate::bytecode::Op::StoreSysvar), and an assignment to a read-only one
/// is a compile-time Syntax error (errnum 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sysvar {
    /// `CSRX` — text-cursor column.
    Csrx,
    /// `CSRY` — text-cursor row.
    Csry,
    /// `CSRZ` — text-cursor depth (the console is a flat grid, so this reads 0).
    Csrz,
    /// `FREEMEM` — free user memory in KB (a fixed faithful model; exact value oracle-pending).
    Freemem,
    /// `VERSION` — packed system version `&HXXYYZZZZ` (3.6.0 → `50724864`).
    Version,
    /// `TABSTEP` — `PRINT ,` tab width. **Writable.**
    Tabstep,
    /// `SYSBEEP` — system-beep enable flag (TRUE = allowed). **Writable.**
    Sysbeep,
    /// `ERRNUM` — the errnum that halted the program (0 = none).
    Errnum,
    /// `ERRLINE` — the 1-based source line an error occurred on.
    Errline,
    /// `ERRPRG` — the program SLOT an error occurred in.
    Errprg,
    /// `PRGSLOT` — the current program SLOT for the PRG* instructions.
    Prgslot,
    /// `RESULT` — last DIALOG result (TRUE/FALSE/-1 = Suspended; 0 with no dialog).
    Result,
    /// `MAINCNT` — frames since SmileBASIC launched (the 60 fps frame counter).
    Maincnt,
    /// `MICPOS` — current mic sampling location (0 when not recording).
    Micpos,
    /// `MICSIZE` — samples in the mic buffer (0 when not recording).
    Micsize,
    /// `MPCOUNT` — participants in a wireless session (0 offline).
    Mpcount,
    /// `MPHOST` — wireless host ID (0 offline).
    Mphost,
    /// `MPLOCAL` — wireless local user ID (0 offline).
    Mplocal,
    /// `TIME$` — the time string `HH:MM:SS`.
    Time,
    /// `DATE$` — the date string `YYYY/MM/DD`.
    Date,
    /// `CALLIDX` — the index passed into an SPFUNC/BGFUNC callback (0 outside one).
    Callidx,
}

impl Sysvar {
    /// Map a canonical (uppercased, `$`-suffix kept) identifier to its system variable, if
    /// any. The `$`-suffixed string names match only with their suffix, e.g. `TIME$`/`DATE$`;
    /// `ERRNUM$`, `MAINCNT$`, … are ordinary string variables.
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "CSRX" => Self::Csrx,
            "CSRY" => Self::Csry,
            "CSRZ" => Self::Csrz,
            "FREEMEM" => Self::Freemem,
            "VERSION" => Self::Version,
            "TABSTEP" => Self::Tabstep,
            "SYSBEEP" => Self::Sysbeep,
            "ERRNUM" => Self::Errnum,
            "ERRLINE" => Self::Errline,
            "ERRPRG" => Self::Errprg,
            "PRGSLOT" => Self::Prgslot,
            "RESULT" => Self::Result,
            "MAINCNT" => Self::Maincnt,
            "MICPOS" => Self::Micpos,
            "MICSIZE" => Self::Micsize,
            "MPCOUNT" => Self::Mpcount,
            "MPHOST" => Self::Mphost,
            "MPLOCAL" => Self::Mplocal,
            "TIME$" => Self::Time,
            "DATE$" => Self::Date,
            "CALLIDX" => Self::Callidx,
            _ => return None,
        })
    }

    /// The canonical name as it appears in source (`$`-suffix kept for the string ones).
    pub fn canonical(self) -> &'static str {
        match self {
            Self::Csrx => "CSRX",
            Self::Csry => "CSRY",
            Self::Csrz => "CSRZ",
            Self::Freemem => "FREEMEM",
            Self::Version => "VERSION",
            Self::Tabstep => "TABSTEP",
            Self::Sysbeep => "SYSBEEP",
            Self::Errnum => "ERRNUM",
            Self::Errline => "ERRLINE",
            Self::Errprg => "ERRPRG",
            Self::Prgslot => "PRGSLOT",
            Self::Result => "RESULT",
            Self::Maincnt => "MAINCNT",
            Self::Micpos => "MICPOS",
            Self::Micsize => "MICSIZE",
            Self::Mpcount => "MPCOUNT",
            Self::Mphost => "MPHOST",
            Self::Mplocal => "MPLOCAL",
            Self::Time => "TIME$",
            Self::Date => "DATE$",
            Self::Callidx => "CALLIDX",
        }
    }

    /// Whether assignment (`NAME = expr`) is legal. Only `TABSTEP` and `SYSBEEP` are writable
    /// (`sysvars.yaml writable=true`, confirmed by the corpus assignment split); every other
    /// system variable raises a Syntax error (errnum 3) on assignment.
    pub fn writable(self) -> bool {
        matches!(self, Self::Tabstep | Self::Sysbeep)
    }

    /// Whether the variable reads as a String (`TIME$`/`DATE$`) rather than an Integer.
    pub fn is_string(self) -> bool {
        matches!(self, Self::Time | Self::Date)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: &[Sysvar] = &[
        Sysvar::Csrx,
        Sysvar::Csry,
        Sysvar::Csrz,
        Sysvar::Freemem,
        Sysvar::Version,
        Sysvar::Tabstep,
        Sysvar::Sysbeep,
        Sysvar::Errnum,
        Sysvar::Errline,
        Sysvar::Errprg,
        Sysvar::Prgslot,
        Sysvar::Result,
        Sysvar::Maincnt,
        Sysvar::Micpos,
        Sysvar::Micsize,
        Sysvar::Mpcount,
        Sysvar::Mphost,
        Sysvar::Mplocal,
        Sysvar::Time,
        Sysvar::Date,
        Sysvar::Callidx,
    ];

    #[test]
    fn names_round_trip() {
        for &sv in ALL {
            assert_eq!(Sysvar::from_name(sv.canonical()), Some(sv));
        }
    }

    #[test]
    fn only_tabstep_and_sysbeep_are_writable() {
        for &sv in ALL {
            assert_eq!(
                sv.writable(),
                matches!(sv, Sysvar::Tabstep | Sysvar::Sysbeep),
                "{}",
                sv.canonical()
            );
        }
    }

    #[test]
    fn only_time_and_date_are_strings() {
        for &sv in ALL {
            assert_eq!(sv.is_string(), matches!(sv, Sysvar::Time | Sysvar::Date));
        }
    }

    #[test]
    fn non_sysvars_and_suffixed_names_are_not_matched() {
        assert_eq!(Sysvar::from_name("A"), None);
        assert_eq!(Sysvar::from_name("ERRNUM$"), None); // suffixed → ordinary string var
        assert_eq!(Sysvar::from_name("TIME"), None); // string sysvars need the `$`
        assert_eq!(Sysvar::from_name("HARDWARE"), None); // routed through the builtin (M4-T4)
        assert_eq!(Sysvar::from_name("TRUE"), None); // folded to a literal by the lexer
    }
}
