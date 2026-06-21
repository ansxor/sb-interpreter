---
title: CALL
slug: docs-sb4-call
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-call
content_id: 19507
created: 2020-04-30
scraped: 2026-06-21
---

# CALL

Call functions by name, or run callbacks.

## Call by Name (Statement)

Call a function, given its name as a string, using the specified arguments. This function *must not* return a value.

### Syntax

```sb4
CALL name$, { in... }
```

| Input | Description |
| --- | --- |
| `name$` | The name of the function to call. |
| `in...` | A list of arguments to pass to the function. Can be zero or more, depending on the function. |

### Examples

```sb4
'call GLINE by name and pass parameters
CALL "GLINE",0,0,100,100

'call a DEF by name
DEF FOO
 PRINT "HELLO"
END
VAR N$="FOO"
CALL N$ '> HELLO
```

## Call by Name (Function Expression)

Call a function, given its name and parameters, and return its value in an expression

### Syntax

```sb4
ret = CALL(name$ {, in... })
```

| Input | Description |
| --- | --- |
| `name$` | The name of the function to call. |
| `in...` | A list of arguments to pass to the function. Can be zero or more, depending on the function. |

| Output | Description |
| --- | --- |
| `ret` | The return value of this function. |

### Examples

```sb4
PRINT CALL("MAX", 2,3) '> 3
```

## Call by Name (`OUT` Form)

Call a function given its name, in `OUT` form.

### Syntax

```sb4
CALL name$ {, in...} OUT { out... }
```

| Input | Description |
| --- | --- |
| `name$` | The name of the function to call. |
| `in...` | A list of arguments to pass to the function. Can be zero or more, depending on the function. |

| Output | Description |
| --- | --- |
| `out...` | The return values of this function. Can be zero or more, depending on the function. |

### Examples

```sb4
VAR YEAR,MONTH,DAY
CALL "DTREAD" OUT YEAR, MONTH, DAY
```

## Sprite Callback

Run all callbacks associated to sprites with `SPFUNC`, in order of sprite ID. The value of `CALLIDX` is changed to the sprite ID for each callback.

### Syntax

```sb4
CALL SPRITE
```

### Examples

```sb4
DEF MYCALL
 PRINT CALLIDX()
END
SPSET 0,0
SPFUNC 0,"MYCALL"
CALL SPRITE 'prints 0
```

## Text Screen Callback

Run all callbacks associated to text screens with `TFUNC`, in order of text screen ID. The value of `CALLIDX` is changed to the text screen ID for each callback.

### Syntax

```sb4
CALL TEXT
```

### Examples

```sb4
DEF MYCALL
 PRINT CALLIDX()
END
TFUNC 0,"MYCALL"
CALL TEXT 'prints 0
```
