---
title: PUSHKEY
slug: docs-sb4-pushkey
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-pushkey
content_id: 19521
created: 2020-05-06
scraped: 2026-06-21
---

# PUSHKEY

Push text to the key buffer or send special control codes. This function is primarily used by the software keyboard (`#SYS/SOFTKEY.PRG`) and is intended for simulating keyboard input. The text will be entered to the focused text entry (`INPUT`, the editor etc.) or can be read using `INKEY$`. The key buffer has a maximum size of 128 characters.

## Syntax

```sbsyntax
PUSHKEY text$
PUSHKEY charCode%
```

| Input | Description |
| --- | --- |
| `text$` | A string of text to push to the key buffer; max length 63 characters. |
| `charCode%` | A character code to push to the buffer as text (0-65535).<br>If the value is in the range 65536-69631, it is interpreted as a *control code*. |

## Examples

```sb4
PUSHKEY "Hello"  'push "Hello" to the key buffer
PUSHKEY 65       'push the character "A" to the key buffer
PUSHKEY 65539    'move the text cursor right (control code)
```

## Control Codes

If the `charCode%` parameter is from 65536 to 69631 it is interpreted as a *control code*. Control codes perform certain actions in the environment (usually related to the editor and key shortcuts.) They are not treated like normal key presses or text characters; they do not enter the key buffer and are processed immediately. It appears that SmileBASIC can only process one control code per frame. *This feature is not officially documented.*

### Control Code Table

An `ENUM` of control codes is listed in `#SYS/SOFTKEY.PRG`, under the comment "KEYBOARD CONTROL CODES." This table has been duplicated here and translated for convenience. The Name column refers to the names of the `ENUM` constants and the Number column is the number of the control code, which is value of that constant; if you want to use the constants by name, you will have to copy the control code `ENUM` from `#SYS/SOFTKEY.PRG` and paste it in your program.

| Name | Value | Description |
| --- | --- | --- |
| `#CK_UP` | 65536 | Move text cursor up |
| `#CK_DOWN` | 65537 | Move text cursor down |
| `#CK_LEFT` | 65538 | Move text cursor left |
| `#CK_RIGHT` | 65539 | Move text cursor right |
| `#CK_BS` | 65540 | Backspace key |
| `#CK_DEL` | 65541 | Delete key |
| `#CK_PAGEUP` | 65542 | Page Up key |
| `#CK_PAGEDOWN` | 65543 | Page Down key |
| `#CK_TAB` | 65544 | Tab key |
| `#CK_LINETOP` | 65545 | Go to beginning of line |
| `#CK_LINEEND` | 65546 | Go to end of line |
| `#CK_LINEJUMP` | 65547 | Open the "Go to line" prompt in the editor |
| `#CK_FILETOP` | 65548 | Go to top of file |
| `#CK_FILEEND` | 65549 | Go to end of file |
| `#CK_INSLINE` | 65550 | Insert blank line at cursor |
| `#CK_DELLINE` | 65551 | Delete line at cursor |
| `#CK_DELRIGHT` | 65552 | Delete text from the cursor to the end of the line |
| `#CK_UNDO` | 65553 | Editor undo |
| `#CK_REDO` | 65554 | Editor redo |
| `#CK_SELECTSTART` | 65555 | Start text selection |
| `#CK_SELECTEND` | 65556 | End text selection |
| `#CK_COPY` | 65557 | Copy selected text to clipboard |
| `#CK_CUT` | 65558 | Cut selected text to clipboard |
| `#CK_PASTE` | 65559 | Paste text from clipboard |
| `#CK_SMARTDEL` | 65560 | Smart Delete? "選択個所を削除 Delete selected location" |
| `#CK_RUN` | 65561 | Go to Direct mode |
| `#CK_EDIT` | 65562 | Go to editor (last used slot) |
| `#CK_EDIT0` | 65563 | Go to editor (slot 0) |
| `#CK_EDIT1` | 65564 | Go to editor (slot 1) |
| `#CK_EDIT2` | 65565 | Go to editor (slot 2) |
| `#CK_EDIT3` | 65566 | Go to editor (slot 3) |
| `#CK_STOP` | 65567 | Stop running program |
| `#CK_EXIT` | 65568 | Go to Top Menu |
| `#CK_TOOL` | 65569 | Run SmileTool 1 |
| `#CK_TOOL2` | 65570 | Run SmileTool 2 |
| `#CK_TOOL3` | 65571 | Run SmileTool 3 |
| `#CK_SYSREQ` | 65572 | Run / stop program (SysReq key) |
| `#CK_LOAD` | 65573 | Open Simple Load menu |
| `#CK_SAVE` | 65574 | Open Simple Save menu |
| `#CK_LISTERR` | 65575 | Acts like `LIST ERR` |
| `#CK_SOFTKEY` | 65576 | Toggle the software keyboard/UI program display |
| `#CK_ESCAPE` | 65577 | Escape key |
| `#CK_FONT` | 65578 | Cycle editor font |
| `#CK_WRAP` | 65579 | Toggle editor line wrap |
| `#CK_SPLIT` | 65580 | Editor split screen (toggle? enable?) |
| `#CK_SPLIT_SINGLE` | 65581 | Disable editor split screen |
| `#CK_SPLIT_VERTICAL` | 65582 | Editor split screen (vertical) |
| `#CK_SPLIT_HORIZONAL` | 65583 | Editor split screen (horizontal) |
| `#CK_TAB_INC` | 65584 | Increase indent |
| `#CK_TAB_DEC` | 65585 | Decrease indent |
| `#CK_COMMENTOUT` | 65586 | Comment text |
| `#CK_UNCOMMENTOUT` | 65587 | Uncomment text |
| `#CK_SCROLL_UP` | 65588 | Scroll up without moving cursor |
| `#CK_SCROLL_DOWN` | 65589 | Scroll down without moving cursor |
| `#CK_SCROLL_LEFT` | 65590 | Scroll left without moving cursor |
| `#CK_SCROLL_RIGHT` | 65591 | Scroll right without moving cursor |
| `#CK_SCROLL_PAGEUP` | 65592 | Page up without moving cursor |
| `#CK_SCROLL_PAGEDOWN` | 65593 | Page down without moving cursor |
| `#CK_HELP` | 65594 | Toggle help menu |
| `#CK_HELP_ON` | 65595 | Open help menu |
| `#CK_HELP_OFF` | 65596 | Close help menu |
| `#CK_HELP_PREVHEADER` | 65597 | Go to previous header in help menu (unused) |
| `#CK_HELP_NEXTHEADER` | 65598 | Go to next  header in help menu (unused) |
| `#CK_HELP_COPY` | 65599 | Copy code sample from help menu (unused) |
| `#CK_HELP_UP` | 65600 | Scroll help menu up (unused) |
| `#CK_HELP_DOWN` | 65601 | Scroll help menu down (unused) |
| `#CK_HELP_UPD` | 65602 | Help menu update? (unused) |
| `#CK_FIND_REPL` | 65603 | Toggle Find & Replace mode |
| `#CK_FIND_REPLALL` | 65604 | Replace all (unused) |
| `#CK_FIND_REPLNEXT` | 65605 | Replace next (unused) |
| `#CK_FIND_REPLSWITCH` | 65606 | Switch between Find and Replace entry |
| `#CK_FIND_PREV` | 65607 | Go to previous match |
| `#CK_FIND_NEXT` | 65608 | Go to next match |
| `#CK_FINDMODE` | 65609 | Toggle Find mode |
| `#CK_FOCUS_NEXT` | 65610 | Focus next (editor panel?) |
| `#CK_FOCUS_PREV` | 65611 | Focus previous (editor panel?) |
| `#CK_FOCUS_MAIN` | 65612 | Focus main program |
| `#CK_FOCUS_SUB` | 65613 | Focus subprogram |
| `#CK_FUNC1` | 65614 | Paste F1 key string |
| `#CK_FUNC2` | 65615 | Paste F2 key string |
| `#CK_FUNC3` | 65616 | Paste F3 key string |
| `#CK_FUNC4` | 65617 | Paste F4 key string |
| `#CK_FUNC5` | 65618 | Paste F5 key string |
| `#CK_INSERT` | 65619 | Switch Insert / Overwrite mode |
| `#CK_RESET` | 65620 | IO Reset (Ctrl+Alt+Del) |
| `#CK_SOFTKEY_ON` | 65621 | Show software keyboard/UI program |
| `#CK_SOFTKEY_OFF` | 65622 | Hide software keyboard/UI program |
| `#CK_TRACE` | 65623 | Acts like `TRACE` |
| `#CK_PERFGAUGE` | 65624 | Toggle the performance gauge |
| `#CK_SPLIT_GUIDE` | 65625 | "Split screen for guide" (unused since 4.1) |
| `#CK_COMPLETION_OFF` | 65626 | Disable text completion popups (unused since 4.1) |
| `#CK_SOFTKEY_RIGHT` | 65627 | Show software keyboard on right side of screen |
| `#CK_SOFTKEY_DOWN` | 65628 | Show software keyboard on lower side of screen |
| `#CK_SOFTKEY_LEFT` | 65629 | Show software keyboard on left side of screen |
| `#CK_SOFTKEY_UP` | 65630 | Show software keyboard on upper side of screen |
| `#CK_ERRJUMP` | 65631 | Jump to error line in editor (different from `LIST ERR`?) |
| `#CK_EDIT4` | 65632 | Go to editor (slot 4) |
| `#CK_EDIT5` | 65633 | Go to editor (slot 5) |

### Unknown Constants

Some constants are commented out or marked "Not compatible?" in the code. The table below is speculative; these constants will not work.

| Name | Description |
| --- | --- |
| `#CK_CAPTURE` | May have activated Switch screen capture |
| `#CK_BACKTRACE` | Acts like `BACKTRACE`? |
| `#CK_CONT` | Acts like `CONT`? |

### Notes

- The documentation states the input string has a maximum length of 64 characters, but it is actually 63. This is likely a bug.
- It is unknown if the rest of the control codes in the valid range do anything.
- `#CK_SPLIT_HORIZONAL` is misspelled. It should be `#CK_SPLIT_HORIZONTAL`.
