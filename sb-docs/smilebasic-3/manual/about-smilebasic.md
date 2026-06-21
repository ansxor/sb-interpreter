---
title: About SmileBASIC
slug: docs-sb3-manual-about-smilebasic
system: SmileBASIC 3
type: guide
topic: 9
source: e-manual.pdf
scraped: 2026-06-21
---

# About SmileBASIC

SmileBASIC is a tool that allows you to easily write programs on a Nintendo 3DS system. As it is also compatible with the system's 3D mode, you can create programs that utilize the 3D feature.

## User Agreement

Programs and resources such as images created using this product can be made open to the public using the Publish feature, allowing large numbers of people to view and run them. Please be sure not to publish any content that other people may find offensive, or content that reveals personally identifiable information or violates the rights of others (including portrait rights, privacy rights, and copyrights).

Anyone who performs improper acts that may cause public nuisance, or who publishes obscene or libelous images, risks punishment in accordance with the applicable laws and regulations.

SmileBoom Co.Ltd. assumes no responsibility for any issues resulting from information or programs published by its customers.

Please note that if we receive a report from a customer that they are offended by a published project, we may delete said project unconditionally, without any prior procedure such as confirming with the relevant creator. We thank you for your understanding.

## Precautions regarding BASIC compatibility

Please also refer to the "Standard BASIC Specification" page.

The BASIC language used in this product is not compatible with pre-existing versions of BASIC. Please also note that this product is not compatible with our Petit Computer and Petit Computer mkII products. Please make sure to pay attention to the differences in syntax when attempting to port programs from these products.

This product uses double-precision real-type numbers or integers to represent values internally. This may cause errors in binary number calculations, and therefore the product should not be used for applications that require precise calculations.

Branch instructions giving line numbers cannot be used. Instead of the line number, you should first assign a label (name tag) beginning with `@` to the branch target line, and then specify that label.

Programs that perform complex calculations or display a large amount of visual data on-screen can lead to slowdowns.

If instructions that write to files, such as `SAVE` and `DELETE`, are used repeatedly, it may take longer to read or write these files.

Although the program edit feature does not have a restriction on the length of a single line, input will no longer be possible once the memory reaches its limit.

Array variables must be declared with the `DIM` instruction before they are used. Omitting the declaration will cause an error.

For array variable parentheses, `[]` should be used instead of `()`. `()` should be used for specifying the priority of calculations, and for specifying the arguments of functions.

The assignment instruction `LET` has been abandoned in this product. You should only use the `=` symbol in the format "Variable name=Expression."

The exponentiation calculation symbols `^` and cannot be used. Please use the `POW()` function.

As in other programming languages, conditional expressions in IF statements are represented using `==` for "equals" and `!=` for "not equals." Please pay particular attention to the differences from conventional BASIC usage, where `=` is often used for "equals" and `<>` for "not equals." For assignment, `=` should be used.

In FOR ... NEXT instructions, this product determines the conditional statement first. In cases such as `FOR I=0 TO -1`, if the conditional statement is not satisfied in STEP1, the program will skip the content of FOR and carry on executing the instructions after NEXT. Please be aware that, unlike in conventional BASIC usage, the instructions in a FOR ... NEXT loop are not guaranteed to be executed at least once.

Variable names such as "NEXT I" cannot be specified in the NEXT instruction. Such specifications will not cause errors, but will work in the same way as when simply specifying "NEXT."

Control variables in ON ... GOTO and ON ... GOSUB should be started from 0, not 1.

The `INT()` function in conventional BASIC usage corresponds to the `FLOOR()` function in this product.

When you require only the integer part in values gained from calculations involving divisions, you should use the `FLOOR()` function to obtain the integer part. Accumulated errors in calculations such as coordinate calculations will cause a subtle deviation.

The `RND()` function returns integer values. If you require real-type values, the `RNDF()` function should be used.

The graphics instructions in this product do not use `()` or `-`. Example for other products: `LINE(0.0)-(639,399)` Example for this product: `GLINE 0,0,399,239`
