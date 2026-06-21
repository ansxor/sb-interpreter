---
title: BIQUAD
slug: docs-sb4-biquad
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-biquad
content_id: 19497
created: 2020-09-03
scraped: 2026-06-21
---

# BIQUAD

mono/stereo biquadratic filter with funny filter coefficient array

```sbfunction
BIQUAD input[], output[], filterCoefficients[]
```

nicole drawing

![](https://smilebasicsource.com/api/File/raw/23626)

reimplementation

```
DEF BIQUAD2 _OU,_IN,F
 VAR L = LEN(_IN)
 'make temporary arrays with 2 extra elements at the start
 DIM IN[L+2],OU[L+2]
 COPY IN,2,_IN
 'fill first 2 elements of temp arrays with old values from F
 IN[0] = F[6]
 IN[1] = F[5]
 OU[0] = F[8]
 OU[1] = F[7]
 '
 VAR I
 FOR I=2 TO L+2-1
  OU[I] = F[0]*IN[I] + F[1]*IN[I-1] + F[2]*IN[I-2] - F[3]*OU[I-1] - F[4]*OU[I-2]
 NEXT
 'store last 2 elements of arrays in F
 F[6] = IN[I-2]
 F[5] = IN[I-1]
 F[8] = OU[I-2]
 F[7] = OU[I-1]
 'return output
 COPY _OU,OU,2,L
END
```

note: even if the input/output arrays are the same array, it creates a copy before doing the operations, so the values in the input array will never be updated during the calculations

F[0..4] - filter parameters
F[5..8] - last values of previous calculation
F[9..12] - (same as 5-8, for stereo mode)

### Examples

(assumes no OPTION DEFINT):

"OOH I BET YOU CAN USE BIQUAD TO GENERATE THE FIBONACCI SEQUENCE LOL"

(I think this skips the first 2 values though, starting 1 2 3 5 8 ...)

```
DIM F[13]
F[4]=-1:F[3]=-1
'output[n] = 0 - -1*output[n-1] - -1*output[n-2]
DEF FIB(N)
 DIM A[N]
 F[8]=1 'output[-2] = 1
 F[7]=0 'output[-1] = 0
 BIQUAD A,A,F
 RETURN A
END
```

generating a list from 0 to n-1:

```
DIM F[13]
F[4]=1:F[3]=-2
'output[n] = -2*output[n-1] - 1*output[n-2]

DEF RANGE(N)
 DIM A[N]
 F[8]=-2 'output[-2] = -2
 F[7]=-1 'output[-1] = -2
 BIQUAD A,A,F
 RETURN A
END
```

array sum:

```
DIM F[13]
      :       :F[0]=1
      :F[3]=-1
'output[n] = 1*input[n] - -1*output[n-1]
DEF SUM(A)
 DIM TEMP[LEN(A)]
 F[7]=0 'output[-1] = 0
 BIQUAD TEMP,A,F
 RETURN TEMP[LEN(A)-1]
END
```

low pass filter 8000Hz on 48kHz stereo data

```sb4
DIM OU[2,1000],IN[2,1000],FP[13]
BQPARAM FP,#BQLPF,48000,8000,1/SQR(2)
BIQUAD OU,IN,FP

'equivalent to setting biquad filter parameters:
DIM OU[2,1000],IN[2,1000],FP[13]
'FP[6]=0:FP[5]=0
FP[2]=0.15505102:FP[1]=0.31010205:FP[0]=0.15505102
FP[4]=0.24040818:FP[3]=-0.62020403
'FP[8]=0:FP[7]=0
BIQUAD OU,IN,FP
```
