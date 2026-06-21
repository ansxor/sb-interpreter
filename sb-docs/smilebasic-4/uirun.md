---
title: UIRUN
slug: docs-sb4-uirun
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-uirun
content_id: 19466
created: 2020-04-29
scraped: 2026-06-21
---

# UIRUN

Load and run a program in the UI environment. If a program is not given, the default UI program will run. Direct Mode only.

## Syntax

```sbsyntax
UIRUN {program$}
```

| Input | Description |
| --- | --- |
|`program$`|The path to the program file to run. (optional)|

## Examples

```sb4
UIRUN                    'run the default UI program
UIRUN "#SYS/SOFTKEY.PRG" 'run the software keyboard
UIRUN "MYUI"             'run the program MYUI in the
                         'current project as a UI program
```

## Notes

- The program is always loaded and run in slot 0 in the UI environment.
- If a UI program is currently running, it is stopped.
- The memory of UI program slot 0 is cleared (including variables and `SPFUNC`/`TFUNC` mappings) before this program is loaded.
- Passing `""` as the program argument will cause SB4 to behave strangely, including aspects of the Top Menu not working.
- As of 4.3.0, the default UI program is `#SYS/SOFTKEY.PRG` and there is no setting to change this.
