---
title: Conditional Judgment
slug: docs-sb3-manual-conditional-judgment
system: SmileBASIC 3
type: guide
topic: 27
source: e-manual.pdf
scraped: 2026-06-21
---

# Conditional Judgment

Now let's have a look at programs that use characters input from the keyboard, and ones that automatically change how processing is handled depending on certain conditions.

## Input Characters - INPUT

In the program shown on the previous page for finding the area of a circle, the value for the radius variable is fixed internally. Unless you modify the program, it cannot be used to calculate the area of a circle with a different radius value.

So, it'll be more convenient if you make it so that any value can be input externally (from the keyboard) for R in the first line, instead of it being fixed internally. The instruction used to achieve this is "INPUT."

When you run the program, you will see the following:

The `INPUT` instruction displays a "?" mark on the screen, and waits for a value to be input from the keyboard. Once a value is input, it is assigned to a variable (in this case, R).

To find the area of a circle with a radius of 3, input 3 and press the ENTER key.

### Display a guidance message for input

As soon as it's run, the program immediately displays "?", which will be confusing to people who aren't familiar with the program. To solve this problem, the `INPUT` instruction has a feature for displaying a guidance message.

The following program displays "What is the radius?" on the screen before waiting for the variable R to be input.

#### INPUT instruction summary

**Format**
```
INPUT "Guidance message"; Variable
```

- Guidance message is optional. (If omitted, the semicolon can also be omitted.)
- String variables are also allowed for the variable.

**Usage example**
```
INPUT "How old are you"; AGE
```

This instruction waits for a value to be input from the keyboard, and then assigns the input value to the variable AGE.

## Jump to a Specified Location - GOTO and Label

The program shown above exits after printing the area of one circle.

In order to make multiple calculations, it will be more convenient if the program returns to the first line once it has reached the last line. This can be achieved by using the "GOTO" instruction.

**Format**
```
GOTO Jump target label name
```

- The label name should begin with @ and consist of arbitrary alphanumeric characters (Example: @TOP).

**Usage example**
```
GOTO @TOP
```

This instruction forces a jump to the line with the label "@TOP".

Programs run lines in order, starting with the first. However, by using `GOTO`, you can force a program to jump to a specified line. You must assign a name called a "label" in advance to the line to be jumped to. If you put @ (at mark) at the beginning of a line, the line will be handled as a GOTO label.

In this program, a jump target label (@TOP) is prepared in the first line. This line only works as a sign, and performs no action itself. In the second to fifth lines, the radius is input, and the result is printed. Then, in the sixth line, the GOTO instruction forces a jump to the "@TOP" label, enabling the program to repeat the process from the top line.

Press START to force the running program to stop. The `GOTO` instruction will cause this program to always return to the top, so in order to stop it, please press START.

Next, let's modify the program so that it will stop automatically depending on the given input. In order to achieve this, conditional judgment is used.

## Conditional Judgment - IF...THEN

In BASIC, it is possible to check the value of a variable, and execute instructions only if the value meets a certain condition. The `IF...THEN` instruction is used for this purpose.

**Format**
```
IF Conditional expression THEN Instruction to execute
```

The conditional expression should be a comparison, such as A==0 or A>4

**Comparison operators:**

- `==` — Equal to
- `!=` — Not equal to
- `>` — Greater than
- `<` — Smaller than
- `>=` — Equal to or greater than
- `<=` — Equal to or smaller than

After "THEN," specify the instruction to execute once the condition is met

**Usage example**
```
IF A$=="YES" THEN PRINT "Bingo"
```

If the value of the variable A$ is "YES," this instruction displays "Bingo" and then proceeds to the next line. Otherwise, it does nothing, and simply proceeds to the next line.

Let's try modifying the previous program so that it will close when zero is input for "What is the radius?"
Insert the conditional judgment instruction after the radius is input.

`IF R==0 THEN END` means "if R is equal to 0, execute the `END` instruction" (close the program).

Please note that for comparisons, you must use "R==0" instead of "R=0." This is different from conventional BASIC usage.

## Terminate the Program - END Instruction

The `END` instruction, which is used in the above program, is an instruction used to close a running program. You may not need to use this instruction for programs without conditional branching, because they will terminate when they reach the last line.

**Format**
```
END
```

Terminate the program.

## Supplement: IF...THEN...ELSE...ENDIF

For users with programming knowledge

You can also use `IF...THEN...ELSE` in this product. Also, by using `ENDIF`, you can write instructions spanning multiple lines that address both cases where the condition is satisfied and where it is not satisfied.

**Format**
```
IF "Conditional expression" THEN
Instructions to be executed when the condition is met
[ELSE
Instructions to be executed when the condition is NOT met...]
ENDIF
```
