---
title: Read-only Strings [Advanced]
slug: docs-sb4-read-only-strings-advanced
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-read-only-strings-advanced
content_id: 19493
created: 2020-11-04
scraped: 2026-06-21
---

# Read-only Strings [Advanced]

Strings in SB can be writable or read-only (Note that this applies to /values/, not /variables/)
Read-only strings don't use any memory (as measured by FREEMEM), and I suspect that their data pointers point to immediate string data in the bytecode, rather than data in user memory like normal strings.

## Creating read-only strings

A read-only string value is created whenever a constant string expression is passed to a function.
String arrays also start out filled with empty read-only strings.
All other string values which you can access (as far as I know) will be writable.
There is no way to convert an arbitrary writable string into a read-only string at runtime, though there would be (as far as I know) no advantage to doing this.

```
TEST "ABC" 'parameter S will contain a read-only string
DEF TEST S
 ...
END

DIM S$[10]
'S$ is filled with read-only strings (with value "")
```

## Converting read-only strings to writable strings

- if the value passed to FILL is read-only, the array will be filled with a writable copy.
In these situations, the string value will be re-allocated:
- using INC, SWAP, = on a variable containing a read-only string 
- assigning to an index in a variable containing a read-only string
- using PUSH or UNSHIFT on a read-only string
Anyway, under most circumstances, read-only strings will not cause problems. Trying to modify them will just replace them with a copy, rather than failing.
However, there are places where they act differently from normal strings:

```
DEF TEST S
 VAR A=S,B=S
 INC A,"D"
 ?A,B
END
```

if you pass a normal, writiable string value, this behaves as expected. S and T store references to the same string value, so modifying it affects both variables.

```
A="ABC"
TEST A 'prints: ABCD  ABCD
```

However, passing a read-only string is different:

```
TEST "ABC" 'prints ABCD  ABC
```

This is because, each time the read-only string is assigned, it creates a copy. So, S and T's values will be different strings.
(this also means that, ironically, read-only strings can be slower because they need to be copied each time they are assigned to something)
