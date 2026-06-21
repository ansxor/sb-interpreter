# SmileBASIC documentation (Markdown)

Offline Markdown mirror of the SmileBASIC family reference, one consistent shape across
three language versions. 435 pages total. Conversion contract: **[STANDARD.md](STANDARD.md)**.

| System | Pages | Source |
| --- | --- | --- |
| [SmileBASIC 4](#smilebasic-4) | 112 | smilebasicsource.com (web) |
| [SmileBASIC 3 (3DS)](#smilebasic-3-3ds) | 248 cmd + 4 ref + 26 guide | `InstructionList.pdf` + `e-manual.pdf` (official) |
| [Petit Computer (PTC)](#petit-computer-ptc) | 45 | smilebasicsource.com (web) |

Every file carries YAML frontmatter and a link back to its source. The web systems were
converted from SmileBASIC Source's "12y"/"12y2" markup; SmileBASIC 3 commands were extracted
from `InstructionList.pdf`'s ruled-table grid and its guide pages from the `e-manual.pdf`
tutorial. See STANDARD.md for the details and per-source rules.

## SmileBASIC 4

`docs-sb4-*` · folder [`smilebasic-4/`](smilebasic-4/)

**Guides**

- [Arrays Guide](smilebasic-4/arrays-guide.md)
- [Functions Guide](smilebasic-4/functions-guide.md)
- [Layer Guide](smilebasic-4/layer-guide.md)
- [MML Guide](smilebasic-4/mml-guide.md)
- [Sprite Guide](smilebasic-4/sprite-guide.md)
- [Strings Guide](smilebasic-4/strings-guide.md)

**Reference tables**

- [Built-In Constants](smilebasic-4/constants.md)
- [Environment Support Table](smilebasic-4/environment-support-table.md)
- [Function List](smilebasic-4/function-list.md)
- [Keyword Table](smilebasic-4/keywords.md)
- [Operator Table](smilebasic-4/operators.md)

**Commands & functions**

- [`ACLS`](smilebasic-4/acls.md)
- [`ARYOP`](smilebasic-4/aryop.md)
- [`ASC`](smilebasic-4/asc.md)
- [`BEEP`](smilebasic-4/beep.md)
- [`BIN$`](smilebasic-4/bin.md)
- [`BIQUAD`](smilebasic-4/biquad.md)
- [`BREAK`](smilebasic-4/break.md)
- [`BREPEAT`](smilebasic-4/brepeat.md)
- [`BUTTON`](smilebasic-4/button.md)
- [`CALL`](smilebasic-4/call.md)
- [`CALLIDX`](smilebasic-4/callidx.md)
- [`CASE ~ WHEN ~ OTHERWISE ~ ENDCASE`](smilebasic-4/case.md)
- [`CHR$`](smilebasic-4/chr.md)
- [`COLOR`](smilebasic-4/color.md)
- [`CONTINUE`](smilebasic-4/continue.md)
- [`CONTROLLER`](smilebasic-4/controller.md)
- [`COPY`](smilebasic-4/copy.md)
- [`DATE$`](smilebasic-4/date.md)
- [`DEC`](smilebasic-4/dec.md)
- [`DIM()`](smilebasic-4/dim.md)
- [`DTREAD`](smilebasic-4/dtread.md)
- [`END`](smilebasic-4/end.md)
- [`EXEC`](smilebasic-4/exec.md)
- [`FILES`](smilebasic-4/files.md)
- [`FILL`](smilebasic-4/fill.md)
- [`FIND`](smilebasic-4/find.md)
- [`FLOAT`](smilebasic-4/float.md)
- [`FOR ~ TO ~ STEP ~ NEXT`](smilebasic-4/for.md)
- [`FORMAT$`](smilebasic-4/format.md)
- [`GPUTCHR`](smilebasic-4/gputchr.md)
- [`HEX$`](smilebasic-4/hex.md)
- [`IF ~ THEN ~ ELSEIF ~ ELSE ~ ENDIF`](smilebasic-4/if.md)
- [`INC`](smilebasic-4/inc.md)
- [`INKEY$`](smilebasic-4/inkey.md)
- [`INSERT`](smilebasic-4/insert.md)
- [`INSPECT / ??`](smilebasic-4/inspect.md)
- [`INSTR`](smilebasic-4/instr.md)
- [`INT`](smilebasic-4/int.md)
- [`LAST`](smilebasic-4/last.md)
- [`LAYER`](smilebasic-4/layer.md)
- [`LCLIP`](smilebasic-4/lclip.md)
- [`LEFT$`](smilebasic-4/left.md)
- [`LEN`](smilebasic-4/len.md)
- [`LFILTER`](smilebasic-4/lfilter.md)
- [`LMATRIX`](smilebasic-4/lmatrix.md)
- [`LOAD`](smilebasic-4/load.md)
- [`LOADV`](smilebasic-4/loadv.md)
- [`LOOP ~ ENDLOOP`](smilebasic-4/loop.md)
- [`MAINCNT`](smilebasic-4/maincnt.md)
- [`MBUTTON`](smilebasic-4/mbutton.md)
- [`METASAVE`](smilebasic-4/metasave.md)
- [`MID$`](smilebasic-4/mid.md)
- [`MILLISEC`](smilebasic-4/millisec.md)
- [`MOUSE`](smilebasic-4/mouse.md)
- [`NEW`](smilebasic-4/new.md)
- [`OPTION`](smilebasic-4/option.md)
- [`PRINT`](smilebasic-4/print.md)
- [`PROJECT`](smilebasic-4/project.md)
- [`PUSHKEY`](smilebasic-4/pushkey.md)
- [`Read-only Strings [Advanced]`](smilebasic-4/read-only-strings-advanced.md)
- [`RESIZE`](smilebasic-4/resize.md)
- [`RGB`](smilebasic-4/rgb.md)
- [`RIGHT$`](smilebasic-4/right.md)
- [`RND`](smilebasic-4/rnd.md)
- [`RUN`](smilebasic-4/run.md)
- [`SAVE`](smilebasic-4/save.md)
- [`SNDMVOL`](smilebasic-4/sndmvol.md)
- [`SNDSTOP`](smilebasic-4/sndstop.md)
- [`SPCHR`](smilebasic-4/spchr.md)
- [`SPCLR`](smilebasic-4/spclr.md)
- [`SPFUNC`](smilebasic-4/spfunc.md)
- [`SPHOME`](smilebasic-4/sphome.md)
- [`SPLINK`](smilebasic-4/splink.md)
- [`SPOFS`](smilebasic-4/spofs.md)
- [`SPROT`](smilebasic-4/sprot.md)
- [`SPSCALE`](smilebasic-4/spscale.md)
- [`SPSET`](smilebasic-4/spset.md)
- [`SPUNLINK`](smilebasic-4/spunlink.md)
- [`SPUSED`](smilebasic-4/spused.md)
- [`SPVAR`](smilebasic-4/spvar.md)
- [`STOP`](smilebasic-4/stop.md)
- [`STR$`](smilebasic-4/str.md)
- [`SUBHIDE`](smilebasic-4/subhide.md)
- [`SUBRUN`](smilebasic-4/subrun.md)
- [`SUBSHOW`](smilebasic-4/subshow.md)
- [`SUBST$`](smilebasic-4/subst.md)
- [`SUBSTOP`](smilebasic-4/substop.md)
- [`SWAP`](smilebasic-4/swap.md)
- [`TCOLOR`](smilebasic-4/tcolor.md)
- [`TIME$`](smilebasic-4/time.md)
- [`TMREAD`](smilebasic-4/tmread.md)
- [`TPAGE`](smilebasic-4/tpage.md)
- [`TPUT`](smilebasic-4/tput.md)
- [`TSCREEN`](smilebasic-4/tscreen.md)
- [`TYPEOF`](smilebasic-4/typeof.md)
- [`UIRUN`](smilebasic-4/uirun.md)
- [`UISTOP`](smilebasic-4/uistop.md)
- [`VAL`](smilebasic-4/val.md)
- [`Variables and Values`](smilebasic-4/variables.md)
- [`WHILE ~ WEND`](smilebasic-4/while.md)
- [`XSCREEN`](smilebasic-4/xscreen.md)

## SmileBASIC 3 (3DS)

`docs-sb3-*` · folder [`smilebasic-3/`](smilebasic-3/) · **full index: [smilebasic-3/README.md](smilebasic-3/README.md)**

Two official sources: the `InstructionList.pdf` command reference and the `e-manual.pdf`
*Handy Instruction Manual* tutorial.

- **248 instructions** (339 syntax forms) across 18 categories — see the [full index](smilebasic-3/README.md)
- **26 guide topics** (tutorial/concept walkthrough) in [`manual/`](smilebasic-3/manual/)
- **Reference tables:** [Constants](smilebasic-3/reference/constants.md) ·
  [Errors](smilebasic-3/reference/error-table.md) ·
  [System Variables](smilebasic-3/reference/system-variables.md) ·
  [MML](smilebasic-3/reference/mml.md)

## Petit Computer (PTC)

`docs-ptc-*` · folder [`petit-computer/`](petit-computer/) — the original Petit Computer reference.

**Reference tables**

- [Function overview](petit-computer/function.md)
- [Operator overview](petit-computer/operator.md)

**Commands & functions**

- [`ABS`](petit-computer/abs.md)
- [`ACLS`](petit-computer/acls.md)
- [`ASC`](petit-computer/asc.md)
- [`Background overview`](petit-computer/background.md)
- [`BGCHK`](petit-computer/bgchk.md)
- [`BGCLIP`](petit-computer/bgclip.md)
- [`BGCLR`](petit-computer/bgclr.md)
- [`BGCOPY`](petit-computer/bgcopy.md)
- [`BGFILL`](petit-computer/bgfill.md)
- [`BGOFS`](petit-computer/bgofs.md)
- [`BGPAGE`](petit-computer/bgpage.md)
- [`BGPUT`](petit-computer/bgput.md)
- [`BGREAD`](petit-computer/bgread.md)
- [`CHKCHR`](petit-computer/chkchr.md)
- [`CLS`](petit-computer/cls.md)
- [`COLOR`](petit-computer/color.md)
- [`Console overview`](petit-computer/console.md)
- [`GCOLOR`](petit-computer/gcolor.md)
- [`GLINE`](petit-computer/gline.md)
- [`GPAGE`](petit-computer/gpage.md)
- [`GPRIO`](petit-computer/gprio.md)
- [`GPSET`](petit-computer/gpset.md)
- [`Graphics overview`](petit-computer/graphics.md)
- [`GSPOIT`](petit-computer/gspoit.md)
- [`INPUT`](petit-computer/input.md)
- [`LINPUT`](petit-computer/linput.md)
- [`LOCATE`](petit-computer/locate.md)
- [`PRINT`](petit-computer/print.md)
- [`SPANGLE`](petit-computer/spangle.md)
- [`SPANIM`](petit-computer/spanim.md)
- [`SPCHR`](petit-computer/spchr.md)
- [`SPCLR`](petit-computer/spclr.md)
- [`SPHIT`](petit-computer/sphit.md)
- [`SPHITSP`](petit-computer/sphitsp.md)
- [`SPHOME`](petit-computer/sphome.md)
- [`SPOFS`](petit-computer/spofs.md)
- [`SPPAGE`](petit-computer/sppage.md)
- [`SPREAD`](petit-computer/spread.md)
- [`Sprite overview`](petit-computer/sprite.md)
- [`SPSCALE`](petit-computer/spscale.md)
- [`SPSET`](petit-computer/spset.md)
- [`SPSETV`](petit-computer/spsetv.md)
- [`VISIBLE`](petit-computer/visible.md)
