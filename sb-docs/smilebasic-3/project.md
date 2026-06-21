---
title: PROJECT
slug: docs-sb3-project
system: SmileBASIC 3
type: command
category: Instructions available only in DIRECT mode
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# PROJECT

> **Category:** Instructions available only in DIRECT mode

## PROJECT (1)

Switches the default project DIRECT mode only

### Format

```sb3
PROJECT "Project name"
```

### Arguments

| Argument | Description |
| --- | --- |
| `Project name` | Name string of project to change<br>- New projects can be created from the TOP MENU<br>- Project name "" specifies the default project |

### Supplement

| Item | Description |
| --- | --- |
| `Three conditions<br>for the current<br>project` | 1) Current project at start-up time (specified using Change Active Project option under the<br>Manage Projects/Files menu)<br>2) Current project at non-execution time (set with the PROJECT instruction)<br>3) Current project at execution time (set during execution, e.g., of EXEC)<br>When the current project is set for 1) - 3), the respective subordinate project settings will<br>also be updated accordingly.<br>For example, if the current project is changed through the Change Active Project setting, the<br>project at non-execution time and the project at execution time will also be changed.<br>Furthermore, when execution is started (by using RUN, executing a tool, or executing a program<br>from the file viewer), the current project at non-execution time will be set as the initial<br>value of the current project at execution time. |

### Examples

```sb3
PROJECT ""
```

## PROJECT (2)

Obtains the default project Can be also used from within programs

### Format

```sb3
PROJECT OUT PJ$
```

### Arguments

| Argument | Description |
| --- | --- |
| `None` |  |

### Return Values

| Return Value | Description |
| --- | --- |
| `PJ$` | Current project name |

### Examples

```sb3
PROJECT OUT PJ$
```
