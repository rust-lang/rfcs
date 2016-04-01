- Feature Name: function-clauses
- Start Date: 2016-04-01
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Add function overloading by using pattern matching, like Erlang and Elixir.
Specifically, a function can have multiple function clauses with each clause
having the same type signature but different refutable patterns, with the
whole set being irretuable.

# Motivation
[motivation]: #motivation

This allows people who write functions that would have a `match` as the top
level expression instead just write multiple functions, and have one less
level of nesting.

It can also help separate out base cases of recursive functions.

# Detailed design
[design]: #detailed-design

I think it's always good to start with an example.

## Example

```rust
fn before(optional_foo: Option<Foo>) {
    match optional_foo {
        Some(foo) -> foo.consume();
        None -> {}
    };
}

fn after(Some(foo): Option<Foo>) { foo.consume(); }
fn after(None: Option<Foo>) {}
```

## Description

Function parameters take refutable patterns instead of irrefutable ones,
and multiple `fn`s can be written with the same name as long as the type
signature is the same. Each `fn` with the same name is a "function clause" of
the overall function. The types for each pattern must be written in each clause.

Function clauses are ordered by declaration. The first function clause that
matches the data is the one that is executed. The function clauses must be
exhaustive — like `match` expressions are exhaustive.

Every function clause must be together. It is an error to put another item
between two function clauses.

The function clauses together make up one item, and as such, any attributes that
act on the entire function must be before the first function clause. Any
attributes that act on the function clause (e.g. lints) must be inside the
function clause.

# Drawbacks
[drawbacks]: #drawbacks

Ultimately, this increases the complexity of the language for syntactic sugar.

It would also increase the number of possible errors. Instead of having multiple
functions with the same name always be an error, now there's two possible
errors. And there's another error with putting attributes in the middle of a set
of functions.

This complects `match` and `fn` in such a way that any change to `match` will
probably also require a change to `fn`.

# Alternatives
[alternatives]: #alternatives

Requiring that attributes be inside the function clauses maintains consistency
because anything on the first function clause is applied to the whole function,
so it would trip people up that attributes on other clauses apply only to that
specific clause. If we're okay with that possible confusion, we could allow
them on the outside of function clauses.

There are quite a lot of other ways to overload functions. This way probably
stops other ways over overloading functions, though it doesn't actually stop
overloading by arity or overloading by different types in a technical sense.
But having multiple ways to overload a function syntactically is likely to add
to the learning curve.

There are other syntactic ways of having function clauses, but they have the
same ergonomics of using a `match` statement at the top of a function without
overloading. One ergonomic benefit would be only having to declare the type
once, not for every clause.

If this is not done, then declaring the same function multiple times remains
impossible, and a different way of overloading functions is possible.

# Unresolved questions
[unresolved]: #unresolved-questions

How many functions would this actually benefit?

Would this help ergonomics of when people do want to have a function that can
take multiple types, but due to lack of overloading on type, uses an enum that
wraps the types?

If it does help ergonomics there, would it just be better to overload on the
type?

Do we use E0004 for non-exhaustive patterns in functions, as it is used for
`match`, or do we want something more specialized?