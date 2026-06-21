---
title: Using the Keyboard
slug: docs-sb3-manual-using-the-keyboard
system: SmileBASIC 3
type: guide
topic: 17
source: e-manual.pdf
scraped: 2026-06-21
---

# Using the Keyboard

Whether you're in DIRECT mode or EDIT mode, the keyboard displayed on the Touch Screen is always used to input characters.

## Switching Character Input Mode

You can switch between alphanumeric characters, symbols, and Kana, as well as between upper and lower case.

1. **Character switching keys** — Switch between Alphanumeric, Symbols, and Kana input modes.
2. **SHIFT key** — Switches between upper and lower case for the next single character that is input. Also switches some symbols.
3. **CAP key** — Locks the SHIFT key after switching to upper or lower case so that the case is kept for subsequent inputted characters. Pressing this key again returns to normal input.

## About Inputting Symbols and Kana

If you use the character switching key to switch to symbol or Kana input, part of the keyboard will change, allowing you to input a greater variety of characters.

1. Switches between symbol categories. There are seven categories in total.
2. Switches between Hiragana and Katakana.

## Spaces and Line Breaks

If you want to insert a space between characters, press the SPACE key. If you want to begin a new line, press the ENTER key.

1. **SPACE key** — Inputs a blank space equal to one character. In some BASIC instructions, you will have to input a space to delimit the elements. Be careful when inputting instructions.
   - Wrong: `BEEP5`
   - Correct: `BEEP 5`

2. **ENTER key** — Performs very important functions:
   - In DIRECT mode: Sends instructions input on the screen to the computer for execution. Input instructions will not be executed until you press the ENTER key.
   - In EDIT mode: Inserts a line break and moves the cursor to the beginning of the next line.

In this electronic manual, the symbol ⏎ indicates that the ENTER key must be pressed at the end of an instruction.

Example:
```
PRINT "HELLO" ⏎
```

## Commonly Confused Characters

**Zero and O:** In BASIC, a strict distinction is made between the number zero and the letter o. If you input the wrong one, BASIC will not work as expected. Please be careful, as these keys are located close to each other on the keyboard.

**1, I (Uppercase i) and l (Lowercase L):** Although they look alike, these characters each have different meanings. Another example of confusing characters is a minus sign and a Kana macron. Please watch out that you don't get these confused.

**Quotation Marks and Separators:**

- **Double quotation mark** — This is used frequently. Characters enclosed in double quotation marks are handled as a character string, not as a number. Example: `135` → numerical value 135 (one hundred and thirty-five); `"135"` → character string (one three five)

- **Semicolon** — This is used to, for example, separate an instruction from its parameters. Example: `PRINT "The amount is ";A`

- **Colon** — This is used to, for example, list multiple instructions. Example: `BEEP 5:GOSUB @DM:PRINT "BYE"`. Please be careful, as in some cases a colon follows a semicolon. Example: `PRINT "Valid";:GOTO @TOP`

- **Comma** — This is used to, for example, separate arguments or pieces of data. Example: `GLINE 0,0,399,239`

- **Period** — This is used to, for example, represent a decimal point. Please be careful, as in some cases a mix of periods and commas will occur. Example: `DATA 3.14, 1.08, 36.5`

## Inserting and Overwriting Characters

There are two different modes used when inputting characters where there are existing characters: insert mode and overwrite mode. Pressing the INS key switches between these two modes.

**Insert mode:** When you input a character, it will be inserted at the cursor position. The character strings to the right of the cursor will all move to the right.

**Overwrite mode:** When you input a character, the character at the cursor position will be replaced with the new character.

## Deleting Characters

Use the BS key or DEL key to delete characters you have input.

1. **BS key** — Deletes the first character to the left of the cursor. The character strings to the right of the cursor will all move to the left.
2. **DEL key** — Deletes the first character to the right of the cursor (or in overwrite mode, the character the cursor is over). The character strings to the right of the cursor will all move to the left.

When in DIRECT mode, the UNDO key cannot be used to restore characters that have been overwritten or deleted using the BS or DEL keys. When in EDIT mode, the UNDO key can be used.

## Function Keys

At the top of the keyboard are five function keys which can be used to input frequently used instructions with a single touch. You can change the functions of each of the keys by using the `KEY` instruction.

Example: Change function key 3 to FILES

```sb3
KEY 3,"FILES"
```

## Program Read / Write Support Features

If you press the L button while the keyboard is being displayed, the support buttons for reading and writing program files will appear in the function key section. Press the LOAD button to open the file list dialog, from which you can select and LOAD files. Press the SAVE button to select and SAVE files. You can also check the name of the file that is currently loaded.

## Instruction Prediction Feature

This feature helps out when you're inputting instructions. When you input the first few letters of an instruction, a list of matches will be displayed.

1. **Possible Match Field** — Possible matches will appear in the space under the function keys.

2. **Input the first letter** — For example, if you press the G key, a list of instructions beginning with G will appear in the possible match field.

3. **Narrow down the selection** — Next, press the C key. The list of possible matches will be changed, narrowing it down to instructions beginning with GC. If you touch a possible match, it will be input.
