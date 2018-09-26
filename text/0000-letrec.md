- Feature Name: letrec
- Start Date: 2018-09-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

A new syntax keyword `letrec`, for recursive local binding.

# Motivation
[motivation]: #motivation

By using the `letrec` keyword, we can make local recursive closures.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `letrec` keyword is similar to `let`,
it should only be used when its right hand side is a closure,
it is different from `let` in that,
the binding introduced by `letrec`
is effective in the body of the closure at the right hand side.

```rust
fn sum (
    term: &impl Fn (f64) -> f64, a: f64,
    next: &impl Fn (f64) -> f64, b: f64,
) -> f64 {
    letrec sum_iter = |a, result| {
        if a > b {
            result
        } else {
            sum_iter (next (a), term (a) + result)
        }
    };
    sum_iter (a, 0.0)
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The semantics and implementation of the `letrec` keyword
is well understood in functional languages such as Scheme and Ocaml.

# Drawbacks
[drawbacks]: #drawbacks

No known drawbacks.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Without recursive local binding,
the example above must be written as the folllowing:

(in which, the recursive local binding
must be defined as a global auxiliary function
with extra arguments.)

```rust
fn sum_iter (
    term: &impl Fn (f64) -> f64, a: f64,
    next: &impl Fn (f64) -> f64, b: f64,
    result: f64,
) -> f64 {
    if a > b {
        result
    } else {
        sum_iter (
            term, next (a),
            next, b,
            term (a) + result)
    }
}

fn sum (
    term: &impl Fn (f64) -> f64, a: f64,
    next: &impl Fn (f64) -> f64, b: f64,
) -> f64 {
    sum_iter (term, a, next, b, 0.0)
}
```

There are known workaround when the language itself does not support recursive local binding.
For example,
in https://stackoverflow.com/questions/16946888/is-it-possible-to-make-a-recursive-closure-in-rust
auxiliary local struct is used.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Also semantics and implementation of `letrec` is well known,
but it still a state of art syntax keyword.

It is still challenging to implement it well,
specially when adding it to an existing language implementation.
