---
title: Managing Programs
slug: docs-sb3-manual-managing-programs
system: SmileBASIC 3
type: guide
topic: 21
source: e-manual.pdf
scraped: 2026-06-21
---

# Managing Programs

Programs you create can be saved to an SD card.

## SAVE Instruction - Save a Program

Use the "SAVE" instruction in DIRECT mode to save programs.

Format: `SAVE "File_name"`

- The only characters that can be used in file names are alphanumeric characters and _ (underscore).

1. As an example, let's try saving a program as "TEST1".

2. A confirmation message will appear on the Touch Screen.

   - Yes — Begins saving the file.
   - No — Does not save the file.

   Do not turn off the power or remove the SD card while files are being saved. Please wait until the operation finishes and a completion message appears.

3. Once saving has finished, a completion message will appear. Please press OK.

### If a file with the same name already exists

If a file with the same name already exists in the SD card, a confirmation message will appear.

- Yes — Saves the file, overwriting the existing file. Overwriting will cause the contents of the previous file to be lost. Please be careful, as this operation cannot be undone.
- No — Does not save the file.

### Application: Save another program SLOT

The `SAVE` instruction saves the contents of SLOT0. If you want to save the program contained in SLOT1, please input "PRG1:" before the file name to specify the SLOT number.

In the same way, you can save the contents of SLOT2 or of SLOT3 by specifying "PRG2:" or "PRG3:" respectively.

### Supplementary: Support feature for saving programs

When you want to SAVE the program currently being edited, you can also do so by pressing the SAVE button, which appears when you press the L button while the keyboard is being displayed on the Touch Screen, and then selecting the file to save.

## LOAD instruction - Load a Program

You can use the `LOAD` instruction to load and run programs you have saved.

When you load a program, the program you are currently inputting in SLOT0 will be overwritten and lost.

Format: `LOAD "File_name"`

- File name specified when the file was saved

1. As an example, let's try loading a file called "TEST1".

2. A confirmation message will appear on the Touch Screen.

   - Yes — Begins loading the file.
   - No — Does not load the file.

   Do not turn off the power or remove the SD card while files are being loaded. Please wait until the operation finishes and a completion message appears.

3. Once loading has finished, a completion message will appear. Please press OK.

### Application: Load into another program SLOT

The `LOAD` instruction loads a program into SLOT0. However, by inputting "PRG1:" before the file name, you can load the program into SLOT1.

In the same way, inputting "PRG2:" or "PRG3:" will load the program into SLOT2 or SLOT3 respectively.

## FILES Instruction - Display a File List

You can use the `FILES` instruction to display a list of the files saved in the SD card.

Format: `FILES`

- Display a list of files saved in the SD memory card on the console screen

## DELETE Instruction - Delete a File

You can use the `DELETE` instruction to delete files saved in the SD card.

You cannot restore files once they have been deleted. Please be very careful not to input wrong file name by mistake.

Format: `DELETE "File_name"`

- File name specified when the file was saved

1. As an example, let's delete a file called "TEST1".

2. A confirmation message will appear on the Touch Screen.

   - Yes — Deletes the file.
   - No — Does not delete the file.

   Do not turn off the power or remove the SD card while files are being deleted. Please wait until the operation finishes and a completion message appears.

3. Once deletion has finished, a completion message will appear. Please press OK.
