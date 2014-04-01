- Start Date: 2014-04-01
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Use different keywords for declaring tagged unions and C-style enums.

For example:

~~~rust
enum AB { A = 1, B }
enum CD { C, D(int) }
~~~

would become:

~~~rust
enum AB { A = 1, B }
union CD { C, D(int) }
~~~

# Motivation

## Demonstration that the `enum` keyword is overloaded

The following examples show how tagged union style enums and C-style enums
provide different functionality. This contributes to the idea that the current
`enum` construct provides overloaded functionality.

### Discriminants

Discriminants can only be supplied for C-style enums:

~~~rust
enum AB { A = 1, B(int) }
~~~

gives the following error on compilation:

~~~
test.rs:1:25: 1:26 error: discriminator values can only be used with a c-like enum
test.rs:1 enum AB { A = 1, B(int) }
                                  ^
~~~

### Casting

C-style enums can be cast:

~~~rust
enum AB { A, B }

fn main() {
    print!("{:?}", B as int);
}
~~~

Tagged union style enums can't be cast:

~~~rust
enum AB { A, B(int) }

fn main() {
    print!("{:?}", A as int);
}
~~~

~~~
test.rs:4:20: 7:31 error: non-scalar cast: `AB` as `int`
test.rs:4     print!("{:?}", A as int);
                             ^~~~~~~~~~~
~~~

## Justification for the separation of tagged union and C-style enum declaration

### Overloading functionality can cause confusion

Whilst it is often desirable to simplify the language by unifying features,
taking this too far can result in confusion. The examples shown under the
heading _Demonstrating that the `enum` keyword is overloaded_ show that tagged
unions and C-style enums declare types with different enough semantics that
they may warrent separate declaration keywords.

### Simplifying terminology

By separating the constructs under different keywords, the awkward 'tagged
union style enum' and 'C-style enum' terms could be dropped in preference of
referring to their keywords, `union` and `enum` when talking in the context
of Rust. This would make discussions far easier - for example on IRC or Github.

In [5.2 Enums](http://static.rust-lang.org/doc/master/tutorial.html#enums),
C-style enums and tagged unions are currently introduced in the same section.
In fact C-style enums are introduced _first_, despite them being used far less
in ordinary Rust code (usually only in FFIs). By separating the constructs,
C-style enums could then be de-emphasised and mentioned further down the page.
Do note that this change could be made without introducing separate keywords,
but doing so makes placing them under different headings far more natural.

### Reduce the barriers of entry for more sceptical programmers with a systems programming background

Whilst the intention behind using `enum` was to make C and C++ developers more
at home, its most common usage (the declaration of tagged unions), can be
confusing at first, and may create a barrier for entry for some users.

Below is an exchange on #ada. Whilst the person in question may have been
overly antagonistic and somewhat close minded, it is illustrative of the
confusion that some users have when first trying to understand the semantics of
Rust's `enum` construct:

~~~
<Lucretia> had a skim through the rust tutorial, not convinced
<bjz> what do you mean?
<Lucretia> enums as Lists? what drugs are they on?
<Lucretia> see the tutorial
<bjz> oh the linked list tutorial
<Lucretia> the way to implement a list with an enum
<bjz> have you used haskell?
<Lucretia> at uni
<bjz> or an ML?
<Lucretia> didn't get it
<Lucretia> ope
<bjz> an enum is a sum type
<Lucretia> was given a haskell tutorial from here
<bjz> or a 'variant type'
<bjz> or tagged union
<bjz> lots of names for it
<bjz> http://en.wikipedia.org/wiki/Tagged_union
<bjz> using enums for lists isn't really ideomatic in Rust
<bjz> but they are very useful for things like abstract syntax trees
<Lucretia> yeah, I wouldn't say an enum is that at all and neither does that link
<bjz> you can think of them as nini dynamic type systems, but you have to check
  what type it is before you can do operations on them
<bjz> what do you mean?
<Lucretia> you say an enum is a tagged_union, that link does not say that at
  all, just searched for enum on that page - it uses an enum to determine the
  type of a variant record
<bjz> yeah, I don't like the use of the 'enum' keyword either
<bjz> but the semantics are the interesting bit
<Lucretia> but to say that an enum and a list go together in the way they do
  is just wrong
<Lucretia> I remember "Cons" from uni - the term only, the meaning, not at all
<bjz> think of it like this: you can express what the semaintics of a C or Java
  enum with rust's enum
<bjz> but you can also express a lot more
<bjz> C/Java's enum is a subset of what you can do with Rust's enum
<Lucretia> but an enumeration is a set of values, that's it, it's not a list,
  no matter how you twist things, it's just not
<Lucretia> it's an orthogonal and separate concept
<bjz> the is a enumerated set of *types*
<bjz> I would highly recommend learning some haskell
<bjz> it would probably make lots of this stuff more clear
<bjz> lists and trees spring naturally out of sum types
<bjz> (no matter what you call them)

...

<darkestkhan> bjz: one thing: enumeration ≠ tagged type
<darkestkhan> bjz: enumerated type is discrete type that has certain set of
  values - nothing more and nothing less
~~~

## Justification for the use of the `union` keyword

### Terms that refer to tagged unions

A number of terms correspond to the behaviour of tagged union style enums:

- _sum type_ (the technical term that is most commonly used in type theory literature)
- _algebraic data type_ (Note that this term is used in type theory to refer to
  _any_ kind of composite datatype. This includes _product types_, _records_,
  and _sum types_)
- _variant type_
- _tagged union_
- _enumerated type_ (this usually refers only to [C-style enumerations](http://msdn.microsoft.com/en-us/library/whbyts4t.aspx),
  ie. unions containing only nullary variants with integer discriminants)

### Keywords used in other languages

- `data`: [Haskell](http://www.haskell.org/haskellwiki/Algebraic_data_type),
  Idris, [Agda](http://wiki.portal.chalmers.se/agda/pmwiki.php?n=ReferenceManual.Data)
- `datatype`: [SML](http://en.wikipedia.org/wiki/Standard_ML#Algebraic_datatypes_and_pattern_matching)
- `enum`: [Haxe](https://en.wikipedia.org/wiki/Haxe#Enumerated_types)
- `type`: [Ocaml](http://caml.inria.fr/pub/docs/u3-ocaml/ocaml-core.html#htoc19)
- `union`: [C](http://en.wikipedia.org/wiki/Union_type#C.2FC.2B.2B),
  [C++](http://www.cplusplus.com/doc/tutorial/other_data_types/#unions),
  [D](http://dlang.org/enum.html) (note that these declare un-tagged unions, and are unsafe)
- `variant`: [Visual Basic](http://msdn.microsoft.com/en-us/library/office/gg251448%28v=office.15%29.aspx),
  [Boost.Variant (C++)](http://www.boost.org/doc/libs/1_55_0/doc/html/variant.html),
  [Nemerle](https://en.wikipedia.org/wiki/Nemerle#Variants)

### Why the `union` keyword is preferred

Although `union` declares an _untagged union_ in C and C++, it is reasoned that
this term is most familiar to this group of programmers, those of whom
constitute a large section of the Rust's target audience. `union` is also five
characters long, which is consistent with Rust's other keywords.

Alternative keywords can be rejected for the following reasons:

- `sum` is pretty much out of the picture as is is too vague.
- `sumtype` would look out of place with the current keywords, being multi-word
  and too long (over five characters). While it is the most accurate of all the
  keywords when viewed through the lens of type theory, the term is not as
  common in programming circles.
- `data` and `type` imply that the declaration of full algebraic data types is
  supported, where as the language construct only supports sum types.
- `datatype` is too long, and the reasoning for `data` and `type` also hold.
- As stated before, `enum` causes confusion because the semantics associated
  with tagged union style enums is extremely different to the semantics
  associated with the keyword in C. Using the keyword for declaring tagged
  unions only limited precedent.

# Detailed design

As shown in the summary, the following:

~~~rust
enum AB { A = 1, B }
enum CD { C, D(int) }
~~~

would become:

~~~rust
enum AB { A = 1, B }
union CD { C, D(int) }
~~~

## Description in the language tutorial

Currently is a description of using `enum` for declaring tagged unions that
seems to be targeted at targeted at C and C++ developers:

> The run-time representation of such a value includes an identifier of the actual form that it
> holds, much like the "tagged union" pattern in C, but with better static guarantees.
> 
> ...
> 
> All of these variant constructors may be used as patterns. The only way to access the
> contents of an enum instance is the destructuring of a match.

Here, "'tagged union' pattern", refers to unions declared using C's `union`
construct, discriminated by type tag (usually declared using the `enum`
keyword). The description could instead read:

> The `union` keyword provides safe language support for the the "tagged
> union" pattern commonly found in C or C++. In order to enforce safety, the
> only way to access the contents of a `union` instance is via pattern matching
> against the variants.

Here is an example of describing `enum` to C developers:

> `enum`, like in C, provides support for groups of constants discriminated by
> integer values. Unlike C, the resulting type name is not an alias to an
> integer type, rather it is a new type that can only be equal to one of the
> declared variants.

# Alternatives

## Continue to use the `enum` keyword for both C-style enums and tagged unions

Changing the keyword now will cause some pain for current users of the language,
and it could contribute the public perception of instability.

`enum`, despite not being a widely used keyword for tagged unions/sum types,
still makes *some* sense if you think of them as '[enumerated types](http://en.wikipedia.org/wiki/Enumerated_type)',
even though this term is used far less for referring to tagged unions.

## Change the `enum` keyword whilst retaining the overloaded behaviour

Incrementing the keyword count of the language could be seen as adding complexity.
The keyword could be renamed to `union` to improve clarity for the most common
use case. There is overlap in functionality when tagged unions only have nullary
variants (although casting variants to their integer discriminants is very rare
in code that does not interface with C FFIs.)

## Use a alternative keyword to `union`

We could use another keyword listed under the _Keywords used in other languages_
heading. However, do note the arguments raised in _Why the `union` keyword is preferred_.

# Unresolved questions

...
