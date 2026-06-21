---
title: Writing in EDIT Mode
slug: docs-sb3-manual-writing-in-edit-mode
system: SmileBASIC 3
type: guide
topic: 19
source: e-manual.pdf
scraped: 2026-06-21
---

# Writing in EDIT Mode

Let's try combining some simple instructions to write a program. Switch to EDIT mode so that you can begin inputting the program.

## Input a Simple Program

Now, try inputting a program that executes the following procedure. The program is made up of three lines in total.

1. Clear the screen — `CLS` instruction
2. Print "HELLO" — `PRINT` instruction
3. Generate a sound — `BEEP` instruction

**Input the first line and press the ENTER key:** On the first line, input the `CLS` instruction, an instruction that clears the screen.

```sb3
CLS
```

The yellow mark at the beginning means that this is the line currently being input. A line number is assigned automatically. The line feed mark on the right side of the line indicates the end of a line.

Once you have finished inputting one line, press the ENTER key to begin a new one.

**Input the next line:** On the second line, input an instruction to print "HELLO" on the screen. The instruction for printing characters is `PRINT`.

```sb3
PRINT "HELLO"
```

The `PRINT` instruction should be accompanied by an argument to specify the character string to print. One of the rules in BASIC is that character strings must be enclosed in double quotation marks ("). Please make sure to input these.

**Input the third line in the same way to complete the program:**

```sb3
BEEP
```

## Running a Program (RUN Instruction)

Next, let's switch to DIRECT mode and run the program you have input.

The instruction for running programs is `RUN`.

```sb3
RUN
```

If the program is input correctly, the screen will be cleared, "HELLO" will be printed, and a sound will be generated.

If the expected result is not attained, go back to EDIT mode and check for errors.

**Press START to force the running program to stop:** Programs begin running from the first line, and stop at the last line. However, some programs may run endlessly. To stop such a program, press START on the 3DS system. The program will be aborted.

## Erasing Programs (NEW Instruction)

To erase all the programs you have input in one go, execute the `NEW` instruction in DIRECT mode.

```sb3
NEW
```

All programs will be erased. If `NEW` is executed, all program SLOTS will be erased. Please be very careful!

**Erase only a specific program SLOT:** The `NEW` instruction has an argument that can specify the program SLOT to erase. You can specify a numerical value between 0 to 3, which corresponds to SLOT0 to SLOT3.

- `NEW 0` → Erases only SLOT0
- `NEW 1` → Erases only SLOT1
- `NEW 2` → Erases only SLOT2
- `NEW 3` → Erases only SLOT3
- `NEW` → Erases all SLOTS
