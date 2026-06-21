---
title: What is DIRECT Mode?
slug: docs-sb3-manual-what-is-direct-mode
system: SmileBASIC 3
type: guide
topic: 18
source: e-manual.pdf
scraped: 2026-06-21
---

# What is DIRECT Mode?

In DIRECT mode, instructions are passed to the computer directly.

## OK Mark — Waiting for an Instruction to be Input

"OK" is displayed on the console screen, with the cursor blinking on the next line. This is called the "instruction waiting" state. When you are about to enter an instruction, make sure that OK is being displayed.

## Inputting BASIC Instructions

Let's try out some simple instructions.

**Generate a sound (BEEP instruction):** Input the following from the cursor position. A funny sound will be generated.

- Insert a space after "BEEP" using the SPACE key.
- At the end, press the ENTER key.
- If you do not hear a sound, please check the volume control.

```sb3
BEEP 5
```

Next, try inputting the instruction with a different number specified after "BEEP."

A different sound from before was generated. Information specified together with an instruction like this is called an "argument." Depending on the instruction, the arguments will have different meanings, or there may be no arguments.

**Clear the screen (CLS instruction):** Input the following:

```sb3
CLS
```

All the characters on the screen have disappeared, and now only "OK" (instruction waiting) is displayed. The `CLS` instruction is used to clear the screen.

**Change the color of characters (COLOR instruction):** The console screen uses white as the standard color, but this can be changed to a different color. Input the following instruction:

```sb3
COLOR 3
```

- Please be careful not to input the number zero instead of the letter O in COLOR.

The execution result will be as follows: The previously white "OK" is now displayed in red. The argument 3 is a numerical value that means "red."

If you input `COLOR 15` and press the ENTER key, the color will return to white.

The range of available color numbers is from 0 to 15. However, 0 is transparent, and 1 is black. Be careful when specifying these, because the characters will become invisible.

## About Errors

Computers interpret instructions exactly as they are entered. If you mistype just a single letter in an instruction, the computer will not execute it. Try inputting a fake instruction ("ABC") and press the ENTER key.

A warning beep sounds, and the message "Undefined function" is displayed. This is an error message meaning "This feature does not exist."

Please refer to the "Error Message Table" page for more information on error messages.

## What is a Program?

In DIRECT mode, instructions are executed one by one as they are input, which means you have to input instructions each time you want to execute them. However, to generate complex actions, such as those seen in games, it's necessary to input all the instructions together, in advance.

This is achieved by using a "program." A program is a set of multiple instructions arranged sequentially. On the next page, we'll try writing a program.
