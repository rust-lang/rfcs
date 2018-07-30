- Feature Name: numeric_literal_types
- Start Date: 2018-07-30
- RFC PR: _
- Rust Issue: _

# Summary
[summary]: #summary

This RFC introduces two new types: `ulit` and `flit`. These are the *numeric 
literal types*, i.e., the type of an integer literal `42` or a float literal
`1.0`. These types exist to give a name to literals that do not have a fixed
size. Consider the following error:
```
error: int literal is too large
 --> src/main.rs:2:32
  |
2 | const VEGETA_CANT_EVEN: u128 = 9_000_000_000_000_000_000_000_000_000_000_000_000_001;
  |                                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```
This expression could instead be given the type `ulit`, which could later
be narrowed into a "real" integer, or fed into a `const` constructor,
like `BigInt::new()`, without having to restrict itself to `u128`. These
types, while unsized, are capable of being coerced into integers,
following the current literal typing rules.

Introducing language-level bignums is an *non-goal*.
This RFC lays the groundwork for custom literals, but custom literals
themselves are *also* a non-goal.

Note: this proposal is given in full generality, with a series of weakened
subsets that might be easier to implement or stabalize. The guide-level
explanation is written only with this full generality in mind, since I don't
think it's too difficult to explain the weakenings. Accepting this RFC will
probably entail picking a weakening and applying it to both explanations.

# Motivation
[motivation]: #motivation

This proposal has a few motivating use cases:
- Untyped compile-time constants, as in Go or C (via `#define`).
- Custom integer literals, in particular bignums.

The former is valuable, because it allows us to hoist several occurences
of the same literal in different typed contexts, without having to type
it as the largest possible numeric type and explicitly narrow, i.e.
```rust
let foo = my_u8() & 0b0101_0101;
let bar = my_i32() & 0b0101_0101;
// becomes
const MY_MASK: ulit = 0b0101_0101;
let foo = my_u8() & MY_MASK;
let bar = my_i32() & MY_MASK;
// instead of
const MY_MASK: u128 = 0b0101_0101;
let foo = my_u8() & (MY_MASK as u8);
let bar = my_i32() & (MY_MASK as i32);
```
This can be emulated by a macro that expands to the given literal, but
that is unergonomic, and calling `MY_MASK!()` does not make it clear
that this is a compile-time constant (`ALL_CAPS` not withstanding).

The latter was not originally the main reason this was proposed, but perhaps
the stronger one. Custom literals need to take a number as input; while
C++, the only language with custom literals, simply takes its versions of
`u64` and `f64` as arguments for literals, this is an unnecessary restriction
in Rust, given that we recently stabalized the `u128` type. This problem
cannot be neatly worked around, as far as we know.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Consider the expression `42`. What's its type? The basic language introduction
might lead you to believe that, like in C++ and Java, it's `i32`. In reality,
the compiler assigns this expression the type `ulit`: the type of all *integer
literals*. Note the `u`: this is because all integer literals are unsigned!
The float equivalent is `flit`.

`ulit` and `flit` are both DSTs, so they can't be passed to functons like
normal values. Unlike other DSTs, however, if they are used in a `Sized` 
context, they will attempt to coerce into a sized integer type, defaulting
to `i32` or `f64` if there isn't an obvious choice. This occurs silently,
since one almost always wants a sized integer:
```rust
let x = 42; // 42 types as ulit, but since a let binding requires a 
            // Sized value, it tries to coerce to a sized integer. since
            // there isn't an obvious one, it picks i32. Hence, x: i32.

let y = 42u32; // 42u32 has type u32, so no coersion occurs.
let z: u32 = 42; // ulit coerces to u32, since it's the required type
```

Literal types are otherwise *mostly* like normal integers. They support
arithmetic and comparisons (but don't implement any std::ops traits, since
they're not Sized). Like any DST, they can be passed around behind references.
You can even write
```rust
const REALLY_BIG: &ulit = &1000000000000000000000;
// analogous to
const HELLO_WORLD: &str = "Hello, world!";
```
You can then use `REALLY_BIG` anywhere you'd use the literal, instead. The
reference types `&'static ulit` and `&'static flit` will automatically coerce 
into any numeric type, via dereferencing.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This RFC introduces two new DSTs: `ulit` and `flit`. Note that we do not
introduce `ilit`; this is because it is not possible to write down a literal
for a negative number, since `-1` is *really* `1.neg()`. These types behave
like most DSTs, with a few exceptions:

If either type is used in a context that requires a `Sized` type 
(a function call, a let binding, a generic paramter, etc), they will
coerce according to the current typing rules for literals: whatever
is infered as correct, or `i32`/`f64` as a fallback. Note that `as` casts
do what is expected.

For ergonomic reasons,
static references to either type are dereferenced automatically in `Sized`
context. This is to support the following pattern:
```rust
const FOO: &ulit = 0b0101_0101;

let x: u8 = 5;
let y = 5 & FOO; // here `FOO` is coerced from `&ulit` to `u8`
```

The representation of `ulit` and `flit` is unspecified, but this RFC suggests
representations. Note that the compiler is *not* required to use these;
they are merely a suggestion for what a good representation would be.
```rust
struct ulit(
    [u8]    // the bytes of the number, in the target
            // endianess. this is so a cast to a sized
            // type can be implemented as a memcpy
);

// represented in base-2 scientific notation, with the bytes
// for the mantissa and exponent back-to-back.
// this way, we avoid having to do float aritmetic
// to perform a coersion at runtime.
struct flit {
    middle: usize,
    bytes: [u8]
}

// alternatively, we could take the above representation to mean
// a ratio of two unsigned integers. while this has the advantage that
// we don't need to pick a precision cutoff, it means that runtime coersion
// requires an expensive fdiv instruction.
```
`ulit` and `flit` do not implement any `ops` arithmetic traits, since
those require `Self: Sized`. They do, however, support all the usual arithmetic
operations primitively. Since these types are *not* meant to be used at runtime
as bignums, the compiler is encouraged to implement these naively, and 
to warn when the constant time expression evaluator can't fold them away.

Again, we emphasize that the representation of these types is unspecified, and
the above is only a discussion of *possible* layouts.

Furthermore, they support the following, self-explanatory interfaces:
```rust
impl ulit {
    fn lit_bytes(&self) -> &[u8];
}

impl flit {
    fn lit_mantissa(&self) -> &[u8];
    fn lit_exponent(&self) -> &[u8];
}
```
The documentation should point out that the endianness of the returned slices
is platform-dependent. Alternatively, we could make it little-endian by default
and add some mechanism to get it in the platform endianess. We may want to
guarantee that, e.g.,
```rust
42i32 == transmute_copy::<[u8], i32>(&42.lit_bytes()[0..4])
```

`ulit` and `flit` implement `PartialEq, Eq, PartialOrd, Ord`. Note that
`flit` cannot take on the IEEE values `Infinity`, `-Infinity`, or `NaN`, so
we can *actually* get away with this.

`ulit` and `flit` are *never* infered as the value of type variables solely
on the basis that they are the type of a literal. It is unclear if we 
should allow the last case here:
```rust
fn foo<T: ?Sized>(x: &'static T) -> Box<T> { .. }

let _ = foo(&42); // T types as i32, not ulit!
let _ = foo::<ulit>(&42); // T is explcitily types as ulit. this is OK!

let _: Box<ulit> = foo(&42); // T types as ulit
```

## Weakenings

The following are ways in which we can weaken this proposal into a workable
subset:
- Arithmetic is not implemented as a polyfill, and instead collapses them 
  first. Thus,
```rust
1 + 1       // coerces to
1i32 + 1i32 // and thus types as i32, not ulit
```
- Static references are not automatically derefenced, so you'd to write
```rust
let y = x & *FOO;
```
- Either type can *only* appear as the `T` in `&'static T`, and in no
  other place. Type aliases are ok, but not associated types. I.e.,
```rust
fn foo<T: ?Sized>(x: &'static T) -> Box<T> { .. }

let _ = foo(&42); // T types as i32, not ulit!
let _ = foo::<ulit>(&42); // Error: cannot use ulit as type parameter right now

let _: Box<ulit> = foo(&42); // Error: cannot use ulit as type parameter right now
```

The compiler actually has a name for these types: `{integer}` and `{float}`,
as they appear in error messages. We may want to use these with `ulit` and
`flit`, but it is up for debate whether this will confuse beginners who
shouldn't be worrying about an advanced language feature.

# Drawbacks
[drawbacks]: #drawbacks

This adds some rather subtle rules to typeck, so we should be *very* careful
to implement this without triggering either soundness or regression.

In fact, this might trigger regression among numeric literals, a core language
feature!

The stronger versions of this proposal also introduce a confusing footgun-
these literal types are *not* meant to be used as runtime bignums, and this
may confuse users if there isn't a big warning in the documentation.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This is the best way to do this because it's the simplest. This proposal shows
what all of the knobs we could add *are*, but at the end of the day, it's a
DST with a magic coersion rule.

I don't know of any good alternatives to this that aren't implementation
details. While we can sidestep untyped `const`s with macros, we can't do
it anywhere as cleanly for `BigNum::new()`, and custom literals.

We can just... not do this, and, for custom literals, accept `u128` and
`f64`, in lieu of C++. Given that it is concievable that we will get
bigger numeric types, e.g. `u256`, which would require breaking whatever 
`ops` trait is used to implement custom integer literals.

# Prior art
[prior-art]: #prior-art

Scala’s dotty compiler has explicit literal types: the type of 1 is 1.type, 
which is a subtype of Int (corresponding to the JVM int type). In addition, 
String literals also have types: "foo".type, but this is beyond the scope of 
this proposal. These types are mostly intended to be used in generics. I don’t
know of any language that uses a single type for all int/float literals.

As pointed at the start of this RFC, many languages have untyped constants, 
but this is often  opt-out, if at all. I think my proposed opt-in mechanism 
for untyped constants is not the enormous footgun typeless-by-default is.

See below for alternatives regarding coersion.

C++ has custom literals, but custom literals are beyond the scope of this 
proposal.

# Unresolved questions
[unresolved-questions]: #unresolved-questions
The main problem is the following:
- How much should we weaken the proposal, to get a tractable subset?

We also don't know exactly in what situations a literal
type coerces to a sized type. This RFC proposes doing so when `ulit`
and `flit` appear in a `Sized` context. We could, alternatively:
- Coerce whenever they're used in a *runtime* setting
- Coerce whenever a type needs to be deduced (so that `ulit` and
  `flit` bindings must be manually typed).

Finally, some other minor considerations:
- The names of the literals. `u__` appeared in the Pre-RFC for this
  proposla, and `IntLit` has also been proposed, though this not
  agree with the naming convention for other numeric types.
- Should we consider a more granular approach, like Scala’s?
- What should `&ulit` look like through FFI?
