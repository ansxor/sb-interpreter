---
title: FORMAT$
slug: docs-sb4-format
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-format
content_id: 19490
created: 2020-10-28
scraped: 2026-06-21
---

# FORMAT$

Format values into a string using specified format codes.

## Syntax

```sbsyntax
FORMAT$ template$ {, args... } OUT result$
```

| Input | Description |
| --- | --- |
| `template$` | A string containing format codes used to format the arguments. |
| `args...` | Values to be formatted into `template$`, corresponding to the format codes. |

| Output | Description |
| --- | --- |
| `result$` | The formatted result string. |

There must be as many arguments as there are format codes (not counting `%%`), and each argument must have an appropriate type for its format code.

## Format Codes

`FORMAT$` uses special format code syntax within the argument string. A format code starts with `%` and consists of multiple parts:  a type specification, a length, and some number of modifiers.

| | |
| --- | --- |
| 0 +- | Modifiers |
| 5 | Length |
| .1 | Precision |
| F | Type |

### Types

The following formatting types are available:

| Format Type | Value Type | Description |
| --- | --- | --- |
| `F` | Real | Real number. By default, displays six decimal digits. |
| `D` | Integer | Integer in decimal. |
| `X` | Integer | Integer in hex. |
| `B` | Integer | Integer in binary. |
| `S` | String | String. |

### Length

Writing a number before the type specifier, e.g. `%5F` forces the formatted value to occupy *at least* that many characters. This is done by inserting spaces or zeros at the beginning or end, depending on format modifiers. If the formatted value is longer than the specified length, it is allowed to go over the length. The length is a *minimum*, not a fixed value.

### Precision

When using `%F` or `%S`, you can specify a precision value using a period. This has no effect on other format codes.

For `%F`, the precision affects the number of fractional digits the value is rounded to, e.g. `%.2F` will display the number rounded to 2 decimal places. If omitted, it defaults to 6. Using 0 rounds the number to the nearest whole.

For `%S`, the string will be truncated to the length given in the precision before formatting, e.g `%.2S` will truncate `"ABCDE"` to just `"AB"` regardless of the length parameter.

If a period is written but no number is written after it, e.g. `%.S`, the precision is 0.

### Modifiers

Before the length, a few modifier symbols can be used to change the formatting of the value.

| Modifier | Description | Affects |
| --- | --- | --- |
| `+` | Show positive numbers and 0 starting with a plus sign. | `%F`, `%D` |
| `-` | Left-align the value to its length, instead of right-align. | All |
| `0` | Display leading zeros when right-aligning. | All |
| ` ` (space) | Reserve a space where the sign would go. | `%F`, `%D` |

### Escaping %

To include a `%` in the output, the `%%` format code must be used.

## Examples

```sb4
'print pi to 2 decimal places
PRINT FORMAT$("%.2F",#PI)
```

```sb4
'show progress percentage
PRINT FORMAT$("CURRENT PROGRESS: %D%%",PROGRESS)
```

```sb4
'announce the final score
DIALOG FORMAT$("GAME OVER. P1 SCORE: %D. P2 SCORE: %D.",SCORE1,SCORE2)
```

```sb4
'display a complex error message
STOP FORMAT$("[%S] A fatal error has occurred (%D:%D)",#_FILENAME,#_SLOT,#_LINE)
```

```sb4
'force a string to a specific length
PRINT FORMAT$("%3.3S","Quick brown")
```
