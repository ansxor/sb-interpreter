---
title: Environment Support Table
slug: docs-sb4-environment-support-table
system: SmileBASIC 4
type: reference
source: https://smilebasicsource.com/forum/thread/docs-sb4-environment-support-table
content_id: 19451
created: 2020-10-27
scraped: 2026-06-21
---

# Environment Support Table

| |
| --- |
| Page Under Construction |

In SmileBASIC 4,  there are various environments code can run within. They are:

- *Main*, the primary environment for running and creating programs.
- *Direct Mode*, the command line in the editor view, attached to the Main environment,
- *Subprogram*, used to run tool programs simultaneous with the Main program or editor,
- *UI*, used to run the software keyboard, autocomplete, and help menu.

Some functions have restrictions or behave differently when in certain environments, so these tables are here to show you what works where. Note that Direct Mode is essentially just a substate of the Main environment, but some functions and keywords are not allowed in Direct Mode, and there are special DIrect Mode commands, so it is included for completeness. Additionally, language keywords are omitted if they work everywhere, since they are core features.

| Legend | |
| --- | --- |
| OK | Function works completely in this environment. |
| Partial | Function has limited support or restrictions in this environment.<br>See the note referenced by the superscript, or the documentation page, for details. |
| Dummied | Function does nothing in this environment, but will not cause any issues. |
| Error | Function raises a SmileBASIC error when used in this environment. |
| Broken | Function will cause crashes or strange issues when used in this environment.<br>*Probably a bug. Report issues to SmileBoom.* |
| ? | Untested. |

| Function Name | Direct | Main | Sub | UI |
| --- | --- | --- | --- | --- |
| `NEW` | OK | Error | Error | Error |
| `CLEAR` | OK | Error | Error | Error |
| `LIST` | OK | Error | Error | Error |
| `RUN` | OK | Error | Error | Error |
| `CONT` | OK | Error | Error | Error |
| `TRACE` | OK | Error | Error | Error |
| `BACKTRACE` | OK | Error | Error | Error |
| `SUBRUN` | OK | Error | Error | Error |
| `SUBSTOP` | OK | Error | Error | Error |
| `SUBSHOW` | OK | Error | Error | Error |
| `SUBHIDE` | OK | Error | Error | Error |
| `UIRUN` | OK | Error | Error | Error |
| `UISTOP` | OK | Error | Error | Error |
| `DEF` | Error | OK | OK | OK |
| `DEFARG` | Error | OK | OK | OK |
| `DEFARGC` | Error | OK | OK | OK |
| `DEFOUT` | Error | OK | OK | OK |
| `DEFOUTC` | Error | OK | OK | OK |
| `COMMON` | Error | OK | OK | OK |
| `OPTION` | Error | OK | OK | OK |
| `END` | Error | OK | OK | OK |
| `STOP` | Error | OK | OK | OK |
| `EXEC` | Error | Partial<sup>2</sup> | Partial<sup>2</sup> | Partial<sup>2</sup> |
| `TYPEOF` | OK | OK | OK | OK |
| `WAIT` | OK | OK | OK | ? |
| `TMREAD` | OK | OK | OK | ? |
| `DTREAD` | OK | OK | OK | ? |
| `DATE$` | OK | OK | OK | ? |
| `TIME$` | OK | OK | OK | ? |
| `CHKLABEL` | OK | OK | OK | OK |
| `CHKCALL` | OK | OK | OK | ? |
| `CHKVAR` | OK | OK | OK | ? |
| `RESULT` | OK | OK | OK | ? |
| `PERFBEGIN` | OK | OK | OK | ? |
| `PERFEND` | OK | OK | OK | ? |
| `FREEMEM` | OK | OK | OK | ? |
| `MILLISEC` | OK | OK | OK | ? |
| `SYSPARAM` | OK | OK | OK | OK |
| `INT` | OK | OK | OK | OK |
| `FLOAT` | OK | OK | OK | OK |
| `FLOOR` | OK | OK | OK | OK |
| `ROUND` | OK | OK | OK | OK |
| `CEIL` | OK | OK | OK | OK |
| `ABS` | OK | OK | OK | OK |
| `SGN` | OK | OK | OK | OK |
| `MIN` | OK | OK | OK | OK |
| `MAX` | OK | OK | OK | OK |
| `RND` | OK | OK | OK | ? |
| `RNDF` | OK | OK | OK | ? |
| `RANDOMIZE` | OK | OK | OK | ? |
| `SQR` | OK | OK | OK | OK |
| `EXP` | OK | OK | OK | OK |
| `LOG` | OK | OK | OK | OK |
| `POW` | OK | OK | OK | OK |
| `RAD` | OK | OK | OK | OK |
| `DEG` | OK | OK | OK | OK |
| `SIN` | OK | OK | OK | OK |
| `COS` | OK | OK | OK | OK |
| `TAN` | OK | OK | OK | OK |
| `ASIN` | OK | OK | OK | OK |
| `ACOS` | OK | OK | OK | OK |
| `ATAN` | OK | OK | OK | OK |
| `SINH` | OK | OK | OK | OK |
| `COSH` | OK | OK | OK | OK |
| `TANH` | OK | OK | OK | OK |
| `CLASSIFY` | OK | OK | OK | OK |
| `ASC` | OK | OK | OK | OK |
| `CHR$` | OK | OK | OK | OK |
| `VAL` | OK | OK | OK | OK |
| `STR$` | OK | OK | OK | OK |
| `HEX$` | OK | OK | OK | OK |
| `BIN$` | OK | OK | OK | OK |
| `FORMAT$` | OK | OK | OK | OK |
| `MID$` | OK | OK | OK | OK |
| `LEFT$` | OK | OK | OK | OK |
| `RIGHT$` | OK | OK | OK | OK |
| `INSTR` | OK | OK | OK | OK |
| `SUBST$` | OK | OK | OK | OK |
| `COPY` | OK | OK | OK | OK |
| `RINGCOPY` | OK | OK | OK | OK |
| `SORT` | OK | OK | OK | OK |
| `RSORT` | OK | OK | OK | OK |
| `PUSH` | OK | OK | OK | OK |
| `POP` | OK | OK | OK | OK |
| `UNSHIFT` | OK | OK | OK | OK |
| `SHIFT` | OK | OK | OK | OK |
| `LEN` | OK | OK | OK | OK |
| `FILL` | OK | OK | OK | OK |
| `ARRAY%` | OK | OK | OK | OK |
| `ARRAY#` | OK | OK | OK | OK |
| `ARRAY$` | OK | OK | OK | OK |
| `RESIZE` | OK | OK | OK | OK |
| `INSERT` | OK | OK | OK | OK |
| `REMOVE` | OK | OK | OK | OK |
| `FIND` | OK | OK | OK | OK |
| `ARYOP` | OK | OK | OK | ? |
| `FILES` | OK | OK | OK | OK |
| `LOAD` | OK | OK | OK | OK |
| `LOADG` | OK | OK | OK | OK |
| `LOADV` | OK | OK | OK | OK |
| `SAVE` | Partial<sup>2</sup> | Partial<sup>2</sup> | OK | OK |
| `SAVEG` | Partial<sup>2</sup> | Partial<sup>2</sup> | OK | OK |
| `SAVEV` | Partial<sup>2</sup> | Partial<sup>2</sup> | OK | OK |
| `DELETE` | Partial<sup>2</sup> | Partial<sup>2</sup> | Partial<sup>2</sup> | Partial<sup>2</sup> |
| `RENAME` | Partial<sup>2</sup> | Partial<sup>2</sup> | Partial<sup>2</sup> | Partial<sup>2</sup> |
| `PROJECT` (set) | OK | Error | Error | Error |
| `PROJECT` (read) | OK | OK | OK | OK |
| `CHKFILE` | OK | OK | OK | OK |
| `METALOAD` | OK | OK | OK | OK |
| `METASAVE` | OK | OK | OK | OK |
| `METAEDIT` | OK | OK | OK | OK |
| `ACLS` | OK | OK | OK | Broken |
| `XSCREEN` (set) | OK | OK | OK | Broken |
| `XSCREEN` (read) | OK | OK | OK | OK |
| `VSYNC` | OK | OK | OK | OK |
| `MAINCNT` | OK | OK | OK | OK |
| `DIALOG` | OK | OK | OK | OK |
| `BACKCOLOR` | OK | OK | OK | Dummied |
| `FADE` | OK | OK | OK | Dummied |
| `FADECHK` | OK | OK | OK | Dummied |
| `ANIMDEF` | OK | OK | OK | OK |
| `RGB` | OK | OK | OK | OK |
| `RGBF` | OK | OK | OK | OK |
| `HSV` | OK | OK | OK | OK |
| `HSVF` | OK | OK | OK | OK |
| `CALLIDX` | OK | OK | OK | OK |
| `LAYER` | OK | OK | OK | OK |
| `LFILTER` | OK | OK | OK | Broken |
| `LCLIP` | OK | OK | OK | OK |
| `LMATRIX` | OK | OK | OK | OK |
| `CLS` | OK | OK | OK | OK |
| `PRINT` | OK | OK | OK | OK |
| `TPRINT` | OK | OK | OK | OK |
| `INSPECT` | OK | OK | OK | OK |
| `INPUT` | OK | OK | OK | OK |
| `LINPUT` | OK | OK | OK | OK |
| `COLOR` | OK | OK | OK | OK |
| `LOCATE` | OK | OK | OK | OK |
| `ATTR` | OK | OK | OK | OK |
| `SCROLL` | OK | OK | OK | OK |
| `CHKCHR` | OK | OK | OK | OK |
| `TSCREEN` | OK | OK | OK | OK |
| `TPAGE` | OK | OK | OK | OK |
| `TCOLOR` | OK | OK | OK | OK |
| `TLAYER` | OK | OK | OK | OK |
| `TPUT` | OK | OK | OK | OK |
| `TFILL` | OK | OK | OK | OK |
| `THOME` | OK | OK | OK | OK |
| `TOFS` | OK | OK | OK | OK |
| `TROT` | OK | OK | OK | OK |
| `TSCALE` | OK | OK | OK | OK |
| `TSHOW` | OK | OK | OK | OK |
| `THIDE` | OK | OK | OK | OK |
| `TBLEND` | OK | OK | OK | OK |
| `TANIM` | OK | OK | OK | OK |
| `TSTART` | OK | OK | OK | OK |
| `TSTOP` | OK | OK | OK | OK |
| `TCHK` | OK | OK | OK | OK |
| `TVAR` | OK | OK | OK | OK |
| `TCOPY` | OK | OK | OK | OK |
| `TSAVE` | OK | OK | OK | OK |
| `TLOAD` | OK | OK | OK | OK |
| `TARRAY` | OK | OK | OK | OK |
| `TUPDATE` | OK | OK | OK | OK |
| `TFUNC` | OK | OK | OK | OK |
| `TCOORD` | OK | OK | OK | OK |
| `GTARGET` | OK | OK | OK | OK |
| `GCOLOR` | OK | OK | OK | OK |
| `GCLIP` | OK | OK | OK | OK |
| `GCLS` | OK | OK | OK | OK |
| `GPGET` | OK | OK | OK | OK |
| `GPSET` | OK | OK | OK | OK |
| `GLINE` | OK | OK | OK | OK |
| `GCIRCLE` | OK | OK | OK | OK |
| `GBOX` | OK | OK | OK | OK |
| `GFILL` | OK | OK | OK | OK |
| `GTRI` | OK | OK | OK | OK |
| `GPAINT` | OK | OK | OK | OK |
| `GPUTCHR` | OK | OK | OK | OK |
| `GPUTCHRP` | OK | OK | OK | OK |
| `GCOPY` | OK | OK | OK | OK |
| `GSAVE` | OK | OK | OK | OK |
| `GLOAD` | OK | OK | OK | OK |
| `GARRAY` | OK | OK | OK | OK |
| `GUPDATE` | OK | OK | OK | OK |
| `GSAMPLE` | OK | OK | OK | OK |
| `FONTINFO` | OK | OK | OK | OK |
| `SPSET` | OK | OK | OK | OK |
| `SPCLR` | OK | OK | OK | OK |
| `SPSHOW` | OK | OK | OK | OK |
| `SPHIDE` | OK | OK | OK | OK |
| `SPHOME` | OK | OK | OK | OK |
| `SPOFS` | OK | OK | OK | OK |
| `SPROT` | OK | OK | OK | OK |
| `SPSCALE` | OK | OK | OK | OK |
| `SPCOLOR` | OK | OK | OK | OK |
| `SPCHR` | OK | OK | OK | OK |
| `SPPAGE` | OK | OK | OK | OK |
| `SPLAYER` | OK | OK | OK | OK |
| `SPDEF` | OK | OK | OK | OK |
| `SPLINK` | OK | OK | OK | OK |
| `SPUNLINK` | OK | OK | OK | OK |
| `SPANIM` | OK | OK | OK | OK |
| `SPSTART` | OK | OK | OK | OK |
| `SPSTOP` | OK | OK | OK | OK |
| `SPCHK` | OK | OK | OK | OK |
| `SPVAR` | OK | OK | OK | OK |
| `SPCOL` | OK | OK | OK | OK |
| `SPCOLVEC` | OK | OK | OK | OK |
| `SPHITSP` | OK | OK | OK | OK |
| `SPHITRC` | OK | OK | OK | OK |
| `SPHITINFO` | OK | OK | OK | OK |
| `SPFUNC` | OK | OK | OK | OK |
| `SPUSED` | OK | OK | OK | OK |
| `SNDSTOP` | OK | OK | OK | ? |
| `BEEP` | OK | OK | OK | OK |
| `BEEPPAN` | OK | OK | OK | ? |
| `BEEPPIT` | OK | OK | OK | ? |
| `BEEPSTOP` | OK | OK | OK | ? |
| `BEEPVOL` | OK | OK | OK | ? |
| `BGMCLEAR` | OK | OK | OK | ? |
| `BGMCONT` | OK | OK | OK | ? |
| `BGMPAUSE` | OK | OK | OK | ? |
| `BGMPITCH` | OK | OK | OK | ? |
| `BGMPLAY` | OK | OK | OK | OK |
| `BGMSET` | OK | OK | OK | ? |
| `BGMSETD` | OK | OK | OK | ? |
| `BGMSTOP` | OK | OK | OK | ? |
| `BGMVAR` | OK | OK | OK | ? |
| `BGMVOL` | OK | OK | OK | ? |
| `BGMWET` | OK | OK | OK | ? |
| `BGMCHK` | OK | OK | OK | ? |
| `EFCEN` | OK | OK | OK | ? |
| `EFCSET` | OK | OK | OK | ? |
| `EFCWET` | OK | OK | OK | ? |
| `PCMCONT` | OK | OK | OK | ? |
| `PCMPOS` | OK | OK | OK | ? |
| `PCMSTOP` | OK | OK | OK | ? |
| `PCMSTREAM` | OK | OK | OK | ? |
| `PCMVOL` | OK | OK | OK | ? |
| `RECCHK` | OK | OK | OK | ? |
| `RECDATA` | OK | OK | OK | ? |
| `RECLEN` | OK | OK | OK | ? |
| `RECPOS` | OK | OK | OK | ? |
| `RECSAVE` | OK | OK | OK | ? |
| `RECSTART` | OK | OK | OK | ? |
| `RECSTOP` | OK | OK | OK | ? |
| `SNDMSBAL` (set) | Dummied | Dummied | OK | ? |
| `SNDMSBAL` (read) | OK | OK | OK | ? |
| `SNDMVOL` | OK | OK | Dummied | ? |
| `TALK` | OK | OK | OK | ? |
| `TALKSTOP` | OK | OK | OK | ? |
| `TALKCHK` | OK | OK | OK | ? |
| `WAVSET` | OK | OK | OK | ? |
| `WAVSETA` | OK | OK | OK | ? |
| `CHKMML` | OK | OK | OK | ? |
| `XCTRLSTYLE` | OK | OK | Dummied | Dummied |
| `CONTROLLER` | OK | OK | OK | ? |
| `BUTTON` | OK | OK | OK | OK |
| `BREPEAT` | OK | OK | OK | OK |
| `STICK` | OK | OK | OK | OK |
| `ACCEL` | OK | OK | OK | ? |
| `GYROV` | OK | OK | OK | ? |
| `GYROA` | OK | OK | OK | ? |
| `GYROSYNC` | OK | OK | OK | ? |
| `VIBRATE` | OK | OK | Dummied | Dummied |
| `TOUCH` | OK | OK | OK | OK |
| `MOUSE` | OK | OK | OK | OK |
| `MBUTTON` | OK | OK | OK | OK |
| `IRSTART` | OK | OK | Dummied | ? |
| `IRSTOP` | OK | OK | Dummied | ? |
| `IRSTATE` | OK | OK | Dummied | ? |
| `IRREAD` | OK | OK | Dummied | ? |
| `IRSPRITE` | OK | OK | Dummied | ? |
| `TCPIANO` | OK | OK | Partial<sup>1</sup> | Partial<sup>1</sup> |
| `TCROBOT` | OK | OK | Partial<sup>1</sup> | Partial<sup>1</sup> |
| `TCVISOR` | OK | OK | Partial<sup>1</sup> | Partial<sup>1</sup> |
| `TCHOUSE` | OK | OK | Partial<sup>1</sup> | Partial<sup>1</sup> |
| `TCFISHING` | OK | OK | Partial<sup>1</sup> | Partial<sup>1</sup> |
| `TCBIKE` | OK | OK | Partial<sup>1</sup> | Partial<sup>1</sup> |
| `TCCAR` | OK | OK | Partial<sup>1</sup> | Partial<sup>1</sup> |
| `TCPLANE` | OK | OK | Partial<sup>1</sup> | Partial<sup>1</sup> |
| `TCSUBM` | OK | OK | Partial<sup>1</sup> | Partial<sup>1</sup> |
| `TCVEHICLE` | OK | OK | Partial<sup>1</sup> | Partial<sup>1</sup> |
| `KEYBOARD` | OK | OK | OK | OK |
| `INKEY$` | OK | OK | OK | Dummied |
| `KEY` | OK | OK | OK | OK |
| `CLIPBOARD` | OK | OK | OK | OK |
| `PUSHKEY` | OK | OK | OK | OK |
| `PRGEDIT` | OK | OK | OK | ? |
| `PRGGET$` | OK | OK | OK | ? |
| `PRGSEEK` | OK | OK | OK | ? |
| `PRGSET` | OK | OK | OK | ? |
| `PRGINS` | OK | OK | OK | ? |
| `PRGDEL` | OK | OK | OK | ? |
| `PRGSIZE` | OK | OK | OK | ? |
| `PRGNAME$` | OK | OK | OK | ? |
| `BIQUAD` | OK | OK | OK | ? |
| `BQPARAM` | OK | OK | OK | ? |
| `FFT` | OK | OK | OK | ? |
| `IFFT` | OK | OK | OK | ? |
| `FFTWFN` | OK | OK | OK | ? |
| `XSUBSCREEN` | Dummied | Dummied | OK | ? |
| `ENVSTAT` | OK | OK | OK | OK |
| `ENVSTAT` (direct) | Dummied | Dummied | OK | Dummied |
| `ENVTYPE` | OK | OK | OK | OK |
| `ENVLOAD` | Dummied | Dummied | OK | ? |
| `ENVSAVE` | Dummied | Dummied | OK | ? |
| `ENVINPUT$` | OK | OK | OK | OK |
| `ENVFOCUS` | Dummied | Dummied | OK | ? |
| `ENVPROJECT` | Dummied | Dummied | OK | Dummied |
| `ENVLOCATE` | OK | OK | OK | OK |
| `HELPINFO` | OK | OK | OK | OK |
| `HELPGET` | OK | OK | OK | OK |
| `UISTATE` (write) | Dummied | Dummied | Dummied | OK |
| `UISTATE` (read) | OK | OK | OK | OK |
| `UIGETCMPL` | ? | ? | ? | OK |
| `UIPUSHCMPL` | ? | ? | ? | OK |
| `UIMASK` | Dummied | Dummied | Dummied | OK |
| `UIFOCUS` | Dummied | Dummied | Dummied | OK |
