- Feature Name: `variadic_generics`
- Start Date: 2017-2-22
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC proposes the addition of several features to support variadic generics:
- An intrinsic `Tuple` trait implemented exclusively by tuples
- `(Head, ...Tail)` syntax for tuple types where `Tail: Tuple`
- `let (head, ...tail) = tuple;` pattern-matching syntax for tuples
- `let tuple = (head, ...tail);` syntax for joining an element with a tuple

# Motivation
[motivation]: #motivation

Variadic generics are a powerful and useful tool commonly requested by Rust
users (see
[#376](https://github.com/rust-lang/rfcs/issues/376) and
[#1921](https://github.com/rust-lang/rfcs/pull/1921)). They allow
programmers to abstract over non-homogeneous collections, and they make it
possible to implement functions which accept arguments of varying length and
type.

Rust has a demonstrable need for variadic generics.

In Rust's own standard library, there are a number of traits which have
been repeatedly implemented for tuples of varying size up to length 12 using
macros. This approach has several downsides:
- It presents arbitrary restrictions on tuples of size 13+.
- It increases the size of the generated code, resulting in slow compile times.
- It complicates documentation
(see the list of trait implementations in
[this documentation](https://doc.rust-lang.org/std/primitive.tuple.html)).

These arbitrary tuple-length restrictions, manual tuple macros, and confusing
documentation all combine to increase Rust's learning curve.

Furthermore, community library authors are required to implement similar
macro-based approaches in order to implement traits for tuples. In the `Diesel`
crate, it was discovered that replacing macro-generated tuple implementations
with a structurally-recursive implementation (such as the one proposed here)
resulted in a 50% decrease in the amount of code generated and a 70% decrease
in compile times ([link](https://github.com/diesel-rs/diesel/pull/747)). This
demonstrates that Rust's lack of variadic generics is resulting in a subpar
edit-compile-debug cycle for at least one prominent, high-quality crate.

The solution proposed here would resolve the limitations above by making it
possible to implement traits for tuples of arbitrary length. This change would
make Rust libraries easier to understand and improve the edit-compile-debug
cycle when using variadic code.


# Detailed design
[design]: #detailed-design

## The `Tuple` Trait
The following would be implemented by all tuple types:
```rust
trait Tuple {
    type AsRefs<'a>: Tuple + 'a;
    type AsMuts<'a>: Tuple + 'a;
    fn elements_as_refs<'a>(&'a self) -> Self::AsRefs<'a>;
    fn elements_as_mut<'a>(&'a mut self) -> Self::AsMuts<'a>;
}
```

TODO: should the above use `TupleRef<'a>` and `TupleMut<'b>` traits to avoid
dependency on ATCs? It seems nicer to have them all together in one trait, but
it's probably not worth the resulting feature-stacking mess.

The types `AsRefs` and `AsMuts` are the corresponding tuples of references to
each element in the original tuple. For example,
`(A, B, C)::AsRefs = (&A, &B, &C)` and
`(A, B, C)::AsMuts = (&mut A, &mut B, &mut C)`

The `Tuple` trait should only be implemented for tuples and marked with the
`#[fundamental]` attribute described in
[the coherence RFC](https://github.com/rust-lang/rfcs/blob/master/text/1023-rebalancing-coherence.md).
This would allow coherence and type-checking to be extended to assume that no
implementations of `Tuple` will be added. This enables an increased level of
negative reasoning making it easier to write blanket implementations of traits
for tuples.

## The `(Head, ...Tail)` Type Syntax
This syntax would allow for a `Cons`-cell-like representation of tuple types.
For example, `(A, ...(B, C))` would be equivalent to `(A, B, C)`. This allows
users to represent the type of tuples in an inductive style when writing trait
implementations.

## The `(head, ...tail)` Pattern-Matching Syntax
This syntax allows for splitting apart the head and tail of a tuple. For
example, `let (head, ...tail) = (1, 2, 3);` moves the head value, `1`, into
`head`, and the tail value, `(2, 3)`, into `tail`.

## The `(head, ...tail)` Joining Syntax
This syntax allows pushing an element onto a tuple. It is the natural inverse
of the pattern-matching operation above. For example,
`let tuple = (1, ...(2, 3));` would result in `tuple` having a value of
`(1, 2, 3)`.

## An Example

Using the tools defined above, it is possible to implement `TupleMap`, a
trait which can apply a mapping function over all elements of a tuple:

```rust
trait TupleMap<F>: Tuple {
    type Out: Tuple;
    fn map(self, f: F) -> Self::Out;
}

impl<F> TupleMap<F> for () {
    type Out = ();
    fn map(self, _: F) {}
}

impl<Head, Tail, F, R> TupleMap<F> for (Head, ...Tail)
    where
    F: Fn(Head) -> R,
    Tail: TupleMap<F>,
{
    type Out = (R, ...<Tail as TupleMap<F>>::Out);
    
    fn map(self, f: F) -> Self::Out {
        let (head, ...tail) = self;
        let mapped_head = f(head);
        let mapped_tail = tail.map(f);
        (mapped_head, mapped_tail...)
    }
}
```

This example is derived from
[a playground example by @eddyb]()
that provided inspiration for this RFC.

The example demonstrates the concise, expressive code enabled
by this RFC. In order to implement a trait for tuples of any length, all
that was necessary was to implement the trait for `()` and `(Head, ...Tail)`.

# How We Teach This
[teach]: #teach

The `(head, ...tail)` and `(Head, ...Tail)` syntax closely mirror established
patterns for working with `Cons`-cell based lists. Rustaceans coming from
other functional programming languages will likely be familiar with the concept
of recursively-defined lists. For those unfamiliar with `Cons`-based
lists, the concept should be introduced using "structural recursion": there's
a base case, `()`, and a recursive/inductive case: `(Head, ...Tail)`. Any tuple
can be thought of in this way
(for example, `(A, B, C)` is equivalent to `(A, ...(B, ...(C, ...())))`).

The exact mechanisms used to teach this should be determined after getting more
experience with how Rustaceans learn. After all, Rust users are a diverse crowd,
so the "best" way to teach one person might not work as well for another. There
will need to be some investigation into which explanations are more
suitable to a general audience.

As for the `(head, ...tail)` joining syntax, this should be explained as
taking each part of the tail (e.g. `(2, 3, 4)`) and inlining or un-"tupling"
them (e.g. `2, 3, 4`). This is nicely symmetrical with the `(head, ...tail)`
pattern-matching syntax.

The `Tuple` trait is a bit of an oddity. It is probably best not to go too
far into the weeds when explaining it to new users. The extra coherence
benefits will likely go unnoticed by new users, as they allow for more
advanced features and wouldn't result in an error where one didn't exist
before. The obvious exception is when trying to implement the `Tuple` trait.
Attempts to implement `Tuple` should resort in a relevant error message,
such as "The `Tuple` trait cannot be implemented for custom types."

# Drawbacks
[drawbacks]: #drawbacks

As with any additions to the language, this RFC would increase the number
of features present in Rust, potentially resulting increased complexity
of the language.

There is also some unfortunate overlap between the proposed `(head, ...tail)`
syntax and the current inclusive range syntax. However, the similarity
between `start...end` and `...tail` can be disambiguiated by whether or not
there is an expression immediately before the ellipsis.

# Alternatives
[alternatives]: #alternatives

- Do nothing.
- Implement one of the other variadic designs, such as
[#1582](https://github.com/rust-lang/rfcs/pull/1582) or
[#1921](https://github.com/rust-lang/rfcs/pull/1921)

# Unresolved questions
[unresolved]: #unresolved-questions
It might be useful in the future to expand on the locations where `...Type`
can be used. Potential extensions to this RFC could allow `...Type` in
non-tuple generics or in function argument types, like
`fn foo<Args>(args: ...Args)`.
This would allow functions and traits to use variadic generics without
explicit tuples. This could enable things like the proposed `foo[i, j]` syntax
using`Index<usize, usize>`.
