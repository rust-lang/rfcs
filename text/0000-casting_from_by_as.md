- Feature Name: `casting_from_by_as`
- Start Date: 2023-04-12
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)


# Summary
[summary]: #summary

This proposal adds more abilities to as_cast operator for conversion into Types, which have `From` (and `TryFrom`) Trait Implementations.

The new operator `as'`(as-prime) is added, which is a synonym to as_cast, but it has low precedence. 

And maybe another one `as!`(as-bang), which is a synonym to as_cast, but it has high precedence.


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

let baz = 12u32 as Ipv4Addr;

let bar = 42 as Rc<i32>;

// which desugars into
let foo = String::from("my string");

let baz = Ipv4Addr::from(12u32);

let bar = Rc::from(12);
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

let baz = 12i32 as Rc<_>;

let foo = 199 as Result<NonZeroU32,_>;
```


## As-Prime Casting

Unfortunately, as_cast has has huge disadvantage of using too many brackets in expressions, because it has strong precedence = 13.

(A) So, it is important to add a new operator as_prime `as'` (or its alternatives, like  `as$` / `as#`, ..) that cast the whole expression.

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

(C) Alternatively a new cousin of `as` should be added, that cast values before referencing.

Let call it `as!`(as_bang), it has strong precedence = 15 (stronger than `&` reference operator)
```rust
let foo = & 5 as! i32;
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Default behavior for as_cast remains (pseudo-code):

```rust
// default
a as T where T == T_as_castable   ~   a as T; // nothing changes
```

But we also add additional sugaring if next pattern matches (pseudo-code):
```rust
// new (A)
a as T    ~   T::from(a);
(a : U) as T<U>  ~  T::from(a);

// new (B)
a as Result<T,_>   ~   T::try_from(a);
(a : U) as Result<T<U>,_>   ~   T::try_from(a);
```

# Drawbacks
[drawbacks]: #drawbacks

The new additional keyword `as'`(weak_as) - as_prime_cast is needed. Maybe two keywords are needed, if `as!` (strong_as) would added.


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

(B) Alternative of implementing casting from both `From` and `TryFrom` Traits, is implementing casting from just one Trait - `From`.

(D) As_prime operator could have another name
- `as'` (as_prime)
- `as#` (in my opinion it is the best choice)
- `as$` or `a$`
- `as!`
- `ast` (as_type)  or `astp`
- `asw` (as_weak) or `asl`(as_late)
- `asst` (as_strong)


# Prior art
[prior-art]: #prior-art

None known.


# Unresolved questions
[unresolved-questions]: #unresolved-questions

None known.


# Future possibilities
[future-possibilities]: #future-possibilities

None known.

