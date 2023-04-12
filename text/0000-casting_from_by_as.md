- Feature Name: `casting_from_by_as`
- Start Date: 2023-04-12
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)


# Summary
[summary]: #summary

This proposal adds more abilities to as_cast operator for conversion into Types, which have by `From` (and `TryFrom`) Trait Implementations.

The new operator `as'`(as-prime) is added, which is a synonym to as_cast, but it has low precedence.


# Motivation
[motivation]: #motivation

As_cast operator has very limited use in Rust today: it is a cost-less type cast operator from primitive types.

But as_cast has a **huge** potential as **smart** transmute operator for every non-general `From` (and `TryFrom`) implementations!


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation


## As Casting

(A) Allow to convert any non-general type `SomeType` of `ImplTraitType` (but not `Result<T,E>`) cast implementation of `From` Trait:

```rust
let foo = "my string" as String;

// which desugars into
let foo = String::from("my string");
```

(B) Alternatively allow also to use for non-general Result-Types like `Result<SomeType,SomeErrorType>` cast implementation of `TryFrom` Trait:
```rust
let foo = 5i32 as Result<u32,TryFromIntError>;

// which desugars into
let foo = u32::try_from(5i32);
```

The wildcard is still a valid option:
```rust
let foo = 5i32 as Result<u32,_>;
let bar : String = "my string" as _;
```


## As-Prime Casting

Unfortunately, as_cast has has huge disadvantage of using too many brackets in expressions, because it has strong precedence = 13.

So, it is important to add a new operator as_prime `as'` (or its alternatives, like  `as$` / `as#`, ..)

This is a new keyword, but fully backward-compatible.

Operator as_prime is a synonym for `as`, but it has one of the lowest precedents = 3 (a bit stronger than assignments operators) and it has Let_to_Right associativity like as_cast.
```rust
let foo = 1 as Foo + "two" as _ as$ Bar; // as' or as# or as$ or a$ or ast

// which desugars into
let foo : Bar = 1 as Foo + "two" as Foo as# Bar;

// which desugars into
let foo : Bar = ((1 as Foo) + ("two" as Foo)) ast Bar;

// which desugars into
let foo : Bar = Bar::from(Foo::from(1) + Foo::from("two"));
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation


# Drawbacks
[drawbacks]: #drawbacks

The new additional keyword `as'` - as_prime_cast is needed.


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

(B) Alternative of implementing casting from both `From` and `TryFrom` Traits, is implementing casting from just one Trait - `From`.

(C) As_prime operator could have another name
- `as'` (as_prime)
- `as#` (in my opinion it is the best choice)
- `as$` or `a$`
- `ast` (as_type)  or `astp`
- `asw` (as_weak) or `asl`(as_late)


# Prior art
[prior-art]: #prior-art

None known.


# Unresolved questions
[unresolved-questions]: #unresolved-questions

None known.


# Future possibilities
[future-possibilities]: #future-possibilities

None known.

