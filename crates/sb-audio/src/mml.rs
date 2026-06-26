//! MML (Music Macro Language) parser — string → per-channel note-event stream.
//!
//! Implements the grammar specified in `spec/concepts/mml-grammar.md` (S-C5): channels,
//! tempo/length/gate/ties, pitch/octave/key, volume/pan/envelope, instruments, modulation/
//! detune/tremolo/vibrato/autopan, repeats `[ ]N`, MML variables `$0`–`$7`, and `{macro}`
//! definitions/uses. The deterministic M5 gate is *MML → events*, so this parser fully
//! resolves control state (octave, default length, dots, ties, repeats, macros, `$`-vars)
//! at parse time and emits a flat per-channel timeline the synth (M5-T2) can consume.
//!
//! Any construct the real parser rejects is **errnum 47 (Illegal MML)**; the error carries
//! the char offset of the failure (the `^` caret position SmileBASIC reports).
//!
//! ## Confidence
//! Grammar + ranges are `documented`/`disassembled` per S-C5. A few values the docs do not
//! pin down are flagged inline and tracked in beads (bd:sb-interpreter-i8p):
//! - tick base = 192/whole-note (192 ticks); the exact tempo→frame conversion is the synth's
//!   (M5-T2) concern and not modelled here;
//! - default channel state (volume/velocity 127, pan 64 center, octave 4, length 4, gate 8,
//!   tempo 120, instrument 0) — defaults the synth may refine;
//! - instrument-number ceiling: the docs list `@256` only, but the corpus attests `@256`–
//!   `@287`+ SFX-bank numbers, so we accept `@0`–`@511` rather than reject real programs;
//! - `@D` detune folds *fine* (sub-semitone) pitch, so it is emitted as a control event, not
//!   folded into the integer MIDI note.

/// errnum raised for any malformed MML (BGMPLAY handler `@0x1a2e3c`: `mov r0,#0x2f`).
pub const ERRNUM_ILLEGAL_MML: i32 = 47;

/// Internal tick resolution: 192 ticks per whole note (48 per quarter). The LCM of the
/// documented length divisors (1,2,3,4,6,8,12,16,24,32,48,64,96,192). Hypothesis — queued.
pub const TICKS_PER_WHOLE_NOTE: u32 = 192;

/// Number of MML channels (`:0`–`:15`).
pub const NUM_CHANNELS: usize = 16;

/// A parse failure. SmileBASIC reports all malformed MML as errnum 47, marking the offending
/// byte with a `^` caret; `offset` is that char position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MmlError {
    pub errnum: i32,
    pub offset: usize,
    pub message: String,
}

impl MmlError {
    fn at(offset: usize, message: impl Into<String>) -> Self {
        MmlError {
            errnum: ERRNUM_ILLEGAL_MML,
            offset,
            message: message.into(),
        }
    }
}

impl core::fmt::Display for MmlError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Illegal MML (errnum {}) at offset {}: {}",
            self.errnum, self.offset, self.message
        )
    }
}

/// One resolved event on a channel timeline. Notes carry pitch/duration/gate/velocity so the
/// synth needs no look-back; channel-wide settings (volume, pan, instrument, envelope, LFOs,
/// tempo, detune, modulation) ride as their own ordered control events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// A sounded note. `pitch` = MIDI key (after octave/accidental/`<`/`>`/`!`), `duration`
    /// in ticks (after length/dots/ties), `gate` = current `Q` (0–8), `velocity` = current
    /// `@V` (0–127). `slur` marks a `&`-connected note of a *different* pitch (legato), and
    /// `portamento` a leading `_` (pitch slides from the previous note).
    Note {
        pitch: u8,
        duration: u32,
        gate: u8,
        velocity: u8,
        slur: bool,
        portamento: bool,
    },
    /// Silence for `duration` ticks (`R`).
    Rest { duration: u32 },
    /// Tempo change `T` (beats/min, 1–512).
    Tempo(u16),
    /// Channel volume `V` (0–127).
    Volume(u8),
    /// Volume up by N steps `(` / `(N` (bare = 1). The documented form is operand-less, but
    /// real SB3 programs pervasively use `(N` (corpus, 20+ programs, see notes) — step *size*
    /// is oracle-pending.
    VolumeUp(u8),
    /// Volume down by N steps `)` / `)N` (bare = 1).
    VolumeDown(u8),
    /// Pan `P` (0–63 left, 64 center, 65–127 right).
    Pan(u8),
    /// Instrument select `@n`.
    Instrument(u16),
    /// ADSR envelope `@E A,D,S,R` (each 0–127).
    Envelope { a: u8, d: u8, s: u8, r: u8 },
    /// Release/reset the envelope `@ER`.
    EnvelopeReset,
    /// Detune `@D` (−128…127, fine sub-semitone pitch).
    Detune(i16),
    /// Tremolo (amplitude LFO) `@MA depth,range,speed,delay`.
    Tremolo {
        depth: u8,
        range: u8,
        speed: u8,
        delay: u8,
    },
    /// Vibrato (pitch LFO) `@MP depth,range,speed,delay`.
    Vibrato {
        depth: u8,
        range: u8,
        speed: u8,
        delay: u8,
    },
    /// Auto-pan (pan LFO) `@ML depth,range,speed,delay`.
    AutoPan {
        depth: u8,
        range: u8,
        speed: u8,
        delay: u8,
    },
    /// Start modulation `@MON`.
    ModulationOn,
    /// Stop modulation `@MOF`.
    ModulationOff,
    /// Start of an endless repeat (`[` whose matching `]` had no count). The synth loops back
    /// here on reaching the matching `LoopEnd`. Finite `[ ]N` repeats are fully unrolled and
    /// produce no marker.
    LoopStart,
    /// End of an endless repeat — loop back to the matching `LoopStart`.
    LoopEnd,
}

/// The parsed song: one event timeline per channel (0–15). Empty channels have empty vecs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Song {
    pub channels: Vec<Vec<Event>>,
}

impl Song {
    fn new() -> Self {
        Song {
            channels: (0..NUM_CHANNELS).map(|_| Vec::new()).collect(),
        }
    }
}

/// Parse an MML string into a per-channel [`Song`]. Returns [`MmlError`] (errnum 47) on any
/// malformed input.
pub fn parse(mml: &str) -> Result<Song, MmlError> {
    // Notes and commands are case-insensitive (`cde` ≡ `CDE`), but **macro labels are
    // case-sensitive** — real SB3 programs define both `{r=…}` and `{R=…}` as distinct macros
    // (sbsave corpus, 10+ programs). So we keep the original case here and fold per-char only
    // at command/note dispatch; macro labels keep their literal case. Work on chars so offsets
    // are stable regardless of multibyte content.
    let chars: Vec<char> = mml.chars().collect();
    let mut macros = MacroTable::new();
    let toks = lex(&chars, 0, &mut macros, true)?;
    let expanded = expand(&toks, &macros)?;
    resolve(&expanded)
}

// ---------------------------------------------------------------------------------------
// Tokens
// ---------------------------------------------------------------------------------------

/// A lexed command plus the char offset where it began (for caret reporting after expansion).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Tok {
    kind: Tk,
    off: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tk {
    Channel(u8),
    Tempo(u16),
    DefaultLen {
        divisor: u32,
        dots: u32,
    },
    Octave(u8),
    OctaveUp,
    OctaveDown,
    OctaveInvert,
    Gate(u8),
    Note {
        semitone: i32,
        accidental: i32,
        len: Option<u32>,
        dots: u32,
    },
    Rest {
        len: Option<u32>,
        dots: u32,
    },
    AbsPitch(u8),
    Tie,
    TieLength {
        len: u32,
        dots: u32,
    },
    Portamento,
    Volume(u8),
    VolumeUp(u8),
    VolumeDown(u8),
    Pan(u8),
    Velocity(u8),
    Instrument(u16),
    Envelope {
        a: u8,
        d: u8,
        s: u8,
        r: u8,
    },
    EnvelopeReset,
    Detune(i16),
    Tremolo {
        depth: u8,
        range: u8,
        speed: u8,
        delay: u8,
    },
    Vibrato {
        depth: u8,
        range: u8,
        speed: u8,
        delay: u8,
    },
    AutoPan {
        depth: u8,
        range: u8,
        speed: u8,
        delay: u8,
    },
    ModulationOn,
    ModulationOff,
    LoopStart,
    LoopEndFinite(u32),
    LoopEndInfinite,
    /// Post-expansion endless-loop terminator (matches a retained `LoopStart`).
    LoopEnd,
    MacroUse(String),
}

type MacroTable = std::collections::HashMap<String, Vec<Tok>>;

// ---------------------------------------------------------------------------------------
// Lexer
// ---------------------------------------------------------------------------------------

/// Lex a (sub)slice of upper-cased chars into tokens. `base` is the absolute offset of
/// `chars[0]` for error reporting. Macro *definitions* are pulled into `macros` and removed
/// from the stream; macro *uses* become `MacroUse`. `allow_channel` is false inside macro
/// bodies (a `:` there is illegal).
fn lex(
    chars: &[char],
    base: usize,
    macros: &mut MacroTable,
    allow_channel: bool,
) -> Result<Vec<Tok>, MmlError> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let start = base + i;
        // Fold case for command/note dispatch only; macro labels (handled under '{') keep
        // their original case because SB3 macro labels are case-sensitive.
        let c = chars[i].to_ascii_uppercase();
        match c {
            ' ' | '\t' | '\r' | '\n' => {
                i += 1;
            }
            ':' => {
                if !allow_channel {
                    return Err(MmlError::at(
                        start,
                        "channel command not allowed in a macro",
                    ));
                }
                i += 1;
                let (n, ni) = read_uint(chars, i, base, macros)?;
                i = ni;
                if n > 15 {
                    return Err(MmlError::at(start, "channel must be 0–15"));
                }
                out.push(Tok {
                    kind: Tk::Channel(n as u8),
                    off: start,
                });
            }
            'T' => {
                i += 1;
                let (n, ni) = read_uint(chars, i, base, macros)?;
                i = ni;
                if !(1..=512).contains(&n) {
                    return Err(MmlError::at(start, "tempo must be 1–512"));
                }
                out.push(Tok {
                    kind: Tk::Tempo(n as u16),
                    off: start,
                });
            }
            'L' => {
                i += 1;
                let (len, dots, ni) = read_len_dots(chars, i, base, macros)?;
                i = ni;
                // The divisor is required for L; dots are allowed (`L2.` = dotted half default —
                // sbsave corpus, 7+ programs, e.g. L2.C / L8.).
                let divisor = len.ok_or_else(|| MmlError::at(start, "expected a number"))?;
                check_len(divisor, start)?;
                out.push(Tok {
                    kind: Tk::DefaultLen { divisor, dots },
                    off: start,
                });
            }
            'O' => {
                i += 1;
                let (n, ni) = read_uint(chars, i, base, macros)?;
                i = ni;
                if n > 8 {
                    return Err(MmlError::at(start, "octave must be 0–8"));
                }
                out.push(Tok {
                    kind: Tk::Octave(n as u8),
                    off: start,
                });
            }
            'Q' => {
                i += 1;
                let (n, ni) = read_uint(chars, i, base, macros)?;
                i = ni;
                if n > 8 {
                    return Err(MmlError::at(start, "gate must be 0–8"));
                }
                out.push(Tok {
                    kind: Tk::Gate(n as u8),
                    off: start,
                });
            }
            'V' => {
                i += 1;
                let (n, ni) = read_uint(chars, i, base, macros)?;
                i = ni;
                if n > 127 {
                    return Err(MmlError::at(start, "volume must be 0–127"));
                }
                out.push(Tok {
                    kind: Tk::Volume(n as u8),
                    off: start,
                });
            }
            'P' => {
                i += 1;
                let (n, ni) = read_uint(chars, i, base, macros)?;
                i = ni;
                if n > 127 {
                    return Err(MmlError::at(start, "pan must be 0–127"));
                }
                out.push(Tok {
                    kind: Tk::Pan(n as u8),
                    off: start,
                });
            }
            'N' => {
                i += 1;
                let (n, ni) = read_uint(chars, i, base, macros)?;
                i = ni;
                if n > 127 {
                    return Err(MmlError::at(start, "key number must be 0–127"));
                }
                out.push(Tok {
                    kind: Tk::AbsPitch(n as u8),
                    off: start,
                });
            }
            'R' => {
                i += 1;
                let (len, dots, ni) = read_len_dots(chars, i, base, macros)?;
                i = ni;
                out.push(Tok {
                    kind: Tk::Rest { len, dots },
                    off: start,
                });
            }
            'C' | 'D' | 'E' | 'F' | 'G' | 'A' | 'B' => {
                let (kind, ni) = read_note_body(chars, i + 1, base, macros, letter_semitone(c), 0)?;
                i = ni;
                out.push(Tok { kind, off: start });
            }
            '+' | '#' | '-' => {
                // Leading accidental(s) applying to the FOLLOWING note: `+B`, `#F`, `-C` —
                // undocumented but pervasive (sbsave corpus, 62+ programs). A note must follow.
                let mut acc = 0i32;
                while i < chars.len() {
                    match chars[i] {
                        '#' | '+' => {
                            acc += 1;
                            i += 1;
                        }
                        '-' => {
                            acc -= 1;
                            i += 1;
                        }
                        _ => break,
                    }
                }
                let semitone = match chars.get(i).map(|c| c.to_ascii_uppercase()) {
                    Some(nl @ ('C' | 'D' | 'E' | 'F' | 'G' | 'A' | 'B')) => letter_semitone(nl),
                    _ => return Err(MmlError::at(start, "accidental must precede a note")),
                };
                let (kind, ni) = read_note_body(chars, i + 1, base, macros, semitone, acc)?;
                i = ni;
                out.push(Tok { kind, off: start });
            }
            '<' => {
                i += 1;
                out.push(Tok {
                    kind: Tk::OctaveUp,
                    off: start,
                });
            }
            '>' => {
                i += 1;
                out.push(Tok {
                    kind: Tk::OctaveDown,
                    off: start,
                });
            }
            '!' => {
                i += 1;
                out.push(Tok {
                    kind: Tk::OctaveInvert,
                    off: start,
                });
            }
            '(' => {
                i += 1;
                let (steps, ni) = read_step(chars, i, base, macros)?;
                i = ni;
                out.push(Tok {
                    kind: Tk::VolumeUp(steps),
                    off: start,
                });
            }
            ')' => {
                i += 1;
                let (steps, ni) = read_step(chars, i, base, macros)?;
                i = ni;
                out.push(Tok {
                    kind: Tk::VolumeDown(steps),
                    off: start,
                });
            }
            '&' => {
                i += 1;
                out.push(Tok {
                    kind: Tk::Tie,
                    off: start,
                });
                // Short tie form `&8`: a bare length right after `&` continues the prior pitch.
                if i < chars.len()
                    && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == '$')
                {
                    let (len, dots, ni) = read_len_dots(chars, i, base, macros)?;
                    i = ni;
                    let len = len.ok_or_else(|| MmlError::at(start, "tie length expected"))?;
                    out.push(Tok {
                        kind: Tk::TieLength { len, dots },
                        off: start,
                    });
                }
            }
            '_' => {
                i += 1;
                out.push(Tok {
                    kind: Tk::Portamento,
                    off: start,
                });
            }
            '[' => {
                i += 1;
                out.push(Tok {
                    kind: Tk::LoopStart,
                    off: start,
                });
            }
            ']' => {
                i += 1;
                // Optional repeat count; bare `]` is an endless loop.
                if i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '$') {
                    let (n, ni) = read_uint(chars, i, base, macros)?;
                    i = ni;
                    if n < 1 {
                        return Err(MmlError::at(start, "repeat count must be ≥ 1"));
                    }
                    out.push(Tok {
                        kind: Tk::LoopEndFinite(n),
                        off: start,
                    });
                } else {
                    out.push(Tok {
                        kind: Tk::LoopEndInfinite,
                        off: start,
                    });
                }
            }
            '$' => {
                // Command-level: an assignment `$n=value`. A bare `$n` reference is only legal
                // as an operand (handled inside read_uint), never standalone.
                i += 1;
                let (n, ni) = read_uint(chars, i, base, macros)?;
                i = ni;
                if n > 7 {
                    return Err(MmlError::at(start, "MML variable index must be 0–7"));
                }
                if i >= chars.len() || chars[i] != '=' {
                    return Err(MmlError::at(start, "expected '=' after $n assignment"));
                }
                i += 1; // '='
                let (val, ni) = read_uint(chars, i, base, macros)?;
                i = ni;
                if val > 255 {
                    return Err(MmlError::at(start, "MML variable value must be 0–255"));
                }
                set_var(macros, n as u8, val);
            }
            '{' => {
                let close = find_matching_brace(chars, i, base)?;
                let inner = &chars[i + 1..close];
                lex_brace(inner, base + i + 1, macros, &mut out, start)?;
                i = close + 1;
            }
            '@' => {
                i += 1;
                i = lex_at(chars, i, base, &mut out, start, macros)?;
            }
            other => {
                return Err(MmlError::at(
                    start,
                    format!("unexpected character '{other}'"),
                ));
            }
        }
    }
    Ok(out)
}

/// Lex everything after an `@`.
fn lex_at(
    chars: &[char],
    mut i: usize,
    base: usize,
    out: &mut Vec<Tok>,
    start: usize,
    macros: &MacroTable,
) -> Result<usize, MmlError> {
    if i >= chars.len() {
        return Err(MmlError::at(start, "'@' must be followed by a command"));
    }
    match chars[i].to_ascii_uppercase() {
        'E' => {
            i += 1;
            if i < chars.len() && chars[i].eq_ignore_ascii_case(&'R') {
                i += 1;
                out.push(Tok {
                    kind: Tk::EnvelopeReset,
                    off: start,
                });
            } else {
                let (vals, ni) = read_operands(chars, i, base, 4, macros)?;
                i = ni;
                check_127(&vals, start)?;
                out.push(Tok {
                    kind: Tk::Envelope {
                        a: vals[0] as u8,
                        d: vals[1] as u8,
                        s: vals[2] as u8,
                        r: vals[3] as u8,
                    },
                    off: start,
                });
            }
        }
        'V' => {
            i += 1;
            let (n, ni) = read_uint(chars, i, base, macros)?;
            i = ni;
            if n > 127 {
                return Err(MmlError::at(start, "@V velocity must be 0–127"));
            }
            out.push(Tok {
                kind: Tk::Velocity(n as u8),
                off: start,
            });
        }
        'D' => {
            i += 1;
            let neg = i < chars.len() && chars[i] == '-';
            if neg {
                i += 1;
            }
            let (n, ni) = read_uint(chars, i, base, macros)?;
            i = ni;
            let v = if neg { -(n as i64) } else { n as i64 };
            if !(-128..=127).contains(&v) {
                return Err(MmlError::at(start, "@D detune must be −128…127"));
            }
            out.push(Tok {
                kind: Tk::Detune(v as i16),
                off: start,
            });
        }
        'M' => {
            i += 1;
            if i >= chars.len() {
                return Err(MmlError::at(start, "'@M' must be followed by O/A/P/L"));
            }
            match chars[i].to_ascii_uppercase() {
                'O' => {
                    i += 1;
                    if i >= chars.len() {
                        return Err(MmlError::at(start, "'@MO' must be followed by N or F"));
                    }
                    match chars[i].to_ascii_uppercase() {
                        'N' => {
                            i += 1;
                            out.push(Tok {
                                kind: Tk::ModulationOn,
                                off: start,
                            });
                        }
                        'F' => {
                            i += 1;
                            out.push(Tok {
                                kind: Tk::ModulationOff,
                                off: start,
                            });
                        }
                        _ => return Err(MmlError::at(start, "expected @MON or @MOF")),
                    }
                }
                'A' | 'P' | 'L' => {
                    let which = chars[i].to_ascii_uppercase();
                    i += 1;
                    let (vals, ni) = read_operands(chars, i, base, 4, macros)?;
                    i = ni;
                    check_127(&vals, start)?;
                    let (d, r, s, dl) =
                        (vals[0] as u8, vals[1] as u8, vals[2] as u8, vals[3] as u8);
                    let kind = match which {
                        'A' => Tk::Tremolo {
                            depth: d,
                            range: r,
                            speed: s,
                            delay: dl,
                        },
                        'P' => Tk::Vibrato {
                            depth: d,
                            range: r,
                            speed: s,
                            delay: dl,
                        },
                        _ => Tk::AutoPan {
                            depth: d,
                            range: r,
                            speed: s,
                            delay: dl,
                        },
                    };
                    out.push(Tok { kind, off: start });
                }
                _ => return Err(MmlError::at(start, "expected @MON/@MOF/@MA/@MP/@ML")),
            }
        }
        c if c.is_ascii_digit() || c == '$' => {
            let (n, ni) = read_uint(chars, i, base, macros)?;
            i = ni;
            // The docs list @256 only, but the corpus attests @256–@287+; accept 0–511 rather
            // than reject real programs. Upper bound hypothesis — queued for oracle.
            if n > 511 {
                return Err(MmlError::at(start, "instrument number out of range"));
            }
            out.push(Tok {
                kind: Tk::Instrument(n as u16),
                off: start,
            });
        }
        _ => return Err(MmlError::at(start, "unknown '@' command")),
    }
    Ok(i)
}

/// Handle a `{...}` group: either a macro definition `{Label=MML}` (recorded, removed from
/// the stream) or a macro use `{Label}` (emits a `MacroUse`).
fn lex_brace(
    inner: &[char],
    inner_base: usize,
    macros: &mut MacroTable,
    out: &mut Vec<Tok>,
    start: usize,
) -> Result<(), MmlError> {
    // Find a top-level '=' separating label from body (depth 0 only).
    let mut depth = 0i32;
    let mut eq = None;
    for (k, &ch) in inner.iter().enumerate() {
        match ch {
            '{' => depth += 1,
            '}' => depth -= 1,
            '=' if depth == 0 => {
                eq = Some(k);
                break;
            }
            _ => {}
        }
    }
    match eq {
        Some(k) => {
            let label = label_str(&inner[..k], start)?;
            if is_reserved_key(&label) {
                return Err(MmlError::at(start, "macro label not allowed"));
            }
            if macros.contains_key(&label) {
                return Err(MmlError::at(start, "macro label redefined"));
            }
            // Body lexed with channel commands forbidden.
            let body = lex(&inner[k + 1..], inner_base + k + 1, macros, false)?;
            macros.insert(label, body);
        }
        None => {
            let label = label_str(inner, start)?;
            out.push(Tok {
                kind: Tk::MacroUse(label),
                off: start,
            });
        }
    }
    Ok(())
}

/// Validate a macro label: 1–8 alphanumerics.
fn label_str(chars: &[char], start: usize) -> Result<String, MmlError> {
    if chars.is_empty() || chars.len() > 8 || !chars.iter().all(|c| c.is_ascii_alphanumeric()) {
        return Err(MmlError::at(start, "macro label must be 1–8 alphanumerics"));
    }
    Ok(chars.iter().collect())
}

/// Read a greedy decimal `$n`-aware unsigned operand. A `$n` operand substitutes the current
/// value of MML variable n (assigned left-to-right; 0 if unassigned). BGMVAR runtime override
/// is M5-T3's concern; the parser resolves the value known at this point in the string.
fn read_uint(
    chars: &[char],
    mut i: usize,
    base: usize,
    vars: &MacroTable,
) -> Result<(u32, usize), MmlError> {
    if i < chars.len() && chars[i] == '$' {
        let at = base + i;
        i += 1;
        let (n, ni) = read_digits(chars, i, base)?;
        if n > 7 {
            return Err(MmlError::at(at, "MML variable index must be 0–7"));
        }
        return Ok((get_var(vars, n as u8), ni));
    }
    read_digits(chars, i, base)
}

/// Read the tail of a note (trailing accidentals, then optional length + dots), given the
/// base `semitone` and any `leading_acc` already accumulated from a leading accidental. `i`
/// points just past the note letter.
fn read_note_body(
    chars: &[char],
    mut i: usize,
    base: usize,
    macros: &MacroTable,
    semitone: i32,
    leading_acc: i32,
) -> Result<(Tk, usize), MmlError> {
    let mut accidental = leading_acc;
    while i < chars.len() {
        match chars[i] {
            '#' | '+' => {
                accidental += 1;
                i += 1;
            }
            '-' => {
                accidental -= 1;
                i += 1;
            }
            _ => break,
        }
    }
    let (len, dots, ni) = read_len_dots(chars, i, base, macros)?;
    Ok((
        Tk::Note {
            semitone,
            accidental,
            len,
            dots,
        },
        ni,
    ))
}

/// Read the optional step count after `(` / `)` (default 1; 0–255).
fn read_step(
    chars: &[char],
    i: usize,
    base: usize,
    macros: &MacroTable,
) -> Result<(u8, usize), MmlError> {
    if i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '$') {
        let (n, ni) = read_uint(chars, i, base, macros)?;
        if n > 255 {
            return Err(MmlError::at(base + i, "volume step must be 0–255"));
        }
        Ok((n as u8, ni))
    } else {
        Ok((1, i))
    }
}

/// Read one or more decimal digits.
fn read_digits(chars: &[char], mut i: usize, base: usize) -> Result<(u32, usize), MmlError> {
    let start = i;
    let mut n: u64 = 0;
    while i < chars.len() && chars[i].is_ascii_digit() {
        n = n * 10 + (chars[i] as u64 - '0' as u64);
        if n > u32::MAX as u64 {
            return Err(MmlError::at(base + start, "number too large"));
        }
        i += 1;
    }
    if i == start {
        return Err(MmlError::at(base + start, "expected a number"));
    }
    Ok((n as u32, i))
}

/// Read an optional length (digits) then zero or more dots.
fn read_len_dots(
    chars: &[char],
    mut i: usize,
    base: usize,
    macros: &MacroTable,
) -> Result<(Option<u32>, u32, usize), MmlError> {
    let len = if i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '$') {
        let (n, ni) = read_uint(chars, i, base, macros)?;
        i = ni;
        Some(n)
    } else {
        None
    };
    let mut dots = 0;
    while i < chars.len() && chars[i] == '.' {
        dots += 1;
        i += 1;
    }
    Ok((len, dots, i))
}

/// Read `count` comma-separated operands.
fn read_operands(
    chars: &[char],
    mut i: usize,
    base: usize,
    count: usize,
    macros: &MacroTable,
) -> Result<(Vec<u32>, usize), MmlError> {
    let mut vals = Vec::with_capacity(count);
    for k in 0..count {
        let (n, ni) = read_uint(chars, i, base, macros)?;
        i = ni;
        vals.push(n);
        if k + 1 < count {
            if i >= chars.len() || chars[i] != ',' {
                return Err(MmlError::at(base + i, "expected ',' between operands"));
            }
            i += 1;
        }
    }
    Ok((vals, i))
}

/// Locate the `}` matching the `{` at `open`, honouring nesting.
fn find_matching_brace(chars: &[char], open: usize, base: usize) -> Result<usize, MmlError> {
    let mut depth = 0i32;
    let mut k = open;
    while k < chars.len() {
        match chars[k] {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(k);
                }
            }
            _ => {}
        }
        k += 1;
    }
    Err(MmlError::at(base + open, "unclosed '{'"))
}

fn letter_semitone(c: char) -> i32 {
    match c {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => unreachable!("only called for note letters"),
    }
}

fn check_len(n: u32, start: usize) -> Result<(), MmlError> {
    if (1..=192).contains(&n) {
        Ok(())
    } else {
        Err(MmlError::at(start, "length must be 1–192"))
    }
}

fn check_127(vals: &[u32], start: usize) -> Result<(), MmlError> {
    if vals.iter().all(|&v| v <= 127) {
        Ok(())
    } else {
        Err(MmlError::at(start, "value must be 0–127"))
    }
}

// ---------------------------------------------------------------------------------------
// MML variables ($0–$7)
//
// `$n` operands are resolved to their current value during lexing (read_uint). Assigned
// values live in the same `macros` map under a reserved (NUL-prefixed) key so they survive
// lex recursion into macro bodies; `is_reserved_key` keeps them out of the macro namespace.
// ---------------------------------------------------------------------------------------

fn var_key(n: u8) -> String {
    format!("\0var{n}")
}

fn is_reserved_key(k: &str) -> bool {
    k.starts_with('\0')
}

fn set_var(macros: &mut MacroTable, n: u8, val: u32) {
    macros.insert(
        var_key(n),
        vec![Tok {
            kind: Tk::AbsPitch(val.min(255) as u8),
            off: 0,
        }],
    );
}

fn get_var(macros: &MacroTable, n: u8) -> u32 {
    match macros.get(&var_key(n)).and_then(|v| v.first()) {
        Some(Tok {
            kind: Tk::AbsPitch(v),
            ..
        }) => *v as u32,
        _ => 0,
    }
}

// ---------------------------------------------------------------------------------------
// Expansion — macros + finite repeats
// ---------------------------------------------------------------------------------------

/// Guard against runaway macro recursion / repeat blow-up.
const MAX_EXPANDED: usize = 4_000_000;
const MAX_MACRO_DEPTH: usize = 64;

fn expand(toks: &[Tok], macros: &MacroTable) -> Result<Vec<Tok>, MmlError> {
    let mut out = Vec::new();
    expand_into(toks, macros, &mut out, &mut Vec::new())?;
    Ok(out)
}

fn expand_into(
    toks: &[Tok],
    macros: &MacroTable,
    out: &mut Vec<Tok>,
    stack: &mut Vec<String>,
) -> Result<(), MmlError> {
    let mut i = 0;
    while i < toks.len() {
        let t = &toks[i];
        match &t.kind {
            Tk::MacroUse(name) => {
                let body = macros
                    .get(name)
                    .ok_or_else(|| MmlError::at(t.off, format!("undefined macro {{{name}}}")))?;
                if stack.iter().any(|s| s == name) || stack.len() >= MAX_MACRO_DEPTH {
                    return Err(MmlError::at(t.off, "macro recursion too deep"));
                }
                stack.push(name.clone());
                expand_into(body, macros, out, stack)?;
                stack.pop();
                i += 1;
            }
            Tk::LoopStart => {
                // Collect the matching repeat body, expanding nested loops/macros first.
                let (body, end_idx, count) = collect_loop(toks, i)?;
                let mut expanded_body = Vec::new();
                expand_into(body, macros, &mut expanded_body, stack)?;
                match count {
                    Some(n) => {
                        for _ in 0..n {
                            out.extend(expanded_body.iter().cloned());
                            if out.len() > MAX_EXPANDED {
                                return Err(MmlError::at(t.off, "repeat expansion too large"));
                            }
                        }
                    }
                    None => {
                        // Endless: keep one body between Loop markers for the synth to loop.
                        out.push(Tok {
                            kind: Tk::LoopStart,
                            off: t.off,
                        });
                        out.extend(expanded_body);
                        out.push(Tok {
                            kind: Tk::LoopEnd,
                            off: toks[end_idx].off,
                        });
                    }
                }
                i = end_idx + 1;
            }
            Tk::LoopEndFinite(_) | Tk::LoopEndInfinite => {
                return Err(MmlError::at(t.off, "']' without matching '['"));
            }
            _ => {
                out.push(t.clone());
                i += 1;
            }
        }
    }
    Ok(())
}

/// Given `toks[start] == LoopStart`, return the body slice, the index of the matching loop-end
/// token, and its repeat count (None = endless).
fn collect_loop(toks: &[Tok], start: usize) -> Result<(&[Tok], usize, Option<u32>), MmlError> {
    let mut depth = 0i32;
    let mut i = start;
    while i < toks.len() {
        match &toks[i].kind {
            Tk::LoopStart => depth += 1,
            Tk::LoopEndFinite(n) => {
                depth -= 1;
                if depth == 0 {
                    return Ok((&toks[start + 1..i], i, Some(*n)));
                }
            }
            Tk::LoopEndInfinite => {
                depth -= 1;
                if depth == 0 {
                    return Ok((&toks[start + 1..i], i, None));
                }
            }
            _ => {}
        }
        i += 1;
    }
    Err(MmlError::at(toks[start].off, "'[' without matching ']'"))
}

// ---------------------------------------------------------------------------------------
// Resolution — tokens → per-channel events
// ---------------------------------------------------------------------------------------

#[derive(Clone)]
struct ChannelState {
    octave: i32,
    default_divisor: u32,
    default_dots: u32,
    gate: u8,
    velocity: u8,
    invert: bool,
    tie_pending: bool,
    portamento_pending: bool,
    last_note_idx: Option<usize>,
    last_pitch: Option<u8>,
}

impl ChannelState {
    fn new() -> Self {
        // Defaults the synth (M5-T2) may refine; octave 4 / length 4 are documented (`O4C=60`),
        // gate 8 / velocity 127 are hypotheses — queued.
        ChannelState {
            octave: 4,
            default_divisor: 4,
            default_dots: 0,
            gate: 8,
            velocity: 127,
            invert: false,
            tie_pending: false,
            portamento_pending: false,
            last_note_idx: None,
            last_pitch: None,
        }
    }
}

fn resolve(toks: &[Tok]) -> Result<Song, MmlError> {
    let mut song = Song::new();
    let mut states: Vec<ChannelState> = (0..NUM_CHANNELS).map(|_| ChannelState::new()).collect();
    let mut cur = 0usize;

    for t in toks {
        let ch = &mut states[cur];
        let evs = &mut song.channels[cur];
        match &t.kind {
            Tk::Channel(n) => {
                cur = *n as usize;
            }
            Tk::Tempo(n) => evs.push(Event::Tempo(*n)),
            Tk::DefaultLen { divisor, dots } => {
                ch.default_divisor = *divisor;
                ch.default_dots = *dots;
            }
            Tk::Octave(n) => ch.octave = *n as i32,
            Tk::OctaveUp => ch.octave += if ch.invert { -1 } else { 1 },
            Tk::OctaveDown => ch.octave += if ch.invert { 1 } else { -1 },
            Tk::OctaveInvert => ch.invert = !ch.invert,
            Tk::Gate(n) => ch.gate = *n,
            Tk::Velocity(n) => ch.velocity = *n,
            Tk::Volume(n) => evs.push(Event::Volume(*n)),
            Tk::VolumeUp(n) => evs.push(Event::VolumeUp(*n)),
            Tk::VolumeDown(n) => evs.push(Event::VolumeDown(*n)),
            Tk::Pan(n) => evs.push(Event::Pan(*n)),
            Tk::Instrument(n) => evs.push(Event::Instrument(*n)),
            Tk::Envelope { a, d, s, r } => evs.push(Event::Envelope {
                a: *a,
                d: *d,
                s: *s,
                r: *r,
            }),
            Tk::EnvelopeReset => evs.push(Event::EnvelopeReset),
            Tk::Detune(n) => evs.push(Event::Detune(*n)),
            Tk::Tremolo {
                depth,
                range,
                speed,
                delay,
            } => evs.push(Event::Tremolo {
                depth: *depth,
                range: *range,
                speed: *speed,
                delay: *delay,
            }),
            Tk::Vibrato {
                depth,
                range,
                speed,
                delay,
            } => evs.push(Event::Vibrato {
                depth: *depth,
                range: *range,
                speed: *speed,
                delay: *delay,
            }),
            Tk::AutoPan {
                depth,
                range,
                speed,
                delay,
            } => evs.push(Event::AutoPan {
                depth: *depth,
                range: *range,
                speed: *speed,
                delay: *delay,
            }),
            Tk::ModulationOn => evs.push(Event::ModulationOn),
            Tk::ModulationOff => evs.push(Event::ModulationOff),
            Tk::LoopStart => evs.push(Event::LoopStart),
            Tk::LoopEnd => evs.push(Event::LoopEnd),
            Tk::Portamento => ch.portamento_pending = true,
            Tk::Tie => ch.tie_pending = true,
            Tk::TieLength { len, dots } => {
                let dur = note_ticks(Some(*len), *dots, ch, t.off)?;
                extend_last(ch, evs, dur, t.off)?;
            }
            Tk::Rest { len, dots } => {
                let dur = note_ticks(*len, *dots, ch, t.off)?;
                evs.push(Event::Rest { duration: dur });
                ch.tie_pending = false;
                ch.portamento_pending = false;
                ch.last_note_idx = None;
                ch.last_pitch = None;
            }
            Tk::Note {
                semitone,
                accidental,
                len,
                dots,
            } => {
                let pitch = clamp_midi((ch.octave + 1) * 12 + semitone + accidental);
                let dur = note_ticks(*len, *dots, ch, t.off)?;
                emit_note(ch, evs, pitch, dur);
            }
            Tk::AbsPitch(n) => {
                // N has no length operand — it always uses the default length.
                let dur = note_ticks(None, 0, ch, t.off)?;
                emit_note(ch, evs, *n, dur);
            }
            Tk::MacroUse(_) | Tk::LoopEndFinite(_) | Tk::LoopEndInfinite => {
                // Removed during expansion.
                return Err(MmlError::at(t.off, "internal: unexpanded token"));
            }
        }
    }
    Ok(song)
}

/// Emit a note, honouring a pending tie/slur and portamento.
fn emit_note(ch: &mut ChannelState, evs: &mut Vec<Event>, pitch: u8, dur: u32) {
    if ch.tie_pending {
        ch.tie_pending = false;
        if ch.last_pitch == Some(pitch) {
            // Same pitch → true tie: extend the held note.
            if let Some(idx) = ch.last_note_idx {
                if let Event::Note { duration, .. } = &mut evs[idx] {
                    *duration += dur;
                    ch.portamento_pending = false;
                    return;
                }
            }
        }
        // Different pitch → slur: a distinct, legato note.
        let porta = ch.portamento_pending;
        ch.portamento_pending = false;
        evs.push(Event::Note {
            pitch,
            duration: dur,
            gate: ch.gate,
            velocity: ch.velocity,
            slur: true,
            portamento: porta,
        });
        ch.last_note_idx = Some(evs.len() - 1);
        ch.last_pitch = Some(pitch);
        return;
    }
    let porta = ch.portamento_pending;
    ch.portamento_pending = false;
    evs.push(Event::Note {
        pitch,
        duration: dur,
        gate: ch.gate,
        velocity: ch.velocity,
        slur: false,
        portamento: porta,
    });
    ch.last_note_idx = Some(evs.len() - 1);
    ch.last_pitch = Some(pitch);
}

/// Extend the last note by `dur` (short tie form `&8`); requires a prior note.
fn extend_last(
    ch: &mut ChannelState,
    evs: &mut [Event],
    dur: u32,
    off: usize,
) -> Result<(), MmlError> {
    ch.tie_pending = false;
    match ch.last_note_idx.and_then(|idx| evs.get_mut(idx)) {
        Some(Event::Note { duration, .. }) => {
            *duration += dur;
            Ok(())
        }
        _ => Err(MmlError::at(off, "tie '&' without a preceding note")),
    }
}

/// Note duration in ticks. An explicit length fully overrides the default (its own dots
/// apply); a length-less note uses the channel's default divisor and combines the default's
/// dots with any dots written on the note itself (so `L2.` then `C.` double-dots).
fn note_ticks(len: Option<u32>, dots: u32, ch: &ChannelState, off: usize) -> Result<u32, MmlError> {
    let (divisor, total_dots) = match len {
        Some(l) => (l, dots),
        None => (ch.default_divisor, ch.default_dots + dots),
    };
    check_len(divisor, off)?;
    let base = TICKS_PER_WHOLE_NOTE / divisor;
    let mut total = base;
    let mut add = base;
    for _ in 0..total_dots {
        add /= 2;
        total += add;
    }
    Ok(total)
}

fn clamp_midi(key: i32) -> u8 {
    key.clamp(0, 127) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch0(mml: &str) -> Vec<Event> {
        parse(mml).unwrap().channels[0].clone()
    }

    fn note(pitch: u8, duration: u32) -> Event {
        Event::Note {
            pitch,
            duration,
            gate: 8,
            velocity: 127,
            slur: false,
            portamento: false,
        }
    }

    #[test]
    fn empty_is_silent() {
        assert_eq!(parse("").unwrap().channels[0], Vec::<Event>::new());
    }

    #[test]
    fn middle_c_is_60() {
        // Default O4, default length L4 → quarter = 48 ticks.
        assert_eq!(ch0("C"), vec![note(60, 48)]);
    }

    #[test]
    fn note_scale_and_lengths() {
        // C D E F G A B at O4, default quarter.
        let evs = ch0("CDEFGAB");
        let pitches: Vec<u8> = evs
            .iter()
            .map(|e| match e {
                Event::Note { pitch, .. } => *pitch,
                _ => panic!(),
            })
            .collect();
        assert_eq!(pitches, vec![60, 62, 64, 65, 67, 69, 71]);
    }

    #[test]
    fn per_note_length_and_default_length() {
        // C8 = eighth = 24 ticks; L8 then C = 24.
        assert_eq!(ch0("C8"), vec![note(60, 24)]);
        assert_eq!(ch0("L8C"), vec![note(60, 24)]);
        // C1 whole = 192, C4 quarter = 48, C16 = 12.
        assert_eq!(ch0("C1"), vec![note(60, 192)]);
        assert_eq!(ch0("C16"), vec![note(60, 12)]);
    }

    #[test]
    fn dotted_default_length() {
        // L2. = dotted half default (96 + 48 = 144); a following C inherits it (corpus form).
        assert_eq!(ch0("L2.C"), vec![note(60, 144)]);
        // A length-less note may add further dots on top of the default's dots.
        assert_eq!(ch0("L4.C"), vec![note(60, 72)]); // dotted quarter = 48 + 24
    }

    #[test]
    fn dotted_notes() {
        // C4. = 48 + 24 = 72; C8. = 24 + 12 = 36; C4.. = 48 + 24 + 12 = 84.
        assert_eq!(ch0("C4."), vec![note(60, 72)]);
        assert_eq!(ch0("C8."), vec![note(60, 36)]);
        assert_eq!(ch0("C4.."), vec![note(60, 84)]);
    }

    #[test]
    fn accidentals() {
        assert_eq!(ch0("C#"), vec![note(61, 48)]);
        assert_eq!(ch0("C+"), vec![note(61, 48)]);
        assert_eq!(ch0("C-"), vec![note(59, 48)]);
        assert_eq!(ch0("E#"), vec![note(65, 48)]); // E# == F pitch
    }

    #[test]
    fn leading_accidentals() {
        // Corpus form: accidental BEFORE the note (`+B`, `#F`, `-C`) — same pitch as trailing.
        assert_eq!(ch0("+C"), ch0("C+"));
        assert_eq!(ch0("+C"), vec![note(61, 48)]);
        assert_eq!(ch0("-D"), vec![note(61, 48)]); // D-flat = 61
        assert_eq!(ch0("#F8"), vec![note(66, 24)]); // F# eighth
                                                    // Leading accidental with no following note is illegal.
        assert_eq!(parse("+R").unwrap_err().errnum, ERRNUM_ILLEGAL_MML);
    }

    #[test]
    fn octave_absolute_and_relative() {
        assert_eq!(ch0("O5C"), vec![note(72, 48)]);
        assert_eq!(ch0("O3C"), vec![note(48, 48)]);
        assert_eq!(ch0("C>C"), vec![note(60, 48), note(48, 48)]); // > is octave DOWN
        assert_eq!(ch0("C<C"), vec![note(60, 48), note(72, 48)]); // < is octave UP
    }

    #[test]
    fn octave_invert() {
        // After '!', < and > swap meaning.
        assert_eq!(ch0("!C<C"), vec![note(60, 48), note(48, 48)]);
    }

    #[test]
    fn rest() {
        assert_eq!(ch0("R4"), vec![Event::Rest { duration: 48 }]);
        assert_eq!(ch0("R"), vec![Event::Rest { duration: 48 }]);
    }

    #[test]
    fn abs_pitch_key_number() {
        assert_eq!(ch0("N60"), vec![note(60, 48)]);
        assert_eq!(ch0("N0"), vec![note(0, 48)]);
        assert_eq!(ch0("N127"), vec![note(127, 48)]);
    }

    #[test]
    fn tie_same_pitch_merges() {
        // C4&C8 = one note held 48 + 24 = 72 ticks.
        assert_eq!(ch0("C4&C8"), vec![note(60, 72)]);
    }

    #[test]
    fn tie_short_form() {
        // C4&8 ≡ C4&C8.
        assert_eq!(ch0("C4&8"), vec![note(60, 72)]);
    }

    #[test]
    fn slur_different_pitch_keeps_two() {
        // C4&D8 → two notes, the second slurred.
        let evs = ch0("C4&D8");
        assert_eq!(evs.len(), 2);
        assert_eq!(evs[0], note(60, 48));
        assert_eq!(
            evs[1],
            Event::Note {
                pitch: 62,
                duration: 24,
                gate: 8,
                velocity: 127,
                slur: true,
                portamento: false
            }
        );
    }

    #[test]
    fn portamento_flag() {
        let evs = ch0("C_D");
        assert_eq!(
            evs[1],
            Event::Note {
                pitch: 62,
                duration: 48,
                gate: 8,
                velocity: 127,
                slur: false,
                portamento: true
            }
        );
    }

    #[test]
    fn gate_and_velocity_ride_on_note() {
        let evs = ch0("Q4@V96C");
        assert_eq!(
            evs,
            vec![Event::Note {
                pitch: 60,
                duration: 48,
                gate: 4,
                velocity: 96,
                slur: false,
                portamento: false
            }]
        );
    }

    #[test]
    fn control_events() {
        assert_eq!(ch0("T120"), vec![Event::Tempo(120)]);
        assert_eq!(ch0("T512"), vec![Event::Tempo(512)]);
        assert_eq!(ch0("V80"), vec![Event::Volume(80)]);
        assert_eq!(ch0("("), vec![Event::VolumeUp(1)]);
        assert_eq!(ch0(")"), vec![Event::VolumeDown(1)]);
        // Corpus form: `(N` / `)N` change volume by N steps.
        assert_eq!(ch0("(3"), vec![Event::VolumeUp(3)]);
        assert_eq!(ch0(")24"), vec![Event::VolumeDown(24)]);
        assert_eq!(ch0("P64"), vec![Event::Pan(64)]);
        assert_eq!(ch0("@7"), vec![Event::Instrument(7)]);
        assert_eq!(ch0("@0"), vec![Event::Instrument(0)]);
        assert_eq!(ch0("@128"), vec![Event::Instrument(128)]);
        assert_eq!(ch0("@256"), vec![Event::Instrument(256)]);
        assert_eq!(ch0("@267"), vec![Event::Instrument(267)]); // corpus SFX bank
    }

    #[test]
    fn envelope_and_reset() {
        assert_eq!(
            ch0("@E127,100,30,100"),
            vec![Event::Envelope {
                a: 127,
                d: 100,
                s: 30,
                r: 100
            }]
        );
        assert_eq!(ch0("@ER"), vec![Event::EnvelopeReset]);
    }

    #[test]
    fn detune_signed() {
        assert_eq!(ch0("@D5"), vec![Event::Detune(5)]);
        assert_eq!(ch0("@D-5"), vec![Event::Detune(-5)]);
        assert_eq!(ch0("@D127"), vec![Event::Detune(127)]);
        assert_eq!(ch0("@D-128"), vec![Event::Detune(-128)]);
    }

    #[test]
    fn lfos_and_modulation() {
        assert_eq!(
            ch0("@MA64,1,16,32"),
            vec![Event::Tremolo {
                depth: 64,
                range: 1,
                speed: 16,
                delay: 32
            }]
        );
        assert_eq!(
            ch0("@MP64,1,16,32"),
            vec![Event::Vibrato {
                depth: 64,
                range: 1,
                speed: 16,
                delay: 32
            }]
        );
        assert_eq!(
            ch0("@ML100,1,8,0"),
            vec![Event::AutoPan {
                depth: 100,
                range: 1,
                speed: 8,
                delay: 0
            }]
        );
        assert_eq!(ch0("@MON"), vec![Event::ModulationOn]);
        assert_eq!(ch0("@MOF"), vec![Event::ModulationOff]);
    }

    #[test]
    fn channels_split() {
        let song = parse("T120:0CCC:1EEE:2GGG").unwrap();
        assert_eq!(
            song.channels[0],
            vec![Event::Tempo(120), note(60, 48), note(60, 48), note(60, 48)]
        );
        assert_eq!(
            song.channels[1],
            vec![note(64, 48), note(64, 48), note(64, 48)]
        );
        assert_eq!(
            song.channels[2],
            vec![note(67, 48), note(67, 48), note(67, 48)]
        );
        assert!(song.channels[3].is_empty());
    }

    #[test]
    fn finite_repeat() {
        // [CDE]2 → CDE CDE.
        let evs = ch0("[CDE]2");
        assert_eq!(
            evs,
            vec![
                note(60, 48),
                note(62, 48),
                note(64, 48),
                note(60, 48),
                note(62, 48),
                note(64, 48)
            ]
        );
    }

    #[test]
    fn nested_repeat_doc_example() {
        // [[CCC]2DEF]2 → CCC CCC DEF CCC CCC DEF.
        let evs = ch0("[[C]2D E]2");
        let pitches: Vec<u8> = evs
            .iter()
            .map(|e| match e {
                Event::Note { pitch, .. } => *pitch,
                _ => panic!(),
            })
            .collect();
        // C C D E  C C D E
        assert_eq!(pitches, vec![60, 60, 62, 64, 60, 60, 62, 64]);
    }

    #[test]
    fn endless_repeat_uses_markers() {
        // Bare ] → not unrolled, wrapped in Loop markers.
        let evs = ch0("[C]");
        assert_eq!(evs, vec![Event::LoopStart, note(60, 48), Event::LoopEnd]);
    }

    #[test]
    fn macro_define_and_use() {
        // Define a macro then use it; expansion is inline.
        let evs = ch0("{M=CDE}{M}{M}");
        let pitches: Vec<u8> = evs
            .iter()
            .map(|e| match e {
                Event::Note { pitch, .. } => *pitch,
                _ => panic!(),
            })
            .collect();
        assert_eq!(pitches, vec![60, 62, 64, 60, 62, 64]);
    }

    #[test]
    fn macro_in_repeat_doc_example() {
        // T240@128O2{PT0=CDEDCDE<G}[{PT0}]4 — must parse; PT0 expanded 4×.
        let song = parse("T240@128O2{PT0=CDEDCDE<G}[{PT0}]4").unwrap();
        let notes = song.channels[0]
            .iter()
            .filter(|e| matches!(e, Event::Note { .. }))
            .count();
        // 8 notes per macro body × 4 repeats.
        assert_eq!(notes, 32);
    }

    #[test]
    fn mml_variable_substitution() {
        // $0=64 V$0 ≡ V64.
        assert_eq!(ch0("$0=64 V$0"), vec![Event::Volume(64)]);
        // Variable used as a length.
        assert_eq!(ch0("$1=8 C$1"), vec![note(60, 24)]);
    }

    #[test]
    fn whitespace_ignored() {
        // :0 (channel) @7 (instrument) V80 (volume) O4 (octave) G16 C16 E8 → Instrument, Volume, 3 notes.
        assert_eq!(ch0(":0 @7 V80 O4 G16C16E8").len(), 5);
    }

    #[test]
    fn case_insensitive() {
        assert_eq!(ch0("cde"), ch0("CDE"));
    }

    // ---- error cases: every one is errnum 47 ----

    fn err(mml: &str) -> MmlError {
        parse(mml).unwrap_err()
    }

    #[test]
    fn errors_are_47() {
        for bad in [
            "X",            // unknown command
            "Z9",           // unknown command
            "T0",           // tempo below range
            "T999",         // tempo above range
            "O9",           // octave above range
            ":16",          // channel above range
            "Q9",           // gate above range
            "V200",         // volume above range
            "N200",         // key above range
            "@",            // dangling @
            "@E1,2,3",      // too few envelope operands
            "@MA1,2,3",     // too few LFO operands
            "@MOX",         // bad modulation
            "[CDE",         // unclosed repeat
            "CDE]2",        // unmatched repeat end
            "{M}",          // undefined macro use
            "{M=CDE}{M=D}", // macro redefined
            "{M=:0C}",      // channel inside macro
            "$8=1",         // var index out of range
            "$0",           // bare var without assignment
            "/comment/",    // SB4-only comments rejected
            "|CEG|",        // SB4-only chords rejected
        ] {
            let e = err(bad);
            assert_eq!(
                e.errnum, ERRNUM_ILLEGAL_MML,
                "input {bad:?} should be errnum 47"
            );
        }
    }

    #[test]
    fn tie_without_note_errors() {
        // "&8" — short tie form with no preceding note.
        assert_eq!(err("&8").errnum, ERRNUM_ILLEGAL_MML);
    }

    #[test]
    fn error_offset_points_at_failure() {
        // "CC X" — the X is at char offset 3.
        let e = err("CCX");
        assert_eq!(e.offset, 2);
    }

    #[test]
    fn corpus_like_program_parses() {
        // A realistic mix of constructs seen in the corpus.
        let mml = ":0@7V80O4Q5L8 G16C16E8 @D-5@E113,102,0,118 [CDEFGAB>C]2 :1@128O2{R=CDED}[{R}]4";
        assert!(parse(mml).is_ok());
    }

    #[test]
    fn real_corpus_strings_parse() {
        // Verified-legal forms lifted from real shipped SB3 programs (sbsave corpus). These
        // PROVE the syntax is legal even where the docs omit it (output unproven → oracle).
        let cases = [
            // 1DVK34J/HNZHC etc — `(N`/`)N` volume-step crescendo via ties.
            "T220@148O5Q8V80L16CE-G<L8C&)16C&)16C&(16C&(16C1.",
            // forward-referenced macros: `{b}` used in PAI before `{b=…}` is defined.
            "{PAI=V100L16{b}RRR}:0{PAI}{b=@266V80P70O2C}",
            // high SFX-bank instruments + LFO + @V + @D, all together.
            "T50:0@296@V100@E127,54,54,27 O4 Q6 L40 <A>A<DA<D L64)80P0>DA<D )20P127>A<D",
            // `)24D&(3D` swell pattern from a macro body.
            "{CD=)24D&(3D&(3D}:0{CD}",
            // EK3E9F/PARKOUR BGMSET 145 — leading accidentals (`+B`), dotted lengths, big macros.
            "T140{M0=V67L4[A1&A2A8.B8.+B8A1&A2A8.B8.+B8A2A8.<C8.D8E8.D8.C8>B8.A8.G8]2}:0@80O5{M0}",
        ];
        for mml in cases {
            assert!(parse(mml).is_ok(), "should parse: {mml}");
        }
    }
}
