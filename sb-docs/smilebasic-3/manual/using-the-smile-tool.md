---
title: Using the SMILE Tool
slug: docs-sb3-manual-using-the-smile-tool
system: SmileBASIC 3
type: guide
topic: 24
source: e-manual.pdf
scraped: 2026-06-21
---

# Using the SMILE Tool

Pressing the SMILE button on the keyboard will start the SMILE tool. You can use it to check information that's required by your programs. You can also run advanced tools, such as a map editor.

## SMILE Tool Screen

### Upper Screen

1. Allows you to check the previously registered definition number of a sprite character and its image. You can push the Circle Pad up or down to check information on other characters.

2. Shows the Z-coordinate of the display element. The farthest forward is -256, while the farthest back is 1024.

3. Shows the values and rotation directions that can be specified when rotating characters using the `ATTR` instruction.

4. Shows the numbers for each of the colors/background colors that can be specified with the `COLOR` instruction.

### Touch Screen

1. **List display field** — Displays the list for the entry selected out of options. Use the +Control Pad to select an entry from the list.

2. **Switch to the sound effect list** — Allows you to check the preset sounds used for `BEEP`. Press the A button to play back each entry.

3. **Switch to the BGM list** — Allows you to check the preset musical pieces for `BGMPLAY`. Press the A button to play back each entry. Press the Y button to stop.

4. **Switch to the MML instrument tone list** — Allows you to check the instrument numbers (equivalent to GM tone generators) that can be used with the `BGMPLAY` instruction. Press the A button to play back each entry. Press the Y button to stop.

5. **Switch to the sprite definition image list** — Allows you to check the currently defined SPDEF definition numbers and contents.

6. **Switch to the BG image list** — Allows you to check the currently defined BG images.

7. **Advanced editing tools:**
   - PAINT (Character creation)
   - MAP (BG screen creation)
   - ANIM. (Animation creation)
   - WAVE (Sampling and waveform editing)

   Please refer to the next section for more details.

8. **Exit the SMILE tool** — Exits the SMILE tool. You can also exit the tool by pressing the X button.

9. **Calculator input** — Allows you to move to a specified entry within the list. Input a number and press ENTER to change the currently selected entry to that list number.

## Advanced Editing Tools

These are the advanced editing tools that can be run from the SMILE tool. Each tool goes into file mode when the Y button is pressed, and closes when the X button is pressed.

### PAINT (Character creation)

This tool is used for creating character data for BG images and sprites. Choose colors and select tools as you draw characters in the EDIT area. Press the Y button to go into file mode, then input L and press ENTER to load the character data. Input S and press ENTER to save the data. The saved data can be used from your programs. (Example) `LOAD"GRP5:MYBG"`

### MAP (BG screen creation)

This tool is used for arranging BG characters to create BG screen data, for example a cityscape. Select characters from the character chart to paste them into the EDIT area. The saved data can be used from your programs by loading it into an array.

### ANIM. (Animation creation)

This tool is used for making adjustments to SPDEF definition contents and registering animation data for SPANIM. If you start an animation playback test, the program data in SLOT1 will be overwritten.

### WAVE (Sampling and waveform editing)

This tool is used for sampling from the microphone and using waveforms you have created as instrument sounds. A saved file can be used as an instrument sound for `BEEP` or `BGMPLAY` in the `WAVSET` instruction by loading it as an array.
