- Feature Name: ffi-returns-twice
- Start Date: 2019.02.08
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC adds a new function attribute, `#[ffi_returns_twice]`, which indicates
that an foreign function can return multiple times.

# Motivation
[motivation]: #motivation

Rust assumes that function calls like:

```rust
let x = foo();
```

return only once. That is, when the execution arrives at the function call, the
function is called, it returns a single value, and that's it. This assumption
allows Rust to perform many optimizations.

However, some foreign functions like [`setjmp`] and [`vfork`] can return
multiple times.

The `#[ffi_returns_twice]` attribute specifies that a foreign function might
returns multiple times, inhibiting optimizations that assume that this is never
the case.

[`setjmp`]: https://en.cppreference.com/w/cpp/utility/program/setjmp
[`longjmp`]: https://en.cppreference.com/w/cpp/utility/program/longjmp
[`vfork`]: http://man7.org/linux/man-pages/man2/vfork.2.html

# Guide-level and reference-level explanation
[guide-level-explanation]: #guide-level-explanation

The `#[ffi_returns_twice]` function attribute specifies that a foreign function
might return multiple times, disabling optimizations that are incorrect for such
functions. Two examples of such functions are [`setjmp`] and [`vfork`].

# Drawbacks
[drawbacks]: #drawbacks

This complicates the language creating a new kind of functions.

# Prior art
[prior-art]: #prior-art

This attribute is provided in both [LLVM] and [GCC]. 

[LLVM]: https://llvm.org/docs/LangRef.html#id979
[GCC]: https://gcc.gnu.org/onlinedocs/gcc/Common-Function-Attributes.html

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

In some platforms, efficient implementations of standard library APIs have to
call low-level platform APIs that return multiple times. For example, the
widely-used [`musl`] library implements [`posix_spawn`] on Linux by calling
[`vfork`] (see: [`musl` blog post]). Rust implementations of such libraries (for
example, see: [`steed`]) should be able to call these platform interfaces
without invoking undefined behavior.

This RFC introduces a function attribute that disables incorrect optimizations
only for those functions for which they would be incorrect.

One alternative could be to remove the assumption that Rust functions return at
most once, eliminating the need for this attribute. This would have negative
performance implications for most Rust functions, which do not return multiple
times, making Rust functions a non-zero-cost abstraction.

Another alternative could be to not support interfacing with this type of
platform APIs, requiring users to use a different programming language (C,
Assembly, etc.) to do so.

[`posix_spawn`]: http://man7.org/linux/man-pages/man3/posix_spawn.3.html
[`musl`]: https://www.musl-libc.org/
[`musl` blog post]: https://ewontfix.com/7/
[`steed`]: https://github.com/japaric/steed

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* Is this attribute sufficient to make write programs with functions that return
multiple times possible?

* Should we give this attribute a different name, e.g.,
  `#[might_return_multiple_times]` or similar? Currently, this attribute is
  called `returns_twice` because that's how the C attribute is called. Using the
  same name as C here makes life easier for them.

* Should we namespace `ffi`-only attributes somehow (e.g. `#[ffi(returns_twice,
  foo, bar)]` ? See: https://github.com/rust-lang/rfcs/issues/2637

# Future possibilities
[future-possibilities]: #future-possibilities

This RFC attempts to make writing programs that interface with foreign functions
that return multiple times possible, but doing so is very challenging because it
is trivial to introduce undefined behavior in those programs.

Future RFCs could try to make it easier to avoid certain types of undefined
behavior. For example, we could extend the borrow checker to take
`#[ffi_returns_twice]` into account and reject programs that would have
undefined behavior of the type "use-after-move".

In the presence of types that implement `Drop`, usage of APIs that return
multiple times requires extreme care to avoid deallocating memory without
invoking destructors - this is critical for using [`Pin`] safely. We could also
explore diagnosing these cases.

[`Drop`]: https://doc.rust-lang.org/std/ops/trait.Drop.html
[`Pin`]: https://doc.rust-lang.org/std/pin/struct.Pin.html
