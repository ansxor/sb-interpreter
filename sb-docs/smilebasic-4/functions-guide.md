---
title: Functions Guide
slug: docs-sb4-functions-guide
system: SmileBASIC 4
type: guide
source: https://smilebasicsource.com/forum/thread/docs-sb4-functions-guide
content_id: 19517
created: 2020-11-17
scraped: 2026-06-21
---

# Functions Guide

Functions are an essential part of most procedural-style programming languages, and SmileBASIC is no exception. If you're a beginner, with no concept of what a function *is*, that information might not help much, but this page aims to explain what a function is in concept, and how they behave in SmileBASIC in particular.

## What is a Function?

In abstract, a function is basically a named procedure in the code that you can "call" at any time to perform its function. Even as a beginner, you use them all the time. See these statements: look familiar?

```sb4
ACLS
GLINE 0,0,100,100,#C_RED
DTREAD OUT YEAR,MONTH,DAY
RND(100)
```

All of these are *function calls.* The function is named at the start of the call, its parameters are the comma-separated list of expressions, and any return values come after `OUT` (or are the result of the expression, but we'll get there.)

So, a function is just a piece of code with a name attached to it, essentially. When you write `ACLS`, you're making a call to the `ACLS` function, which runs its "body", and then your program's control flow returns and goes onto the next statement. As you can see they can be passed *arguments* as input and produce *return values.* The concept is really simple, actually.

## Arguments and Return Values

Values passed into a function are called *arguments.* An argument can be any valid expression (which includes literals, variables, labels etc.) The arguments are passed into the function via a comma-separated list of expressions.

```sb4
GLINE 0,0,100,100,#C_RED
     '^----------------^----these are the arguments
```

All functions (or variants of built-in functions) have a specific number of arguments (with exception of the occasional variadic.) Built-in functions additionally usually have specific argument types, unlike the rest of SB4 which is dynamically typed.

*Return values* are values that a function produces as output. Functions in SB4 can return any number of values, up to 255, or no return values at all, which is fairly unique as far as popular languages go. Like arguments, the number of return values is usually fixed.

## Function Call Forms

Broadly speaking, there's three forms of function call in SmileBASIC, based on the number of return values.

### Function Statements (No Return Values)

If a function has no return values, it's called in this simple form. The function call stands on its own as a single statement, alongside its argument list, if necessary.

```sb4
'ACLS, with no arguments
ACLS
'drawing a line on screen with GLINE
GLINE 0,0,100,100,#C_RED
```

### Function Expression (One Return Value)

Functions with one return value are most commonly called as expressions. Here, the argument list is written inside a pair of parentheses after the function name. This is a valid expression, and its value is the return value of the function.

```sb4
'print a random number
PRINT RND(10)
'assign a random number to a variable
VAR DICE=RND(6)+1
```

In the above example `RND(number)` is a function call to `RND`, with `number` being its argument. As you can see, this function call form can be used anywhere a normal expression can be used, and can be a sub-expression, so it's extremely useful.

Note that this form is *only* an expression and just an expression does not count as a standalone statement in SB4, so these functions cannot be called on their own, or without something to receive the return value.

```sb4
RND(10)  'this is illegal
```

### OUT Form (Multiple Return Values)

If a function has *more than one* return value, you must call it in `OUT` form. The `OUT` keyword is written after the argument list, and a comma-separated list of variable names (the "return list") is written after that, corresponding to the number of return values. The variables in this list are assigned the corresponding return values from the function.

```sb4
'read the current date
DTREAD OUT YEAR,MONTH,DAY
PRINT YEAR
PRINT MONTH
PRINT DAY
'find something inside of a 2D array
FIND ARRAY,VALUE OUT INDEX1,INDEX2
```

In fact, the `OUT` form is *actually* the universal form of calling a function. It works with functions of any number of arguments or return values, even when working with user-defined functions. The other two calling forms are actually just special-case syntax for functions with zero or one return value.

```sb4
'no-return functions are special case of OUT with no return variables
ACLS
GLINE 0,0,100,100,#C_RED
'''
ACLS OUT
GLINE 0,0,100,100,#C_RED OUT
```

```sb4
'function expressions are a special case of OUT with one return variable
DICE=RND(6)
RND 6 OUT DICE
```

## Overloads

Many built-in functions have multiple variants, called *overloads*, which are chosen based on the number of arguments and return values, and sometimes the type of some of the arguments. Overloads are included to provide a variety of functionality to the built-ins without having to use many different, unique function names.

For example, `ACLS` has a default form with no arguments at all, and an overload that allows you to pass flags controlling its functionality.

```sb4
ACLS
ACLS #TRUE,#TRUE,#FALSE
```

`SPSET` has a form with no return value, for setting a specific sprite ID, or a form with one return value, to automatically choose an unused sprite ID.

```sb4
SPSET 0,0
ID=SPSET(1)
```

The benefits of overloading manifest as optional arguments, variations of specific functions, getter/setter functions, and so on. It can't all be explained here, and it's best understood by exploring all of the various built-in functions.

## Variadic Functions

Some functions are defined to take *any* number of arguments or return values, these are called *variadic functions*, or just variadics for short. The most typical example is `MAX` and `MIN`.

```sb4
PRINT MAX(1,2,3,4,5)
PRINT MIN(1,2,3,4,5)
```

## User-Defined Functions

Using a `DEF` block, you can define your own functions. There is an entire page of this documentation dedicated to `DEF`; go there for more details on usage.

However, `DEF`s are not *exactly* like built-in functions. You can't define overloads and the parameters are not type-checked, but some of this can be circumvented by using variadics or manual type-checking.

## CHKCALL and CALL

You can check if a specific function exists and is callable using `CHKCALL`.

```sb4
PRINT CHKCALL("ACLS")  'true
PRINT CHKCALL("FSADFAFSFD")  'false
```

Plus, you can call any function using its name in a string with the `CALL` keyword.

```sb4
'ACLS is called here
CALL "ACLS"
```

Of course, this is most powerful when used with string variables, expressions, or libraries of user-defined functions, allowing code that can perform different function calls at runtime based on certain conditions.
