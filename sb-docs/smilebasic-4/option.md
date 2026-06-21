---
title: OPTION
slug: docs-sb4-option
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-option
content_id: 19474
created: 2020-11-12
scraped: 2026-06-21
---

# OPTION

Set language options relating to variable declarations.

## Syntax

```sbsyntax
OPTION STRICT | DEFINT
```

| Option | Description |
| --- | --- |
| `STRICT` | All variables must be declared with `VAR` or `DIM` before they are used. Variables are not automatically declared by the compiler. |
| `DEFINT` | Unsuffixed variables and arrays are integer typed by default, instead of real number typed. |

An `OPTION` statement can only contain one option, but multiple `OPTION` statements may be used.

## Examples

```sb4
'require explicit declaration of variables
OPTION STRICT
VAR A=1  '← this line is fine
B=2      '← this is an error! `Undefined variable`, must use `VAR` or `DIM`
```

```sb4
'unsuffixed variables are ints by default
OPTION DEFINT
VAR A
INSPECT A  'INT: 0
```

## Notes

### Position-Dependent

The `OPTION` statement only takes effect /after/ its position in the source code; it is not a global statement, it relies on order. This can become confusing, for multiple reasons.

In the case of `OPTION STRICT`, variables before do not need to be explicitly declared, but variables after do.

```sb4
A=1
OPTION STRICT
B=2  '← error occurs HERE, not line 1
```

In the case of `OPTION DEFINT`, unsuffixed variables declared before the statement are reals, and ones declared afterward are integers.

```sb4
VAR A
OPTION DEFINT
VAR B

INSPECT A
INSPECT B
```

SmileBASIC 4 is dynamically typed, so this only affects the type of the initial value (or the type of the array, in the case of an array declaration) but it can still be confusing.

To avoid these issues, `OPTION` statements should always be the *first* statements in your program.

### Use of STRICT

The use of `OPTION STRICT`, sometimes called "strict mode", is highly encouraged for any program beyond a simple example or scratch file. Giving the parser the ability to detect uninitialized variables, instead of just having it automatically declare them when they are encountered, helps to avoid a variety of bugs. Plus, explicit declaration of variables is good code style, and even sometimes required for local variables in functions, etc.
