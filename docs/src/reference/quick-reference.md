# Quick reference

## Bind and define

```que
(let name value)
(mut name value)
(alter! name value)
(letrec name (lambda (args) body))
(sig name Type)
(enum Name Variant...)
```

## Control

```que
(if test yes no)
(cond test value ... default)
(when test body...)
(unless test body...)
(while test body...)
(loop i test body...)
(block body...)
(do body...)
```

## Vectors and tuples

```que
[a b c]
{a b}
(length xs)
(get xs i)
(set! xs i value)
(push! xs value)
(pop! xs)
(empty! xs)
(capacity! xs n)
```

## Functions and composition

```que
(lambda (x) body)
(lambda (x y) body)
(|> value (map f) sum)
(<| value f g)
(comp f g h)
(apply f arg...)
```

## Static inspection

```sh
que file.que --debug
que file.que --opt
que file.que --emit types
que explain file.que --opt
que fmt file.que
que --lib types 'pattern*'
```

For the complete current library surface, `que --lib types` is the reference.
