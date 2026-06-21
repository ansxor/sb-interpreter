---
title: CASE ~ WHEN ~ OTHERWISE ~ ENDCASE
slug: docs-sb4-case
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-case
content_id: 19512
created: 2020-11-04
scraped: 2026-06-21
---

# CASE ~ WHEN ~ OTHERWISE ~ ENDCASE

A `CASE` block allows you to choose a section of code to run based on a value or result of an expression. This control structure is often called a `switch` statement in other languages.

## Syntax

```sb4
CASE caseVal
 WHEN whenVal1
  whenBody1...
{WHEN whenVal2
  whenBody2...}...
{OTHERWISE
  otherwiseBody...}
ENDCASE
```

| Name | Description |
| --- | --- |
| `caseVal` | This value is compared with every `whenVal` to choose which `whenBody` to branch to. |
| `whenVal` | If this value is equal to `caseVal`, then the `whenBody` associated with this `WHEN` statement is executed. |
| `whenBody` | The code after this `WHEN` statement is run if the `whenVal` is equal to `caseVal`. |
| `otherwiseBody` | The code after `OTHERWISE` is run if no `WHEN` is matched. Optional. |

## Examples

A `CASE` block can contain any number of `WHEN` sections. These sections are checked in order to see if they match the `CASE`, and the first match is run.

```sb4
VAR I%
PRINT "1) Cake"
PRINT "2) Burger"
PRINT "3) Salad"
INPUT "Choice";I%
PRINT "You chose: ";
CASE I%
 WHEN 1
  PRINT "Cake"
 WHEN 2
  PRINT "Burger"
 WHEN 3
  PRINT "Salad"
ENDCASE
```

If two `WHEN` sections have the same value, then the first one is used; the second will never be reached.

```sb4
INPUT I
CASE I
 WHEN 1
  PRINT "one"
 WHEN 2
  PRINT "two"
 WHEN 1
  PRINT "you'll never meet me!"
ENDCASE
```

If no `WHEN` sections are matched, then the `OTHERWISE` section is matched, if it is present.

```sb4
CASE RND(100)
 WHEN 7
  PRINT "Lucky!"
 WHEN 13
  PRINT "Unlucky..."
 OTHERWISE
  PRINT "Just a number."
ENDCASE
```

To match multiple values to one block, just use two `WHEN` statements next to each other.

```sb4
VAR I
INPUT I
CASE I
 '1 and 2 match the same block
 WHEN 1
 WHEN 2
  PRINT "One or two"
 'this way also works
 WHEN 3:WHEN 4
  PRINT "Three or four"
ENDCASE
```

Keep in mind that this means you cannot write a `WHEN` block that does absolutely nothing, unless it is last in the `CASE`. If a `WHEN` block is empty, it just matches to the one below it. To match a value and then do nothing, you'll have to write some code in the block that does nothing, or write it last.

```sb4
CASE I
 WHEN 2
  PRINT "two"
 'we want 1 to do nothing
 WHEN 1
ENDCASE
```

Also note that, since matches are evaluated in order, if this `CASE` contains an `OTHERWISE` you cannot rely on writing the "ignored" match last. You will have to resort to writing meaningless code in the matches you want to explicitly ignore

```sb4
CASE I
 WHEN 1
  'this does nothing
  NOP
 WHEN 2
  PRINT "two"
 OTHERWISE
  PRINT "another number"
ENDCASE
'this function does nothing
DEF NOP END
```

If an empty `WHEN` is written before an `OTHERWISE`, then that value is simply matched to `OTHERWISE`.

```sb4
CASE I
 WHEN 2
  PRINT "two"
 WHEN 1
 OTHERWISE
  PRINT "another number"
ENDCASE
```

The `OTHERWISE` section can be placed anywhere within the `CASE` block, but it is standard to put it at the end, because matches are evaluated in order. If `OTHERWISE` is written first, for example, then it will always match.

```sb4
'OTHERWISE will be matched, even if 7 or 13 are entered!
CASE RND(100)
 OTHERWISE
  PRINT "Just a number."
 WHEN 7
  PRINT "Lucky!"
 WHEN 13
  PRINT "Unlucky..."
ENDCASE
```

If multiple `OTHERWISE` blocks are written, then they are all checked in order and will all match. This can be confusing, so it's recommended to just use one.

```sb4
'you shouldn't do this!
'OTHERWISE will be matched if 13 is entered, but not 7, due to order
CASE RND(100)
 WHEN 7
  PRINT "Lucky!"
 OTHERWISE
  PRINT "Just a number."
 WHEN 13
  PRINT "Unlucky..."
 OTHERWISE
  PRINT "Still just a number."
ENDCASE
```

Using multiple `OTHERWISE` blocks or placing them anywhere other than last can cause unexpected behavior, so it should be avoided.

```sb4
'just do this instead...
'OTHERWISE will only be matched if 7 or 13 are not matched
CASE RND(100)
 WHEN 7
  PRINT "Lucky!"
 WHEN 13
  PRINT "Unlucky..."
 OTHERWISE
  PRINT "Just a number."
  PRINT "Still just a number."
ENDCASE
```

An empty `OTHERWISE` cannot be written before a `WHEN`. This is a syntax error.

```sb4
CASE I
 WHEN 2
  PRINT "two"
 OTHERWISE  '<--- syntax error
 WHEN 1
  PRINT "one"
ENDCASE
```

## Notes

### Fallthrough

In some languages, matches will "fall through" one another unless the associated block doesn't contain a `break`. There is no fallthrough feature in SmileBASIC, so a `BREAK` is unnecessary.

### Exit Behavior: WHEN vs OTHERWISE

After a `WHEN` is matched, the interpreter stops checking for further `WHEN` matches and exits the `CASE` block. However, after `OTHERWISE` is matched, the interpreter keeps looking for possible matches. This is what allows there to be multiple `OTHERWISE` blocks, and in any order, while also meaning that only one `WHEN` of a specific value will be matched. This also means that whether or not `OTHERWISE` will be matched is dependent on the *order* of the matches, and that a `WHEN` can be matched *after* an `OTHERWISE`.

In this example, if `I` is 1, no `OTHERWISE` blocks are matched. If `I` is 2, only the first is matched, and then `WHEN 2` is matched. If `I` is any other value, *both* are matched.

```sb4
CASE I
 WHEN 1
  PRINT "One"
 OTHERWISE
  PRINT "Otherwise A"
 WHEN 2
  PRINT "Two"
 OTHERWISE
  PRINT "Otherwise B"
ENDCASE
```
