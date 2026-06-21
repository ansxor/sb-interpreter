---
title: Variables and Values
slug: docs-sb4-variables
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-variables
content_id: 19475
created: 2021-12-25
scraped: 2026-06-21
---

# Variables and Values

If you're making anything, you're gonna need variables? How else will you keep track of your character's health? You put it in a variable! But what goes in those variables? Values.

## What is a Variable?

You've seen it before. Don't pretend you haven't.

```sb4
A=1
```

The *variable*, `A`, is on the left, and the value here is `1`.  A variable isn't just a name, it's a *place to store a value.* When we say `A=1`, we aren't strictly saying that everywhere we write `A` it's the same as writing `1`, and we definitely aren't saying that everywhere we write `1` we mean `A`. What we're saying is that `A` now "contains" the value `1`.

## Assignment

The classic example is to say `A` is a box, and you put things into it. Putting a thing in the box is called *assignment*.

> {#sub !https://smilebasicsource.com/api/file/raw/25286
> Variable assignment: Putting a value in the box.}

The thing on the left *always* has to be a valid variable name (or a `VAR()` expression, which you'll see later), but the thing on the right can be any valid *expression*. An expression is just anything that produces a value, which could be

- any literal value,
- another variable,
- some other compound expression, like `2+2` or `RND(10)` or something else.

A variable name is valid if the variable exists, or has been declared (we'll get to it.) There are rules to choosing a variable name, though. A variable name

- must not start with a number,
- can only contain letters, numbers, and underscores.

Look at these variable names as examples of what is right and wrong.

```sb4
FOO            ' ok
FOO2           ' ok
_THING         ' ok
LONG_NAME      ' ok
_X242HQE_DRW3  ' ok, but nonsense
2              ' not ok, this is just a number
6TH_ENEMY      ' not ok, it starts with a number
444_444        ' not ok, still just a number
HELLO,FRIEND   ' not ok, commas are not allowed
```

SmileBASIC is case-insensitive, meaning it doesn't care if letters are uppercase or lowercase in language syntax. This means `A` and `a` both mean the same variable. Usually SmileBASIC code is written in all uppercase.

```sb4
A=10
PRINT a
```

Because a variable is a storage location, we can put different stuff into it whenever we want. "Variable" means "changing" after all.

```sb4
A=2
A=20.4
A="Bob"
```

Keep in mind that when you assign a variable multiple times, the previous value is replaced. You won't be able to get it back! Use multiple variables if you have to. Also notice that all of these assignments have a different type of value. SmileBASIC 4 is *dynamically typed.* Like most common scripting languages, variables can contain any type of value at any time. This is convenient, but it can become confusing in some cases, as you'll see later.

## Values

Speaking of values... you gotta have something to put in your variables! If variables are the boxes of the programming world, *values* are the "things." Values are data. They are *information itself.* Let's get into it.

### Types of Values

Every value has an associated *type*. A value wouldn't be useful to the program if nobody knew what it was supposed to be. That's what types are. As mentioned above, a variable can contain any type of value at any time, so variables do not have types. *Values have types, not variables.*

Broadly speaking, there are three "basic" types of values, corresponding types for storing collections of values (arrays), and one extra-special type that you don't need to usually worry about.

| Types | Types |
| --- | --- |
| Default/Empty | Default/Empty |
| Integer | Integer array |
| Real number | Real number array |
| String | String array |

Most types have some way of writing their values directly in code. Otherwise, using them would be... hard. These are called *literals*, and you'll see examples of them for each type soon.

### Numbers

There's nothing you can do without numbers. Numbers are the stuff of computers. It's why computers were invented! So of course, SmileBASIC has numbers.

```sb4
I=1
F=2.2
```

As you can see above, there's actually *two* types of numbers: *integers and real numbers.* You may have learned the difference at some point, but if you haven't: integers are whole numbers, and real numbers can contain stuff after the decimal point.

Why would you put them in different categories, though? As far as computer engineers care, there's really good reasons to. Even mathematicians do it. You'll find out why later.

#### Integers

Integer values can contain *signed whole numbers*, meaning they can be positive, negative, or zero, and can't have any fractional parts. Specifically, they are *two's complement signed 32-bit integers.*

*Integer literals* look mostly like how you see numbers written everywhere. There's some specific things to mention though. A literal cannot contain any commas or spaces, like you might see them written. You just have to write numbers without them.

```sb4
1
-20
365
1000
' can't do this
1,000
1 250
```

If SmileBASIC sees a `.` dot in a number, it treats it as the decimal point, separating the integer part from the fractional part, and treats that literal as a real number. So if you live somewhere that uses `.` as the thousands separator, keep in mind it's the decimal separator here.

The `-` negative sign is actually an *operator* that returns the negative of its right hand side. In number literals you can think of it as a normal negative sign, but keep in mind it means more than that. It's not actually part of a number literal.

```sb4
A=10
PRINT -A  ' -10
```

Integer literals can also be written in *binary* or *hexadecimal* notation. This relates to the "two's complement signed" thing I mentioned before. This way, you can write integers exactly how they're represented in memory, which is helpful sometimes. Some programming tasks are more convenient when working with binary directly.

A binary literal starts with `&b` or `&B` and contains only the digits `0` or `1`. A hexadecimal literal starts with `&h` or `&H` and contains only the digits `0` through `9`, and the letters `A` through `F`.

```sb4
25       ' 25
&b11001  ' 25 in binary
&h19     ' 25 in hex
```

All of the above literals represent the same value: the integer 25. They're just written in different notations. Binary is how all information in a computer is stored, and hexadecimal is a base-16 number system used by programmers and computer engineers as a convenient way to write binary numbers. If you need to know how to use them, you can find guides online.

Because integers are 32-bit, binary literals can contain a maximum of 32 digits, and hex literals can contain a maximum of 8 (one hex digit is four bits.)

#### Real Numbers

This brings us to the real numbers.  A real number, specifically, is an [IEEE 754 double-precision binary floating-point number.](https://en.wikipedia.org/wiki/IEEE_754) This is a lot for a beginner to grasp, but you don't need to know what that is. It just helps to know that this is a standard data type, so if something is confusing, you can try to look it up.

A floating-point number can contain an extremely wide range of fractional values. Its range of numbers is actually much more than the integer type, even ignoring the fact that it can store fractional digits too.

Real number literals are a lot like integer literals, just with a decimal point.

```sb4
1.0
3.1415
-20.001
19.99
```

If you're wondering, "hey, aren't 1.0 and 1 the same thing?" Then you're asking a good question. The answer is that they're treated as equal, but they're different data types. `1` and `1.0` are strictly different values. SmileBASIC treats all whole number, binary, and hex literals as integers, and all other number literals as reals. You probably won't even notice the difference, but we'll get to why later.

If, for whatever reason, you want to represent a whole number as a real (it's not uncommon) you can just add `.0` to the end, or even `#`.

```sb4
1
1.0
1#
```

The first `1` is an integer, but the other two are reals, even if they all *represent* the same number in some way.

---

If you need to know even more about numbers than what is written here, you should check out the Math section of the documentation.

### Strings

The third basic data type is the *string.* A string value contains any number of text characters (even none.) A string literal is any text written within double quotes.

```sb4
"Hello!"
"I am a string"
"      a lot   of    spaces       "
```
