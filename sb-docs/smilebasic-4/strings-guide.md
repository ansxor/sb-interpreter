---
title: Strings Guide
slug: docs-sb4-strings-guide
system: SmileBASIC 4
type: guide
source: https://smilebasicsource.com/forum/thread/docs-sb4-strings-guide
content_id: 19489
created: 2020-10-28
scraped: 2026-06-21
---

# Strings Guide

*Strings* are one of the three fundamental data types in SmileBASIC 4. This guide will explain how to use them and the built-in features that are available.

## What is a String?

A string is a list of text characters. They are created by writing something between double quotes (`"`), like this:

```sb4
"This is a string!"
```

Being a basic data type, they can be assigned to variables, passed to functions, or used in expressions, like any other value.

```sb4
VAR A$="foo"
PRINT "hello, ";A$;"!"
```

Note that, if a string is unclosed at the end of a line, it will be automatically closed. This means any trailing whitespace on the line is included, be careful! SmileBASIC *does not* support multiline strings, however.

```sb4
'the string is unclosed,
'but at the end of the line the parser will close it
'in this case there is no extra whitespace,
'so the result is the same
VAR A$="foo
PRINT "hello, ";A$;"!"
```

## Operators

Strings support three basic operators: concatenation (`+`), repetition (`*`), and indexing (`[]`).

### Concatenation

Using the plus sign (`+`), two strings can be concatenated together. The two strings are joined, end to end.

```sb4
PRINT "FOO"+"BAR"  'FOOBAR
```

This might look like addition, but it isn't! SB4 might be dynamically-typed, but it doesn't implicitly convert strings to any other type or vice-versa.

```sb4
PRINT "123"+"456"  '123456, not 579!
```

`123` and `"123"` are two different things. One is the actual number 123, and the other contains the /text/ "123". Strings simply represent text.

Likewise, strings and numbers cannot be mixed! If you need to concatenate a number to a string, use `STR$`.

```sb4
VAR SCORE=10
'bad! Type mismatch
PRINT "Your score: "+SCORE
'good
PRINT "Your score: "+STR$(SCORE)
```

### Repetition

Using the multiplication symbol (`*`), a string's contents can be repeated a number of times.

```sb4
PRINT "FOO"*3  'FOOFOOFOO
```

Like real multiplication, multiplying by 0 will return the empty string.

```sb4
PRINT "FOO"*0  'nothing!
```

The repetition count is an integer, so real numbers will be truncated. Also, negative numbers are not allowed!

```sb4
PRINT "FOO"*1.2  'FOO
PRINT "FOO"*-1   'error!
```

### Indexing

Much like arrays, strings support indexing operations (`[]`).

#### Access

Used in an expression, the indexing bracket will copy the character at that index to a new string. You can use this to refer to individual characters in string variables, for example.

```sb4
VAR A$="ABC"
PRINT A$[2]  'C
```

Indexing is zero-based just like anything else in SB4. Also, specifying indexes out of range is an error.

```sb4
PRINT "ABC"[0]   'char 0, A
PRINT "ABC"[1]   'char 1, B
PRINT "ABC"[2]   'char 2, C
PRINT "ABC"[3]   'error
PRINT "ABC"[-1]  'error
```

#### Assignment

A string variable can be used with an indexed assignment to modify the contents of a string in-place. The character at the given index is replaced with the string on the right hand side of the assignment.

```sb4
VAR A$="ABC"
A$[1]="D"
PRINT A$  'ADC
```

Note that only the single character at that index is replaced. If more complex replacements are necessary, use [the `SUBST$` function](https://smilebasicsource.com/forum/thread/docs-sb4-subst).

```sb4
VAR A$="FOOBARBAZ"
A$[3]="QUX"
PRINT A$  'FOOQUXARBAZ
```

Additionally, assigning an empty string to an index will remove the character there. Again, if you need to remove multiple characters at once, [the `SUBST$` function](https://smilebasicsource.com/forum/thread/docs-sb4-subst) will do you better.

```sb4
VAR A$="FOOBARBAZ"
A$[3]=""
PRINT A$  'FOOARBAZ
```

## Functions

SmileBASIC includes a set of primitive functions for working with strings. Click each function name to go to its reference page.

### Length

- [`LEN` Get the length of a string](https://smilebasicsource.com/forum/thread/docs-sb4-len)
- [`LAST` Get the index of the last character of a string](https://smilebasicsource.com/forum/thread/docs-sb4-last)

### Characters

- [`CHR$` Get the character corresponding to a character code](https://smilebasicsource.com/forum/thread/docs-sb4-chr)
- [`ASC` Convert a character to its character code](https://smilebasicsource.com/forum/thread/docs-sb4-asc)

### Conversion / Formatting

- [`STR$` Convert a number to a string](https://smilebasicsource.com/forum/thread/docs-sb4-str)
- [`VAL` convert a string to a number](https://smilebasicsource.com/forum/thread/docs-sb4-val)
- [`HEX$` Convert a number to a hexidecimal string](https://smilebasicsource.com/forum/thread/docs-sb4-hex)
- [`BIN$` Convert a number to a binary string](https://smilebasicsource.com/forum/thread/docs-sb4-bin)
- [`FORMAT$` Format values into a string](https://smilebasicsource.com/forum/thread/docs-sb4-format)

### Substrings

- [`LEFT$` Get characters from the start of a string](https://smilebasicsource.com/forum/thread/docs-sb4-left)
- [`RIGHT$` Get characters from the end of a string](https://smilebasicsource.com/forum/thread/docs-sb4-right)
- [`MID$` Get a substring from a string](https://smilebasicsource.com/forum/thread/docs-sb4-mid)
- [`INSTR` Check if a string contains a substring](https://smilebasicsource.com/forum/thread/docs-sb4-instr)
- [`SUBST$` Replace the contents of a substring](https://smilebasicsource.com/forum/thread/docs-sb4-subst)

### Manipulation

- [`COPY` Copy strings and substrings](https://smilebasicsource.com/forum/thread/docs-sb4-copy)
- [`PUSH` Insert characters at the end](https://smilebasicsource.com/forum/thread/docs-sb4-push)
- [`POP` Remove and return the last character](https://smilebasicsource.com/forum/thread/docs-sb4-pop)
- [`UNSHIFT` Insert characters at the start](https://smilebasicsource.com/forum/thread/docs-sb4-unshift)
- [`SHIFT` Remove and return the first character](https://smilebasicsource.com/forum/thread/docs-sb4-shift)
- [`INC` Append characters to the end](https://smilebasicsource.com/forum/thread/docs-sb4-inc)

## Pointers, Mutation, and Copying

Under the hood, strings are represented as a pointer to their character data. This is very efficient, because it means a string's contents don't have to be copied whenever it is referenced in the code. Of course, most operations on strings (all of the built-in string functions, concatenation, repetition, etc.) create a new copy, so that any other references to the same string data don't get changed as well, but some change the array's data in-place. In fact, this is why indexed assignment works on strings: the string is mutated in-place. However, this has some further-reaching gotchas you should probably know about.

On assignment, a string's *pointer* is copied, not its contents. In this example, both variables refer to the same *contents*, so both are mutated in the same way.

```sb4
VAR A$="ABCDE"
VAR B$=A$  'the POINTER is copied
B$[0]="F"
PRINT A$
```

Even though `B$` was modified, the contents of `A$` still changed, since both variables contain the same string pointer. While it is rare that this will become a problem in practice, if it /does/, you can use `COPY` to explicitly copy a string.

```sb4
VAR A$="ABCDE"
VAR B$=COPY(A$)  'the CONTENT is copied
```

Of course, this behavior is what allows us to use some of the array functions directly on strings. Mentioned above is `COPY`, but `PUSH`, `POP`, `SHIFT`, and `UNSHIFT` also work.
