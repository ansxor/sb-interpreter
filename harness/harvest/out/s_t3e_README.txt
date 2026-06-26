s_t3e harvest (sb-interpreter-cj8) — XON/XOFF/COMMON, real SB 3.6.0 via Azahar, 2026-06-26.

CANONICAL case files: harness/harvest/cases/s_t3e_xon{,_err,_str}.txt, s_t3e_call_func_form.txt
Slotcopy scripts:       .claude/skills/sb-oracle/tools/s_t3e_common_harvest.py, s_t3e_call_func_harvest.py

OUTPUTS:
  s_t3e_xon.tsv            RESULT after XON/XOFF (boots 1; EXPAD->1; MOTION/MIC/XOFF leave boot 1)
  s_t3e_xon_err.tsv        error edges: XON / XON FOO / XON 1 / XOFF / XOFF FOO -> errnum 3
  s_t3e_xon_str.tsv        string-literal feature: XON "EXPAD"/"MOTION" NOERR; bad -> 3
  s_t3e_call_func.tsv      V=CALL("name",args) -> errnum 3 (no function form); OUT form works
  s_t3e_common.tsv         cross-slot COMMON: callable, args+OUT, no-USE->16, private->16, globals 0,999
  s_t3e_xon_disambig*.tsv  dead-end probes (RESULT=0 errors; DIALOG/MPSTART don't zero RESULT in Azahar)
                           -> RESULT boots 1 and no on-device path zeroes it; only EXPAD->1 is asserted

KEY FINDINGS:
  - RESULT boots TRUE (1). XON EXPAD -> RESULT 1 (documented). MOTION/MIC/XOFF don't observably change it.
  - XON/XOFF accept a string-literal feature name on real 3.6.0 (osb 3.5.0 rejects it -> divergence).
  - Real SB has NO value-returning function form of CALL: V=CALL("F",1) -> errnum 3 (filed ufi).
  - Cross-slot COMMON: USE required (else 16), non-COMMON DEF private (16), globals bind to defining
    slot (slot-1 top-level G=111 does NOT run on CALL -> load-time 0; caller's G=999 untouched -> "0,999").
