- Feature Name: `#[target_feature]` 1.1
- Start Date: 2018-04-06
- RFC PR: [rust-lang/rfcs#2396](https://github.com/rust-lang/rfcs/pull/2396)
- Rust Issue: [rust-lang/rust#69098](https://github.com/rust-lang/rust/issues/69098)

# Summary
[summary]: #summary

This RFC attempts to resolve some of the unresolved questions in [RFC 2045
(`target_feature`)]. In particular, it allows: 

* specifying `#[target_feature]` functions without making them `unsafe fn`
* calling `#[target_feature]` functions in some contexts without `unsafe { }` blocks

It achieves this by proposing three incremental steps that we can sequentially
make to improve the ergonomics and the safety of target-specific functionality
without adding run-time overhead.

[RFC 2045 (`target_feature`)]: https://github.com/rust-lang/rfcs/pull/2045

# Motivation
[motivation]: #motivation

> This is a brief recap of [RFC 2045 (`target_feature`)].

The `#[target_feature]` attribute allows Rust to generate machine code for a
function under the assumption that the hardware where the function will be
executed on supports some specific "features".

If the hardware does not support the features, the machine code was generated
under assumptions that do not hold, and the behavior of executing the function
is undefined.

[RFC 2045 (`target_feature`)] guarantees safety by requiring all
`#[target_feature]` functions to be `unsafe fn`, thus preventing them from being
called from safe code. That is, users have to open an `unsafe { }` block to call
these functions, and they have to manually ensure that their pre-conditions
hold - for example, that they will only be executed on the appropriate hardware
by doing run-time feature detection, or using conditional compilation.

And that's it. That's all [RFC 2045 (`target_feature`)] had to say about this.
Back then, there were many other problems that needed to be solved for all of
this to be minimally useful, and [RFC 2045 (`target_feature`)] dealt with those.

However, the consensus back then was that this is far from ideal for many
reasons:

* when calling `#[target_feature]` functions from other `#[target_feature]`
  functions with the same features, the calls are currently still `unsafe` but
  they are actually safe to call. 
* making all `#[target_feature]` functions `unsafe fn`s and requiring `unsafe
  {}` to call them everywhere hides other potential sources of `unsafe` within
  these functions. Users get used to upholding `#[target_feature]`-related 
  pre-conditions, and other types of pre-conditions get glossed by.
* `#[target_feature]` functions are not inlined across mismatching contexts,
  which can have disastrous performance implications. Currently calling
  `#[target_feature]` function from all contexts looks identical which makes it
  easy for users to make these mistakes (which get reported often).

The solution proposed in this RFC solves these problems.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Currently, we require that `#[target_feature]` functions be declared as `unsafe
fn`. This RFC relaxes this restriction:

* safe `#[target_feature]` functions can be called _without_ an `unsafe {}`
block _only_ from functions that have at least the exact same set of
`#[target_feature]`s. Calling them from other contexts (other functions, static
variable initializers, etc.) requires opening an `unsafe {}` even though they
are not marked as `unsafe`:

```rust
// Example 1:
#[target_feature(enable = "sse2")] unsafe fn foo() { }  // RFC2045
#[target_feature(enable = "sse2")] fn bar() { }  // NEW

// This function does not have the "sse2" target feature:
fn meow() {
    foo(); // ERROR (unsafe block required)
    unsafe { foo() }; // OK
    bar(); // ERROR (meow is not sse2)
    unsafe { bar() }; // OK
}

#[target_feature(enable = "sse2")]
fn bark() {
    foo(); // ERROR (foo is unsafe: unsafe block required)
    unsafe { foo() }; // OK
    bar(); // OK (bark is sse2 and bar is safe)
    unsafe { bar() }; // OK (as well - warning: unnecessary unsafe block)
}

#[target_feature(enable = "avx")]  // avx != sse2
fn moo() {
    foo(); // ERROR (unsafe block required)
    unsafe { foo() }; // OK
    bar(); // ERROR (moo is not sse2 but bar requires it)
    unsafe { bar() }; // OK 
}
```

> Note: while it is safe to call an SSE2 function from _some_ AVX functions,
> this would require specifying how features relate to each other in
> hierarchies. It is unclear whether those hierarchies actually exist, but
> adding them to this RFC would unnecessarily complicate it and can be done
> later or in parallel to this one, once we agree on the fundamentals.

First, this is still sound. The caller has a super-set of `#[target_features]`
of the callee. That is, the `#[target_feature]`-related pre-conditions of the
callee are uphold by the caller, therefore calling the callee is safe.

This change already solves all three issues mentioned in the motivation:

* When calling `#[target_feature]` functions from other `#[target_feature]`
  functions with the same features, we don't need `unsafe` code anymore.
* Since `#[target_feature]` functions do not need to be `unsafe` anymore,
  `#[target_feature]` functions that are marked with `unsafe` become more
  visible, making it harder for users to oversee that there are other
  pre-conditions that must be uphold.
* `#[target_feature]` function calls across mismatching contexts require
  `unsafe`, making them more visible. This makes it easier to identify
  calls-sites across which they cannot be inlined while making call-sites across
  which they can be inlined more ergonomic to write.

The `#[target_feature]` attribute continues to be allowed on inherent methods -
this RFC does not change that.

The `#[target_feature]` attribute continues to not be allowed on safe trait
method implementations because that would require an `unsafe` trait method
declaration:

```rust
// Example 2:
trait Foo { fn foo(); }
struct Fooish();
impl Foo for Fooish { 
    #[target_feature(enable = "sse2")] fn foo() { }  
    // ^ ERROR: #[target_feature] on trait method impl requires 
    // unsafe fn but Foo::foo is safe
    // (this is already an error per RFC2045)
}

trait Bar { unsafe fn bar(); }
struct Barish();
impl Bar for Barish { 
    #[target_feature(enable = "sse2")] unsafe fn bar() { }  // OK (RFC2045)
}
```

* safe `#[target_feature]` functions are not assignable to safe `fn` pointers.


```rust
// Example 3
#[target_feature(enable = "avx")] fn meow() {}

static x: fn () -> () = meow;
// ^ ERROR: meow can only be assigned to unsafe fn pointers due to 
// #[target_feature] but function pointer x with type fn()->() is safe.
static y: unsafe fn () -> () = meow as unsafe fn()->(); // OK
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This RFC proposes to changes to the language with respect to [RFC 2045 (`target_feature`)]:

* safe `#[target_feature]` functions can be called _without_ an `unsafe {}`
block _only_ from functions that have at least the exact same set of
`#[target_feature]`s. Calling them from other contexts (other functions, static
variable initializers, etc.) requires opening an `unsafe {}` even though they
are not marked as `unsafe`

* safe `#[target_feature]` functions are not assignable to safe `fn` pointers.

# Drawbacks
[drawbacks]: #drawbacks

This RFC extends the typing rules for `#[target_feature]`, which might
unnecessarily complicate future language features like an effect system.

# Rationale and alternatives
[alternatives]: #alternatives

Since `#[target_feature]` are effects or restrictions (depending on whether we
`enable` or `disable` them), the alternative would be to integrate them with an
effect system. 

# Prior art
[prior-art]: #prior-art

[RFC2212 target feature unsafe](https://github.com/rust-lang/rfcs/pull/2212)
attempted to solve this problem. This RFC builds on the discussion that was
produced by that RFC and by many discussions in the `stdsimd` repo.

# Unresolved questions
[unresolved]: #unresolved-questions

None.

# Future possibilities
[future-possibilities]: #future-possibilities

## Negative features

[RFC 2045 (`target_feature`)] introduced the `#[target_feature(enable = "x")]`
syntax to allow introducing negative features in future RFCs in the form of
`#[target_feature(disable = "y")]`. Since these have not been introduced yet we
can only speculate about how they would interact with the extensions proposed in
this RFC but we probably can make the following work in some form:

```rust
// #[target_feature(enable = "sse")]
fn foo() {}

#[target_feature(disable = "sse")] 
fn bar() {
    foo(); // ERROR: (bar is not sse)
    unsafe { foo() }; // OK
}

fn baz() {
  bar(); // OK 
}
```

## Effect system

It is unclear how `#[target_feature]` would interact with an effect system for
Rust like the one being tracked
[here](https://github.com/Centril/rfc-effects/issues) and discussed in
[RFC2237](https://github.com/rust-lang/rfcs/pull/2237).

In particular, it is unclear how the typing rules being proposed here would be
covered by such an effect system, and whether such system would support
attributes in effect/restriction position. 

Such an effect-system might need to introduce first-class target-features into
the language (beyond just a simple attribute) which could lead to the
deprecation of the `#[target_feature]` attribute.

It is also unclear how any of this interacts with effect-polymorphism at this
point, but we could _maybe_ support something like `impl const Trait` and `T:
const Trait`:

```rust
impl #[target_feature(enable = "...")] Trait for Type { ... }
fn foo<T: #[target_feature(enable = "...")] Trait>(...) { ...}
```

if all trait methods are `unsafe`; otherwise they can't have the
`#[target_feature]` attribute.

