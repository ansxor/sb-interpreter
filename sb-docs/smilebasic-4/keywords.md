---
title: Keyword Table
slug: docs-sb4-keywords
system: SmileBASIC 4
type: reference
source: https://smilebasicsource.com/forum/thread/docs-sb4-keywords
content_id: 19453
created: 2020-11-18
scraped: 2026-06-21
---

# Keyword Table

A *keyword* is a special identifier reserved by the language syntax. They are used for control structures and special language features. A keyword cannot be used as a name for a variable or user-defined function in most cases.

> *Note:* do not confuse keywords with operators. The "word" operators `DIV` `MOD` `AND` `OR` `XOR` and `NOT` are also treated as keywords, but are listed in the [Operator Table](https://smilebasicsource.com/forum/thread/3164).

## Declarations/Variables

| Keyword | Description |
| --- | --- |
| `VAR` | Declares a variable or array. |
| `DIM` | Declares a variable or array. |
| `VAR()` | Refer to a variable given its name as a string. |
| `DIM()` | Check the dimensions of an array. |
| `DEF` | Starts a function definition. |
| `COMMON` | Comes before `DEF` to mark this function as available to all slots. |
| `INC` | Increment a variable or append to a string. |
| `DEC` | Decrement a variable. |
| `SWAP` | Swaps the values of two variables. |
| `DEFOUT` | Set a return value by position. Only usable within user-defined functions. |
| `OUT` | Specifies the return value list of a function. |
| `CONST` | Declare a constant. |
| `ENUM` | Declare an enum. |

## Control Structures

### General

| Keyword | Description |
| --- | --- |
| `END` | Ends the program, or marks the end of a function definition. |
| `RETURN` | Returns from a `GOSUB` or `DEF`.<br>Within expression-style `DEF`, `RETURN` accepts a return value. |
| `EXEC` | Executes a program or slot. |
| `CALL` | Call a function given its name and parameters. |
| `BREAK` | Exit the loop immediately.<br>When used with `ON BREAK GOTO`, specify the label to `GOTO` when Plus is pressed. |
| `CONTINUE` | Skip to the next iteration of the loop. |

### IF Statements

| Keyword | Description |
| --- | --- |
| `IF` | Starts an `IF` statement. |
| `THEN` | Comes after the condition in an `IF` or `ELSEIF` (except when using `GOTO`.) |
| `ELSEIF` | Starts an `ELSEIF` branch in a multi-line `IF`. |
| `ELSE` | Starts the `ELSE` section of an `IF` statement. |
| `ENDIF` | Ends a multi-line `IF` statement. |

### CASE Statements

| Keyword | Description |
| --- | --- |
| `CASE` | Starts a `CASE` statement, specifying the case value. |
| `WHEN` | Specifies a possible match for the `CASE` statement. |
| `OTHERWISE` | Specifies the path to take when nothing matches (or, matches anything.) |
| `ENDCASE` | Ends the `CASE` statement. |

### LOOP

| Keyword | Description |
| --- | --- |
| `LOOP` | Starts a `LOOP`. |
| `ENDLOOP` | Ends a `LOOP`. |

### FOR Loop

| Keyword | Description |
| --- | --- |
| `FOR` | Starts a `FOR` loop. |
| `TO` | Specifies the end value.<br>`TO` is only a keyword when used in the context of a `FOR` loop. |
| `STEP` | Specifies the step value.<br>`STEP` is only a keyword when used in the context of a `FOR` loop. |
| `NEXT` | Ends a `FOR` loop. |

### WHILE Loop

| Keyword | Description |
| --- | --- |
| `WHILE` | Starts a `WHILE` loop, specifying the loop condition. |
| `WEND` | Ends a `WHILE` loop. |

### REPEAT Loop

| Keyword | Description |
| --- | --- |
| `REPEAT` | Starts a `REPEAT` loop. |
| `UNTIL` | Ends a `REPEAT` loop, specifying the loop condition. |

### GOTO

| Keyword | Description |
| --- | --- |
| `GOTO` | Jump to the given label. |
| `GOSUB` | Jump to the given label, as a subroutine.<br>The label can be returned from using `RETURN`. |
| `ON` | Starts an `ON` statement. |

## Text Screen / Console

| Keyword | Description |
| --- | --- |
| `PRINT` | Print values on the console text screen. |
| `?` | Print values on the console text screen. |
| `TPRINT` | Print values on a text screen. |
| `T?` | Print values on a text screen. |
| `INPUT` | Read values from keyboard input on the console screen. |
| `LINPUT` | Read a line of keyboard input on the console screen. |
| `??` | Special alias of the `INSPECT` function. |

## DATA

| Keyword | Description |
| --- | --- |
| `DATA` | Starts a `DATA` statement. |
| `READ` | Read values from the current `DATA` position. |
| `RESTORE` | Set/push/pop the `DATA` read position. |

## Other

| Keyword | Description |
| --- | --- |
| `REM` | Starts a comment. |
