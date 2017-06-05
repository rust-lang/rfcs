- Start Date: 2015-01-11
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)


# Summary

Change the syntax of explicit type instantiation from `foo::<T>(x)` to
`foo@<T>(x)`.


# Motivation

Most of the time `::` is used for module scoping, which is entirely unrelated.
(Yes, you can rationalize it by squinting your eyes and saying that selecting an
item from a module is kind-of-like instantiating a term at a type, in the way
that you can think of *anything* as being kind-of-like anything else if you
squint hard enough, but this hurts your eyes.)

The first time a person (such as myself) sees `cat::dog::<Chicken>(pig)`, at
first glance he or she is likely to try to interpret it as `<Chicken>` being
scoped under `dog`, wonder what kind of syntactic beast a `<Chicken>` is, and
get confused. (Or perhaps as something crazy involving the `<` and `>`
operators, and not enough whitespace.) This is an unforced error, and we can do
better by choosing just about anything else.

The `@` symbol has a useful mnemonic in terms of instantiating "at".


# Detailed design

Refer to summary.


# Drawbacks

> What happens during the alpha cycle?
>
> If you’re already a Rust user, the first thing you’ll notice during the alpha
> cycle is a dramatic drop in the pace of breaking changes.


# Alternatives

We could choose a different symbol, as long as it doesn't carry different
baggage.
