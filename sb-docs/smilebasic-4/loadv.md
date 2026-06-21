---
title: LOADV
slug: docs-sb4-loadv
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-loadv
content_id: 19555
created: 2023-03-08
scraped: 2026-06-21
---

# LOADV

Load a file as a value. All file formats are supported.

## Load as Value

```sb4
contents = LOADV(filename$)
LOADV filename$ OUT contents
```

| Input | Description |
| --- | --- |
| `filename$` | The name or path of the file to load. |

| Output | Description |
| --- | --- |
| `contents` | The contents of the file. |

`LOADV` loads any type or format of file and returns its contents. `TXT` files contain strings, `DAT` files contain arrays of any shape or type (integer, real, or string,) and `GRP` files contain graphics pages. GRPs are loaded as 2D integer arrays of the appropriate size.

`filename$` is a standard file name or path string. A file type is required (`"TXT:"`, `"DAT:"`, or `"GRP:"`.) Programs running in any environment can load files from any project path.

### Examples

```sb4
'load the character's name
VAR NAME$=LOADV("TXT:CHARNAME$")
PRINT "Hello, ";NAME$
```

```sb4
'load a high score table
VAR HISCORE%=LOADV("DAT:SCORES")
VAR I%
FOR I%=0 TO LAST(HISCORE%)
 PRINT HISCORE%[I]
NEXT I
```

## Load to Array

```sb4
LOADV filename$, destination[]
```

| Input | Description |
| --- | --- |
| `filename$` | The name or path of the file to load. |
| `destination[]` | The array used to store the file contents. |

This form of `LOADV` is used to overwrite an existing array with the contents of a `DAT` or `GRP` file. `TXT` files cannot be loaded this way because they contain strings. Because `destination[]` is modified in-place, it must already have a compatible type or number of dimensions (e.g. a 2D string `DAT` file must be loaded to a 2D string array.) The lengths of each dimension do not matter; they will be resized to match the file.

`filename$` is a standard file name or path string. A file type is required (`"DAT:"` or `"GRP:"`.) Programs running in any environment can load files from any project path.

### Examples

```sb4
'load a high score table
DIM HISCORE%[]
LOADV "DAT:SCORES",HISCORE%
VAR I%
FOR I%=0 TO LAST(HISCORE%)
 PRINT HISCORE%[I]
NEXT I
```
