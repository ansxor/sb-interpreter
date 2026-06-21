---
title: "PRINT" and Variables
slug: docs-sb3-manual-print-and-variables
system: SmileBASIC 3
type: guide
topic: 25
source: e-manual.pdf
scraped: 2026-06-21
---

# "PRINT" and Variables

The following will introduce the basic instructions required for writing programs. Please make sure to read it if you're new to programming.

## Print Characters - PRINT Instruction

"PRINT" is an important instruction that displays characters on the screen. Please try inputting the following program.

Once you have input the program, run it in DIRECT mode.

The `PRINT` instruction prints character strings enclosed in double quotations as-is.

### How semicolons (;) and commas (,) work

Input ; (semicolon) to the right of "HELLO" on the first line, and then run the program.

"HELLO" and "A" have now been joined together.

Normally, the `PRINT` instruction causes a line break to occur automatically after printing the specified character string. However, if you add a semicolon (;), subsequent characters will follow directly after the printed string.

Next, change the semicolon (;) to a comma (,).

If you use a comma (,), subsequent characters will be printed after a set space.

## The Difference between "A" and A - Understanding Variables

What will happen if you forget to insert double quotations when you were supposed to input `PRINT "A"`?

0 is displayed. This is because A without double quotations means "variable A."

### What is a variable?

Computers contain lots of memory, which can store numbers and characters in individual pieces.

In BASIC, pieces of memory that store values are managed by giving them names. A piece of memory that has been given a name is called a "variable."

`PRINT A` without double quotations means "print the contents of the variable A" instead of "print the character A". In this case, the contents of the variable A happened to be 0, so 0 was printed.

More details about variables are explained on the next page.

## Print Characters from a Chosen Location - LOCATE Instruction

You can use the `LOCATE` instruction to specify the location (coordinates) at which characters should be printed with the `PRINT` instruction.

Format: `LOCATE X-coordinate, Y-coordinate`

- The X-coordinate specifies the number of characters to the right (0-49)
- The Y-coordinate specifies the number of characters down (0-29)

The following program prints HELLO at the position X=10, Y=3.

### The depth of characters can also be specified

You can also specify the depth location (Z-coordinate) to display at. This can be used to achieve 3D effects when in 3D mode.

Format: `LOCATE X-coordinate, Y-coordinate, Z-coordinate`

- The Z coordinate specifies the depth (positive values = into the screen, zero = on the 3D screen surface, negative values = in front of the screen)
- For depth into the screen, a value in the range 0 to 1024 should be specified, and for depth in front of the screen, a value in the range 0 to -256 should be specified
