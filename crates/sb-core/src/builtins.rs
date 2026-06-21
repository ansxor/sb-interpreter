//! Builtin commands & functions (milestone M1+).
//!
//! Registration is data-driven: a native function's signature determines its
//! argument/return [`crate::value::ValueType`]s and overload table — echoing the
//! compile-time reflection in `osb/SMILEBASIC/builtinfunctions.d` (`static this()`).
//! Each builtin's contract is mirrored by a YAML spec under `spec/instructions/`.

// TODO(M1): the registration macro + the first builtins (math/string/control/PRINT).
