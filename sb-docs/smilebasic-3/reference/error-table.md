---
title: Error Table
slug: docs-sb3-error-table
system: SmileBASIC 3
type: reference
category: Error Table
source: InstructionList.pdf
scraped: 2026-06-21
---

# Error Table

If an error has occurred, the relevant information is stored in the system variables ERRNUM (error number) and ERRLINE (the line where the error occurred).

| Number | Error | Description |
| --- | --- | --- |
| 3 | Syntax error | syntax does not follow the grammar rules |
| 4 | Illegal function call | the number of arguments specified in an instruction or function is wrong |
| 5 | Stack overflow | an overflow has occurred in the stack |
| 6 | Stack underflow | an underflow has occurred in the stack |
| 7 | Divide by zero | division by zero was attempted |
| 8 | Type mismatch | an inconsistent variable type is specified |
| 9 | Overflow | the calculation result exceeded the allowed range |
| 10 | Out of range | a value outside the allowed range was specified |
| 11 | Out of memory | sufficient memory area is not available |
| 12 | Out of code memory | sufficient code memory area is not available |
| 13 | Out of DATA | DATA that can be READ is insufficient |
| 14 | Undefined label | the specified label could not be found |
| 15 | Undefined variable | the specified variable could not be found |
| 16 | Undefined function | the specified instruction/function could not be found |
| 17 | Duplicate label | the same label has been defined twice |
| 18 | Duplicate variable | the same variable has been defined twice |
| 19 | Duplicate function | the same instruction/function has been defined twice |
| 20 | FOR without NEXT | a FOR has no NEXT |
| 21 | NEXT without FOR | a NEXT has no FOR |
| 22 | REPEAT without UNTIL | a REPEAT has no UNTIL |
| 23 | UNTIL without REPEAT | an UNTIL has no REPEAT |
| 24 | WHILE without WEND | a WHILE has no WEND |
| 25 | WEND without WHILE | a WEND has no WHILE |
| 26 | THEN without ENDIF | a THEN has no ENDIF |
| 27 | ELSE without ENDIF | an ELSE has no ENDIF |
| 28 | ENDIF without IF | an ENDIF has no IF |
| 29 | DEF without END | a DEF has no END |
| 30 | RETURN without GOSUB | a RETURN has no GOSUB |
| 31 | Subscript out of range | array subscripts are not within the allowed range |
| 32 | Nested DEF | a DEF has been defined within another DEF |
| 33 | Can't continue | the program cannot resume with CONT |
| 34 | Illegal symbol string | a label string has been incorrectly described |
| 35 | Illegal file format | the file is in a format that SmileBASIC cannot support |
| 36 | Mic is not available | a microphone instruction was used without using XON MIC |
| 37 | Motion sensor is not available | a motion instruction was used without using XON MOTION |
| 38 | Use PRGEDIT before any PRG function | one of the PRG instructions was used without using PRGEDIT |
| 39 | Animation is too long | animation definition is too long |
| 40 | Illegal animation data | animation data is incorrect |
| 41 | String too long | string is too long |
| 42 | Communication buffer overflow | an overflow has occurred in the buffer for sending MPSEND |
| 43 | Can't use from DIRECT mode | an instruction that does not work in DIRECT mode was used |
| 44 | Can't use in program | an instruction that cannot be used in a program was used |
| 45 | Can't use in tool program | an instruction that cannot be used from a tool program was used |
| 46 | Load failed | failed to read the file |
| 47 | Illegal MML | the MML [Music Macro Language] is incorrect |
