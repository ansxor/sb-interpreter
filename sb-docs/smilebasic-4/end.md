---
title: END
slug: docs-sb4-end
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-end
content_id: 19516
created: 2020-11-04
scraped: 2026-06-21
---

# END

The `END` keyword has two meanings based on context: ending the program and ending a function body.

## End of Program

If an `END` statement on its own is used, the program ends

```sb4
PRINT "Will see this"
END
PRINT "Won't see this"
```

The program ends automatically when the end of the code is reached, so `END` is not always necessary.

## End of Function

`END` is also used to end a function definition.

```sb4
DEF MYFUNC
 PRINT "HELLO"
END  '<-- end of function
```

Note that this means `END` cannot be used inside of a function to end the program. In some situations this can result in a `Syntax error`.

```sb4
DEF KILL
 PRINT "The program is now over!!!!"
 END  '<-- BASIC thinks the function ends here
END  '<-- BASIC thinks the program ends here
```

If you *must* end the program from within a function, you will have to use `STOP`.
