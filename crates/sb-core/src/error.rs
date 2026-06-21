//! SmileBASIC 3.6.0 runtime errors.
//!
//! The error numbers and messages come from the official `InstructionList.pdf`
//! "Error Table" (mirrored at `sb-docs/smilebasic-3/reference/error-table.md`),
//! numbers 3..=47. After an error, SmileBASIC stores the number in `ERRNUM` and
//! the line in `ERRLINE`.
//!
//! NOTE on fidelity: the documented messages here are the *documented* layer of the
//! spec confidence ladder. The exact strings baked into the 3.6.0 binary live in the
//! disassembly (`cia_3.6.0.lst`, around `0x1E965C`) and may differ in capitalization
//! or punctuation. Those get reconciled to `hw_verified` during oracle harvest.

/// SmileBASIC error numbers (the value stored in the `ERRNUM` system variable).
///
/// Numbers 0..=2 are not documented in the official table (reserved / "OK"/"Break"
/// style codes) and are intentionally omitted until verified against hardware.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ErrNum {
    SyntaxError = 3,
    IllegalFunctionCall = 4,
    StackOverflow = 5,
    StackUnderflow = 6,
    DivideByZero = 7,
    TypeMismatch = 8,
    Overflow = 9,
    OutOfRange = 10,
    OutOfMemory = 11,
    OutOfCodeMemory = 12,
    OutOfData = 13,
    UndefinedLabel = 14,
    UndefinedVariable = 15,
    UndefinedFunction = 16,
    DuplicateLabel = 17,
    DuplicateVariable = 18,
    DuplicateFunction = 19,
    ForWithoutNext = 20,
    NextWithoutFor = 21,
    RepeatWithoutUntil = 22,
    UntilWithoutRepeat = 23,
    WhileWithoutWend = 24,
    WendWithoutWhile = 25,
    ThenWithoutEndif = 26,
    ElseWithoutEndif = 27,
    EndifWithoutIf = 28,
    DefWithoutEnd = 29,
    ReturnWithoutGosub = 30,
    SubscriptOutOfRange = 31,
    NestedDef = 32,
    CantContinue = 33,
    IllegalSymbolString = 34,
    IllegalFileFormat = 35,
    MicNotAvailable = 36,
    MotionSensorNotAvailable = 37,
    UsePrgeditFirst = 38,
    AnimationTooLong = 39,
    IllegalAnimationData = 40,
    StringTooLong = 41,
    CommunicationBufferOverflow = 42,
    CantUseFromDirectMode = 43,
    CantUseInProgram = 44,
    CantUseInToolProgram = 45,
    LoadFailed = 46,
    IllegalMml = 47,
}

impl ErrNum {
    /// The documented human-readable message for this error.
    pub fn message(self) -> &'static str {
        use ErrNum::*;
        match self {
            SyntaxError => "Syntax error",
            IllegalFunctionCall => "Illegal function call",
            StackOverflow => "Stack overflow",
            StackUnderflow => "Stack underflow",
            DivideByZero => "Divide by zero",
            TypeMismatch => "Type mismatch",
            Overflow => "Overflow",
            OutOfRange => "Out of range",
            OutOfMemory => "Out of memory",
            OutOfCodeMemory => "Out of code memory",
            OutOfData => "Out of DATA",
            UndefinedLabel => "Undefined label",
            UndefinedVariable => "Undefined variable",
            UndefinedFunction => "Undefined function",
            DuplicateLabel => "Duplicate label",
            DuplicateVariable => "Duplicate variable",
            DuplicateFunction => "Duplicate function",
            ForWithoutNext => "FOR without NEXT",
            NextWithoutFor => "NEXT without FOR",
            RepeatWithoutUntil => "REPEAT without UNTIL",
            UntilWithoutRepeat => "UNTIL without REPEAT",
            WhileWithoutWend => "WHILE without WEND",
            WendWithoutWhile => "WEND without WHILE",
            ThenWithoutEndif => "THEN without ENDIF",
            ElseWithoutEndif => "ELSE without ENDIF",
            EndifWithoutIf => "ENDIF without IF",
            DefWithoutEnd => "DEF without END",
            ReturnWithoutGosub => "RETURN without GOSUB",
            SubscriptOutOfRange => "Subscript out of range",
            NestedDef => "Nested DEF",
            CantContinue => "Can't continue",
            IllegalSymbolString => "Illegal symbol string",
            IllegalFileFormat => "Illegal file format",
            MicNotAvailable => "Mic is not available",
            MotionSensorNotAvailable => "Motion sensor is not available",
            UsePrgeditFirst => "Use PRGEDIT before any PRG function",
            AnimationTooLong => "Animation is too long",
            IllegalAnimationData => "Illegal animation data",
            StringTooLong => "String too long",
            CommunicationBufferOverflow => "Communication buffer overflow",
            CantUseFromDirectMode => "Can't use from DIRECT mode",
            CantUseInProgram => "Can't use in program",
            CantUseInToolProgram => "Can't use in tool program",
            LoadFailed => "Load failed",
            IllegalMml => "Illegal MML",
        }
    }

    /// The numeric `ERRNUM` value.
    pub fn num(self) -> u8 {
        self as u8
    }
}

/// A SmileBASIC error carrying its number and (optionally) the source line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SbError {
    pub num: ErrNum,
    /// 1-based source line (`ERRLINE`), if known.
    pub line: Option<u32>,
}

impl SbError {
    pub fn new(num: ErrNum) -> Self {
        Self { num, line: None }
    }

    pub fn at(num: ErrNum, line: u32) -> Self {
        Self {
            num,
            line: Some(line),
        }
    }
}

impl core::fmt::Display for SbError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // SmileBASIC prints e.g. "Type mismatch in 0:120".
        match self.line {
            Some(line) => write!(f, "{} (line {})", self.num.message(), line),
            None => write!(f, "{}", self.num.message()),
        }
    }
}

impl std::error::Error for SbError {}

/// Convenient `Result` alias for the core.
pub type SbResult<T> = Result<T, SbError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errnum_values_match_official_table() {
        // A few anchors from sb-docs/smilebasic-3/reference/error-table.md.
        assert_eq!(ErrNum::TypeMismatch.num(), 8); // famously NOT 20
        assert_eq!(ErrNum::ForWithoutNext.num(), 20);
        assert_eq!(ErrNum::SyntaxError.num(), 3);
        assert_eq!(ErrNum::IllegalMml.num(), 47);
        assert_eq!(ErrNum::DivideByZero.message(), "Divide by zero");
    }
}
