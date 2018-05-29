- Feature Name: unused_const_fn_result_lint
- Start Date: 2018-05-29
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add a lint for unused results of `const fn` functions,
if we know for sure that the invocation is dead code.

# Motivation
[motivation]: #motivation

[RFC 1940](https://github.com/rust-lang/rfcs/blob/master/text/1940-must-use-functions.md)
has made the `#[must_use]` attribute available on functions.
This has caused [discussion](https://github.com/rust-lang/rust/issues/48926)
about the policy on where to apply the `#[must_use]` attribute
inside the `std` library.

That discussion floated the idea that `#[must_use]` shall be
applied to every *side effect free* function, but mostly
discarded it because it would involve attaching `#[must_use]`
to so many functions.

This idea makes great sense, as in principle, if you pass data
to side effect free functions and then don't use the result,
the code is practically dead.

However, there is a better approach than to
apply `#[must_use]` everywhere:
The Rust language already has formalized side effect
freedom for functions through the `const fn` language subset.

This mechanism can be used to create a lint that,
checks for unused results of const fn invocations.

The end result is the same, but now the compiler verifies automatically
whether the function is actually side effect free,
and no manual annotations on the functions are needed beyond `const fn`.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The compiler should get a new lint called `unused_const_fn_result` which
is meant to fire if you invoke a `const fn` function but discard the result.

If you write code like:

```rust
const fn add_one(v: u32) -> u32 {
    v + 1
}
fn foo() {
    add_one(2);
}
```
The lint will fire, pointing to the invocation of `add_one`.
It shall suggest you to remove the entire function call.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `unused_const_fn_result` lint shall only consider function calls of functions
applicable for linting that:

* are `const fn`
* don't have uninhabited types as a return type (`enum Void {}`, `!`)

The first criterion gives the lint its name, and contributes a large
component in ensuring that the call actually is side effect free,
but the other criteria are important as well.

The second criterion is needed as divergence is a clear side effect.
Also, as the never type `!` affects the type of the enclosing function,
removing the invocation might might stop the code from compiling.

Of course, one might argue that functions that never return despite
having a return type distinct from an uninhabited one, e.g. by
running an infinite computation but still claiming to return `u32`,
qualify as side effect causing as well. However, finding out
whether a call terminates or not is precisely the halting problem
which is undecidable as famously shown.
From a practical standpoint, it is of low relevance whether
a function call takes 3 googol years to evaluate, or infinitely
long. Optimizers already expect that they are allowed to remove
dead code with finite nonzero execution time (and in C,
infinite as well), so relying on this for side effects
is quite brittle.

Additionally, in order for the lint to fire on a concrete function call, the call must:

* have all its generic types known... no generic types might be contributed from the calling context
* have the types of all input parameters contain no mutability (`&mut` references, `* mut` pointers, `UnsafeCell`)
* have the types of all input params which are moved have a side effect free `Drop` impl

The first two criteria are to prevent the lint from firing for function invocations which still have mutable invocations.
A simple example (inspired by [@rkruppe's example](https://github.com/rust-lang/rust/pull/50805#issuecomment-389654872)):


```rust
trait AddOne {
    fn add_one(self);
}

impl<'a> AddOne for &'a mut u32 {
    fn add_one(self) {
        *self += 1;
    }
}

const fn add_one<T: const AddOne>(v: T) {
    v.add_one();
}
```
This is of course only if the functionality actually becomes possible, like via `const Trait` ([RFC 2327](https://github.com/rust-lang/rfcs/pull/2237)).

The third criterion exists as when params are being moved, the function might invoke the param's `Drop` impl.
If this impl contains side effects, the code might not actually be not dead.
Thus, the lint should require that all moved input params either don't override the default `Drop` impl or use `const Drop` or something like it.
An initial version of the lint could simply require that overriding the `Drop`
impl is not allowed.

# Drawbacks
[drawbacks]: #drawbacks

Introduction of the lint might cause many new warnings, and break some
codebases with `#![deny(warnings)]` or similar attributes.
In the worst case, this might make people weary to add
`const fn` annotations to their functions because
of fear of breaking downstream code that has such
attributes.

However, it is already now considered bad practice to attach
`#![deny(warnings)]` to shipped code.

# Rationale and alternatives
[alternatives]: #alternatives

The alternative of manually adding `#[must_use]` to functions
already has been discussed.

We might extend the lint to functions that don't bear the `const fn` marker
but in theory *could* bear it. However, this makes it less clear to the user
why the actual invocation is dead code, and makes the compiler's descisions
harder to verify and understand. A much better idea would be to have a lint
suggesting `const fn` markers for functions that could theoretically bear them,
but currently don't. However, such a change is outside of the scope of this RFC.

# Prior art
[prior-art]: #prior-art

None known to the RFC author.

But languages that like Rust have a prominent concept of purity/side effect
freedom where you can easily discard results may have such lints.

# Unresolved questions
[unresolved]: #unresolved-questions

The lint was designed to be false positive proof in order to meet Rustc's
high bar of quality. But how false positive proof is it actually?

The policy on where to apply `#[must_use]` is left open to be discussed and
decided separately. But most likely this RFC will have an impact.

`const fn` functions might opt to conditionally unwind.
Programs might rely on that behaviour.

# Links to related discussion about the issue

* https://github.com/rust-lang/rust/issues/48926
* https://github.com/rust-lang/rust/pull/50805
