---
title: FILES
slug: docs-sb4-files
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-files
content_id: 19456
created: 2020-10-26
scraped: 2026-06-21
---

# FILES

Get a list of files.

## Syntax

```sbsyntax
FILES { filter$ } {, output$[] }
output$[] = FILES({ filter$ })
FILES { filter$ } OUT output$[]
```

| Input | Description |
| --- | --- |
| `filter$` | String specifying which project and file type to list (optional.) |

| Output | Description |
| --- | --- |
| `output$[]` | String array to store the result (optional.) |

If `output$` is not specified, then the file list is printed to the screen.

## Filter String

The `filter$` parameter allows you to specify both the type of files and the project to list the files from. It takes the form `type:project`, where `project` is the project name (e.g. `#SYS/`) and `type` is one of:

| Type | Description |
| --- | --- |
| `TXT` | Text file (programs, strings) |
| `GRP` | Graphic page |
| `DAT` | Array |

For example, if `filter$` is `"TXT:#SYS/"` then the list will contain only `TXT` files within the `#SYS` project. If the project is omitted, e.g. `"TXT:"`, then `FILES` will search the active project.

`filter$` also has a special value, `"//"`, which will return a list of all project names. Note that project names end in `/`.

If `filter$` is omitted, then `FILES` defaults to returning all files in the active project.

## Examples

```sb4
'print list of files in active project
FILES
```

```sb4
'print only TXT files in active project
FILES "TXT:"
```

```sb4
'get list of projects in destination array
PROJ$=FILES("//")
FILES "//" OUT PROJ$
```

```sb4
'read list of GRP files in #SYS/ to an existing string array
DIM STR$[]
FILES "GRP:#SYS/",STR$
```

## Notes

### Two Ways to Return an Array

`FILES` has two different forms for returning an array. One takes the destination array as a parameter, and the other returns a new array (either with `OUT` or as the return value.)

```sbsyntax
FILES array$[]
FILES OUT array$[]
array$[] = FILES()
```

The former originated in SmileBASIC 3, where returning arrays from functions was not common, and was the only way to do it. This form is still kept around primarily to make code easier to port. The latter is a new addition to SmileBASIC 4, and is generally the preferred version if you're not porting.

The two methods seem the same, but they are nuanced in certain ways. In the SB3 method, the output array parameter *must* be an existing 1D string array (e.g. declared with `DIM`) or you will get `Type mismatch`. This is because the array contents are modified in place.

```sb4
'okay
DIM STR$[]
FILES STR$
INSPECT STR$
'bad!
VAR V
FILES V
```

In the SB4 method, a new array is created to hold the contents and simply returned from the function. This is generally the preferred method, and will work with any variable due to SB4's dynamic type system.

```sb4
'okay!
VAR V
V = FILES()
INSPECT V
```
