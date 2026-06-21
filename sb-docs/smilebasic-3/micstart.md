---
title: MICSTART
slug: docs-sb3-micstart
system: SmileBASIC 3
type: command
category: Various kinds of input
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# MICSTART

> **Category:** Various kinds of input

Starts sampling from the microphone

- The microphone should be enabled beforehand with XON MIC
- Recorded into memory used for sampling in the system

## Format

```sb3
MICSTART Sampling rate, Number of bits, Number of seconds
```

## Arguments

| Argument | Description |
| --- | --- |
| `Sampling rate` | 0: 8180Hz<br>1: 10910Hz<br>2: 16360Hz<br>3: 32730Hz |
| `Number of bits` | 0: 8 bits<br>1: 16 bits |
| `Number of seconds` | 0: Loop mode<br>1-: Number of seconds for sampling<br>- 8180Hz: Up to 32 sec for 8 bits, 16 sec for 16 bits<br>- 10910Hz: Up to 24 sec for 8 bits, 12 sec for 16 bits<br>- 16360Hz: Up to 16 sec for 8 bits, 8 sec for 16 bits<br>- 32730Hz: Up to 8 sec for 8 bits, 4 sec for 16 bits<br>- In loop mode, data will be overwritten from the beginning once the maximum number of seconds<br>has been reached |

## Examples

```sb3
XON MIC
MICSTART 0,1,10
```
