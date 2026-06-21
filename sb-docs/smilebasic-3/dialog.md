---
title: DIALOG
slug: docs-sb3-dialog
system: SmileBASIC 3
type: command
category: Basic instructions (data operations and others)
source: InstructionList.pdf
forms: 6
scraped: 2026-06-21
---

# DIALOG

> **Category:** Basic instructions (data operations and others)

## DIALOG (1)

Displays a dialog and waits for a button to be pressed

- The result is returned with the system variable RESULT
- RESULT: 1 (Confirmed), -1 (Canceled), 0 (Time out)

### Format

```sb3
DIALOG "Text string"
```

### Arguments

| Argument | Description |
| --- | --- |
| `Text string` | Character string to display in the dialog |

### Supplement (common for DIALOG instructions)

```
- Dialog are always displayed on the Touch Screen
- The total length of the text string and caption string should be 256 characters or less
- If CHR$(10) or CHR$(13) is included in the text string, a line break will occur at that point
- If a negative value is set for the Timeout period, texts are handled in frame units
```

### Examples

```sb3
DIALOG "Good morning!"
```

## DIALOG (2)

Displays a dialog and waits for a button to be pressed

### Format

```sb3
DIALOG "Text string",[Selection type],["Caption string"],[Timeout period]
```

### Arguments

| Argument | Description |
| --- | --- |
| `Text string` | Character string to display in the dialog |
| `Selection type` | 0: OK (default)<br>5: Next |
| `Caption string` | Character string to display in the caption field at the top of the dialog |
| `Timeout period` | Number of seconds to wait before closing the dialog automatically (If omitted, 0: Not closed) |

### Supplement (common for DIALOG instructions)

```
- Dialog are always displayed on the Touch Screen
- The total length of the text string and caption string should be 256 characters or less
- If CHR$(10) or CHR$(13) is included in the text string, a line break will occur at that point
- If a negative value is set for the Timeout period, texts are handled in frame units
```

### Examples

```sb3
DIALOG "Let's get started",5,"Scenario",-120
```

## DIALOG (3)

Displays a dialog and waits for the specified button to be pressed

### Format

```sb3
Variable = DIALOG("Text string",[Selection type],["Caption string"],[Timeout period])
```

### Arguments

| Argument | Description |
| --- | --- |
| `Text string` | Character string to display in the dialog |
| `Selection type` | 0: OK (default)<br>1: No/Yes<br>2: Back/Next<br>3: Cancel/Confirm<br>4: Cancel/Execute<br>5: Next |
| `Caption string` | Character string to display in the caption field at the top of the dialog |
| `Timeout period` | Number of seconds to wait before closing the dialog automatically (If omitted, 0: Not closed) |

### Return Values

```
-1: Negation (Left button)
0: Timeout
1: Affirmation (Right button)
* These values will also remain in the system variable RESULT.
```

### Supplement (common for DIALOG instructions)

```
- Dialog are always displayed on the Touch Screen
- The total length of the text string and caption string should be 256 characters or less
- If CHR$(10) or CHR$(13) is included in the text string, a line break will occur at that point
- If a negative value is set for the Timeout period, texts are handled in frame units
```

### Examples

```sb3
R=DIALOG("Would you like to try again?" ,1,"
",0)
```

## DIALOG (4)

Displays a dialog and waits for the Touch Screen or a hardware button to be pressed

### Format

```sb3
Variable = DIALOG("Text string",Button type,["Caption string"],[Timeout period])
```

### Arguments

| Argument | Description |
| --- | --- |
| `Text string` | Character string to display in the dialog |
| `Button type` | \|b00\| ABXY buttons (1)<br>\|b01\| +Control Pad (2)<br>\|b02\| L,R buttons (4)<br>\|b03\| Touch Screen (8)<br>- Specify a value for which the logical OR is calculated with the above bit value and the sign<br>is reversed<br>- ZL and ZR buttons cannot be detected<br>- -1 causes only ABXY to be specified<br>- For example, to detect ABXY and +Control Pad, -3 should be specified |
| `Caption string` | Character string to display in the caption field at the top of the dialog |
| `Timeout period` | Number of seconds to wait before closing the dialog automatically (if omitted, 0: Not closed) |

### Return Values

128: A button pressed 129: B button pressed 130: X button pressed 131: Y button pressed 132: +Control Pad up pressed 133: +Control Pad down pressed 134: +Control Pad left pressed 135: +Control Pad right pressed 136: L button pressed 137: R button pressed 140: Touch Screen pressed

### Examples

```sb3
R=DIALOG("ABXYLR/+Control Pad/Touch",-15,"Special",0)
```

## DIALOG (5)

Displays a dialog used only for inputting file names

### Format

```sb3
String=DIALOG( "Initial string", "Caption string" [,Maximum characters])
```

### Arguments

| Argument | Description |
| --- | --- |
| `Initial string` | String that is initially input |
| `Caption string` | String to be displayed in the caption field |
| `Maximum characters` | Up to 14 characters |

### Return Values

```
The obtained character string will be returned
* If RESULT=-1, Canceled (the character string is invalid)
```

### Supplement (common for DIALOG instructions)

```
- Dialog are always displayed on the Touch Screen
- The total length of the text string and caption string should be 256 characters or less
- If CHR$(10) or CHR$(13) is included in the text string, a line break will occur at that point
- If a negative value is set for the Timeout period, texts are handled in frame units
```

### Examples

```sb3
T$=DIALOG( "NEWNAME0","SAVE", 14 )
```

## DIALOG (6)

Displaying special characters in DIALOG To use special character and symbols, pass the character code in the UTF-16 format to CHR$

- For details on the UTF-16 format, please refer to a technical book or similar resource.

### Supplement (common for DIALOG instructions)

```
- Dialog are always displayed on the Touch Screen
- The total length of the text string and caption string should be 256 characters or less
- If CHR$(10) or CHR$(13) is included in the text string, a line break will occur at that point
- If a negative value is set for the Timeout period, texts are handled in frame units
```

### Examples to display Japanese Kanji characters

```sb3
'
T$="
"+CHR$(&H4E00)+CHR$(&H5EA6)+"
'
C$=CHR$(&H78BA)+CHR$(&H8A8D)
R=DIALOG(T$,1,C$,0)
"
```
