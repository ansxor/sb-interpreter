---
title: SPVAR
slug: docs-sb4-spvar
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-spvar
content_id: 19538
created: 2020-12-14
scraped: 2026-06-21
---

# SPVAR

Manage variables associated with sprites.

Each sprite has its own pool of *sprite variables*, allowing you to store /values/ associated with /keys/ "within" a sprite (an https://en.wikipedia.org/wiki/Associative_array[associative array].) The key can be either an integer or a string, and the value stored with it can be anything (even an array.) All unique keys refer to unique variables in the sprite, e.g. `0` and `"0"` are (necessarily) different.

## Set/Get

Set or get the value of a sprite variable.

```sbfunction
SPVAR spriteID%, key, value
SPVAR spriteID%, key OUT value
```

| Input | Description |
| --- | --- |
| `spriteID%` | ID of the target sprite. |
| `key` | The key of the target sprite variable. Can be an integer or a string. |
| `value` | The value associated with `key`. |

When reading an unset sprite variable, 0 is returned.

## Delete

Delete a sprite variable, or all sprite variables.

```sbsyntax
SPVAR spriteID% {, key }
```

| Input | Description |
| --- | --- |
| `spriteID%` | ID of the target sprite. |
| `key` | The key of the sprite variable to delete.<br>If omitted, all of the sprite's variables are deleted. |

## Examples

```sb4
'set the sprite variable "FOO" to 1 in sprite 0
SPVAR 0,"FOO",1
```

```sb4
'read "FOO" from sprite 0
PRINT SPVAR(0,"FOO")
```

```sb4
'delete "FOO"
SPVAR 0,"FOO"
```

```sb4
'delete them all
SPVAR 0
```

## Notes

### Keys 0-7 and `SPANIM`

In SmileBASIC 3, sprites only had 8 integer sprite variables, with the keys 0-7, set to 0 by default. These variables might be allocated or handled specially, but since all unset sprite variables are 0 by default they are functionally the same as any other.

Sprite variable 7, however, is the target of the `"V"` `SPANIM` animation target. This target sets or changes variable 7 by some integer value. If variable 7 is not an integer value, it will be set to 0 when the animation starts.
