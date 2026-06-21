---
title: Function overview
slug: docs-ptc-function
system: Petit Computer
type: reference
source: https://smilebasicsource.com/forum/thread/docs-ptc-function
content_id: 19764
created: 2025-08-02
scraped: 2026-06-21
---

# Function overview

Functions are used to perform calculations or to read values. The distinction of a function from a command is the single returned value.

Functions can be passed zero or more arguments, with each argument being separated by a comma. If no arguments are passed, the closing parenthesis is placed immediately after the opening parenthesis.

Functions have the highest level of precedence aside from parenthesized statements. See [Operator overview](https://smilebasicsource.com/forum/thread/docs-ptc-operator).

## Function table

### Mathematical functions

|`ABS(number)`| Calculates the absolute value of a number.|
| --- | --- |
|`ATAN(tangent)`| Calculates the arctangent of a value. Returns results in radians.|
|`ATAN(y, x)`| Calculates the angle from the X-axis of a coordinate pair in radians.|
|`COS(angle)`| Calculates the cosine of the given angle. `angle` is in radians.|
|`DEG(radians)`| Converts an angle in radians into an angle in degrees.|
|`EXP(exponent)`| Calculates e raised to the power of `exponent`|
|`FLOOR(number)`| Rounds number to the next lowest integer|
|`LOG(number)`| Calculates the natural logarithm of the number|
|`PI()`| Returns the approximate value of pi. Returns 12867/4096, or about 3.141.|
|`POW(base, exponent)`| Calculates the value of `base` raised to the power of `exponent`.|
|`RAD(degrees)`| Converts an angle in degrees into an angle in radians.|
|`RND(maximum)`| Generates a pseudorandom integer between 0 and `maximum`|
|`SGN(number)`| Calculates the sign of the number.|
|`SIN(angle)`| Calculates the sine of the given angle. `angle` is given in radians.|
|`SQR(number)`| Calculates the square root of the number.|
|`TAN(angle)`| Calculates the tangent of given angle. `angle` is given in radians.|

### String functions

|`ASC(string)`| Returns the character code of the first character in the string.|
| --- | --- |
|`CHR$(code)`| Returns a single character string corresponding to the character code.|
|`HEX$(number)`| Converts a number into a hexadecimal string representation.|
|`HEX$(number,digits)`| Converts a number into a hexadecimal string represnetation with a specific number of digits|
|`INSTR(string, substring)`| Searches within a string for a substring, returning the index if found, or -1 if not found|
|`INSTR(string, substring, start)`| Searches within a string for a substring starting from index `start`, returning the index `substring` is found at, or -1 if not found.|
|`LEFT$(string, length)`| Returns a substring consisting of the `length` left-most characters|
|`LEN(string)`| Returns the number of characters in the string|
|`MID$(string, start, length)`| Returns a substring consisting of the `length` characters starting from index `start`.|
|`RIGHT$(string, length)`| Returns a substring consisting of the `length` right-most characters.|
|`STR$(number)`| Converts a number into a decimal string representation.|
|`SUBST$(string, start, count, replacement)`| Returns a new string, replacing part of `string` with `replacement`.|
|`VAL(string)`| Converts a string into a number.|

### System functions

These are functions that interact with other subsystems of Petit Computer, such as the graphics or audio systems.

### Background
|`BGCHK(layer)`| Checks if the BG layer specified is currently animated.|
| --- | --- |

### Audio
|`BGMCHK()`| Checks if background music is playing on track 0.|
| --- | --- |
|`BGMCHK(track)`| Checks if a specific background music track is in use.|
|`BGMGETV(var)`| Gets the value of a MML variable.|
|`TALKCHK()`| Checks if the speech synthesis system is currently active. On English copies of Petit Computer, always returns 0.|

### User input
|`BTRIG()`| Checks if a button is pressed, with repeat checking. Identical to BUTTON(1).|
| --- | --- |
|`BUTTON()`| Checks if a button is currently pressed.|
|`BUTTON(type)`| Checks buttons pressed, held, or released depending on type.|
|`INKEY$()`| Retrieve character input from the keyboard|

### Console
|`CHKCHR(x, y)`| Gets the character code of a location on the text console.|
| --- | --- |

### Graphics
|`GSPOIT(x, y)`| Gets the color of a location on the graphics page|
| --- | --- |
|`GSPOIT(x, y, page)`| Gets the color of a location on a specific graphics page|

### Panel
|`ICONCHK()`| Checks if a panel icon is pressed|
| --- | --- |

### Sprite
|`SPCHK(id)`| Checks the animationstatus of a sprite.|
| --- | --- |
|`SPGETV(id, var)`| Gets the value of a sprite variable.|
|`SPHIT(id)`| Checks for collisions between the sprite `id` with other sprites. Returns 1 if there is a collision, and sets some system variables.|
|`SPHIT(id, start)`| Checks for collisions between sprite `id` and sprites with IDs greater than `start`. Returns 1 if there is a collision, and sets some system variables.|
|`SPHITRC(id, x, y, width, height)`| Checks for collision between a sprite and a rectangular region.|
|`SPHITRC(id, x, y, width, height, dx, dy)`| Checks for collision between a sprite and a rectangular region, accounting for motion of the rectangle.|
|`SPHITSP(id, other)`| Checks for collision between two specific sprites, returning 1 if collision occurs.|
