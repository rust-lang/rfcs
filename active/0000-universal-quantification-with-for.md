- Start Date: 2014-07-05
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)


# Summary

Whenever we want to want to introduce universal quantification explicitly, as 
opposed to just attaching a type parameter list to an existing entity, we should 
do it with the keyword `for`.


# Motivation

What we call "generic types" or "generic functions", alternately "a function 
generic over a type", is formally known as universally quantifying that type (or 
function) over another type (or types).

Universal quantification, mathematically written '∀', is pronounced as "for 
all". The keyword used by Haskell and some other languages for the same thing is 
also `forall`. `for` is very close to `forall`, and is already a keyword.

The primary existing use of the `for` keyword is in expression-level `for`..`in` 
loops. Even here, it is implicitly pronounced as "for *each* thing in
(the things)...". (`foreach` was in fact the transitional keyword at one point).
"For each" and "for all" are almost the same.

The only place in the current language where universal quantification is 
explicitly introduced is for closures with higher-rank lifetimes:

    fn call_with_arg<'a, A, B>(fun: <'x> |&'x A| -> &'x B, arg: &'a A) -> &'a B;

This sudden outburst of syntax can be difficult to parse when coming across it 
unexpectedly. It would be friendlier if one had some kind of explicit warning,
"watch out, type/lifetime parameter list coming up":

    fn call_with_arg<'a, A, B>(fun: for<'x> |&'x A| -> &'x B, arg: &'a A) -> &'a B;


# Detailed design

As noted above, the only change to the current language would be to add a `for` 
keyword to the lifetime parameter list of [higher-rank lifetimes][unboxed-closures].

If we introduce other instances of explicit universal quantification in the 
future, such as on items as proposed by [RFC PR 122][122], they should use the 
`for` keyword as well:

    // equivalent to fn print<T: Show> ...
    for<T: Show>
    fn print(val: &T);

    // scoped over multiple items
    for<T: TraitA, U: TraitB> {
        struct MyPair { a: T, b: U }

        fn make(a: (T, U)) -> MyPair;

        fn unmake(a: MyPair) -> (T, U);
    }

These can in fact be read as "for each `T` (and `U`) in (some set), define these 
items...".

[unboxed-closures]: 
https://github.com/rust-lang/rfcs/pull/114/files?short_path=157886e#closures-that-are-quantified-over-lifetimes
[122]: https://github.com/rust-lang/rfcs/pull/122


# Drawbacks

You tell me.


# Alternatives

We could just surprise people with type/lifetime parameter lists out of nowhere,
without a keyword to introduce them.

We could add a `forall` keyword. I personally think `for` is a better fit for 
Rust's character.


# Unresolved questions

None.
