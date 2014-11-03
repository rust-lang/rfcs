- Start Date: 2014-11-02
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

This RFC proposes to rename "lifetime" to "scope". This is almost
entirely a change to documentation and error messages, but it also
affects the keyword used for associated lifetimes.

# Motivation

Borrowing, and lifetimes in particular, are one of the features of
Rust that newcomers often find most confusing and intimidating.
It doesn't help that the term "lifetime" is both unfamiliar, and does
not have exactly the desired connotations (more below).

We've been steadily working on terminology, metaphors, and intuitions,
and used these to give in-person and online tutorials on Rust; this proposal is
based on these experiences.

# Detailed design

## The problems with "lifetime"

The "lifetime" terminology suffers from a kind of one-two punch:

1. The term isn't widely used in programming, so it is unfamiliar and mysterious.

2. Your intuition or guess about the meaning of the term is likely to be wrong.

For point (1), lifetime is somewhat better than some of the existing
contenders from the academic literature: region or extent. None of
these terms are familiar to most working programmers, but lifetime is
at least a bit less generic and more lively.

For point (2), the problem is that lifetime tends to suggest a kind of
lifecycle of creation, usage, and destruction -- so at the end of a
lifetime, you'd expect the relevant object to be destroyed. But in
fact, it is exactly the opposite: as long as there is a borrow with
*any* lifetime, the borrowed object *cannot* be destroyed.

## The benefits of "scope"

The term "scope" has an immediate meaning to most programmers, and the
idea of "something happening" when a scope is exited (such as a
destructor being run) is familiar to many.

Because of its simplicity and familiarity, the term "scope" could help
the borrowing system seem more approachable to newcomers.
Moreover, upon hearing the phrase "the scope of the borrow", a
programmer is likely to have a guess about the meaning of the phrase
-- and that guess is likely to be correct.

In other words, the term overcomes both of the downsides mentioned
above with "lifetime".

Anecdotally, "scope" has proved successful in introducing borrows to
new audiences without a lot of ceremony. See Yehuda Katz's
[blog post](http://blog.skylight.io/rust-means-never-having-to-close-a-socket/)
on ownership for a great example. An excerpt explaining the "missing
lifetime specifier" error:

> Typically, Rust ties the scope of returned values to the scope of a
> borrowed argument. Here, we have no borrowed arguments, so Rust is
> asking us to be more explicit.

In reading that explanation, "scope" is evocative and leads the reader
to the right intuition with a minimum of fuss.

Scope is not without downsides of its own. In particular, as we
[move away from](https://github.com/rust-lang/rfcs/pull/396) purely
lexical scopes for determining the extent of a borrow, the term will
become somewhat less accurate.

However, when there is any chance of ambiguity, the term "borrow
scope" can be used to talk specifically about regions of code for
which a borrow may exist.

# Drawbacks

It is quite late in the game to be making this change.  However, our
experience with
[renaming `fail` to `panic`](https://github.com/rust-lang/rust/pull/17894)
suggests that it can be done. And our experience teaching Rust
suggests that it should be done.

As mentioned above, a downside of the existing meaning of "scope" is
that the term may be technically inaccurate (and grow more so over
time), but this seems unlikely to cause any real confusion, because
the starting intuition is still the right one, and "borrow scope" can
disambiguate.

# Alternatives

Another possibility would be to emphasize owning/borrowing/lending. We
could use a term drawn from contracts or leases, such as "borrow term"
or "lease length" or "borrow duration".

However, none of these terms are terribly compelling -- none of them
generate quite the same correct visceral reaction for programmers as
scope.
