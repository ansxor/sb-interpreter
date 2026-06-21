---
title: IF ~ THEN ~ ELSEIF ~ ELSE ~ ENDIF
slug: docs-sb4-if
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-if
content_id: 19513
created: 2020-11-04
scraped: 2026-06-21
---

# IF ~ THEN ~ ELSEIF ~ ELSE ~ ENDIF

`IF` statements allow you to only run code if a *condition* is true. There are multiple forms of `IF` statement, which will be covered in this page.

## Conditions

A *condition* is an expression that results in a true or false value. The "true" value is 1 (or any non-zero number) and the "false" value is 0. The condition cannot result in a string or array, or a `Type mismatch` error will occur.

## Simple IF

```sb4
IF condition THEN statement
```

| Name | Description |
| --- | --- |
| `condition` | A conditional expression. |
| `statement` | Any single SmileBASIC statement. This will run if `condition` is true. |

This `IF` statement will run `statement` only if `condition` is true.

```sb4
IF RND(2) THEN PRINT "50/50 odds!"
```

## ELSE

If you want to run a different statement when the condition is false, you can use `ELSE`.

```sb4
IF condition THEN trueStatement ELSE falseStatement
```

| Name | Description |
| --- | --- |
| `condition` | A conditional expression. |
| `trueStatement` | Any single SmileBASIC statement. This will run if `condition` is true. |
| `falseStatement` | Any single SmileBASIC statement. This will run if `condition` is false. |

```sb4
HEALTH=50
IF HEALTH<100 THEN PRINT "Perfect health!" ELSE PRINT "Less-than-perfect health."
```

## Multi-Line IF Blocks

If you want to write long conditions or complex statements, multi-line `IF` is much more convenient. If a line break comes after `THEN`, then the `IF` statement is a multi-line block, and can span multiple statements.

```sb4
IF condition THEN
 statements...
ENDIF
```

| Name | Description |
| --- | --- |
| `condition` | A conditional expression. |
| `statements...` | Any number of SmileBASIC statements. This will run if `condition` is true. |

Multi-line `IF` is not strictly necessary, but if your lines of code becomes too long it can become harder to read. It is a good idea to use it where appropriate.

```sb4
'this line is too long...
IF LONG_VARIABLE_NAME<OTHER_LONG_NAME THEN PRINT "LESS":DO_SOMETHING
'let's fix it
IF LONG_VARIABLE_NAME<OTHER_LONG_NAME THEN
 PRINT "LESS"
 DO_SOMETHING
ENDIF
```

A multi-line `IF` can contain one `ELSE` section as well.

```sb4
IF LONG_VARIABLE_NAME<OTHER_LONG_NAME THEN
 PRINT "LESS"
 DO_SOMETHING
ELSE
 PRINT "MORE"
 DO_SOMETHING_ELSE
ENDIF
```

## ELSEIF

An `IF` block can also contain multiple alternative conditions, in the form of `ELSEIF` statements.

```sb4
IF ifCond THEN
 ifStatements...
ELSEIF elseifCond1 THEN
 elseifStatements...
ENDIF
```

| Name | Description |
| --- | --- |
| `ifCond` | The condition associated with the opening `IF` statement. |
| `ifStatements...` | These statements are run if `ifCond` is true. |
| `elseifCond` | A condition associated with an `ELSEIF` statement. |
| `elseifStatements...` | These statements are run if the associated `elseifCond` is true. |

You can use any number of `ELSEIF` sections. In the entire `IF` block, each condition is checked in order, and the first one that is true is taken. If none of the conditions are true, then the `ELSE` section is taken, if it exists.

```sb4
HEALTH=50
IF HEALTH==0 THEN
 PRINT "Dead!"
ELSEIF HEALTH<25 THEN
 PRINT "Worse for wear..."
ELSEIF HEALTH<50 THEN
 PRINT "A little beat up"
ELSEIF HEALTH<75 THEN
 PRINT "Getting tired"
ELSE
 PRINT "Perfect health!"
ENDIF
```

Note that only the *first* condition that is true is taken. `ELSEIF` is intended to be used for mutually-exclusive conditions. Even if two conditions are exactly the same and are both true, only the first is taken. You may have to use two separate `IF` statements to achieve the effect you want.

```sb4
IF #TRUE THEN
 PRINT "I'm TRUE!!!!"
ELSEIF #TRUE THEN
 PRINT "Me too, but you'll never see it..."
ENDIF
```

## Notes

### IF GOTO

If the body of the `IF` statement contains a single `GOTO`, there is a shorthand you can use.

```sb4
IF condition GOTO @label
```

The entire statement must be written on one line. The `GOTO` shortcut can also be used in an `ELSEIF`.
