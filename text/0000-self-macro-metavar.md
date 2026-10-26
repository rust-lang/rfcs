- Feature Name: `self_macro_metavar`
- Start Date: 2020-08-03
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Introduce the `$self` macro metavariable, a companion to `$crate`, that allows
macros hygienic access to items.

# Motivation
[motivation]: #motivation

It is presently impossible to define macros with identifiers that resolve at the
macro's definition site upon expansion. This shortcoming is well-acknowledged
and well-known, and, while [declarative macros 2.0] aimed to resolve this issue,
its implementation and subsequent stabilization sit in limbo.

As an example of a macro that's presently impossible to write, consider the
following, where `PRIVATE` is expected to resolve to `submod::PRIVATE`
regardless of where `m` is expanded:

```rust
mod submod {
    static PRIVATE: &'static str = "PRIVATE_SUBMOD";

    #[macro_export]
    macro_rules! m {
        () => (println!("{}", PRIVATE))
    }

    pub use m;
}

pub fn main() {
    submod::m!(); // error[E0425]: cannot find value `PRIVATE` in this scope
}
```

As illustrated, the call to the `m!()` errors as "`PRIVATE` is not in-scope".
Specifically, the call to `m!()` expands to `println!("{}, PRIVATE);`, where
`PRIVATE` resolves as if it were an `item` identifier. This implies that the
following _does_ compile, printing `Hi!` when run, perhaps unexpectedly:

```rust
fn main() {
    submod::m!();
    static PRIVATE: &'static str = "Hi!";
}
```

Today, no combination of `macro_rules!()` or `proc_macro` invocations embedded
within allows for declaring an `m` that expands such that `PRIVATE` in the
expansion resolves to `submod::PRIVATE`. Even the following example, which
mimics what is possible with identifiers today, fails:

```rust
mod submod {
    static PRIVATE: &'static str = "PRIVATE_SUBMOD";

    macro_rules! make_local {
        ($local:expr) => (
            #[macro_export]
            macro_rules! m {
                () => (println!("{}", $local))
            }

            pub use m;
        )
    }

    make_local!(PRIVATE);
}

pub fn main() {
    submod::m!(); // error[E0425]: cannot find value `PRIVATE` in this scope
}
```

`$self` resolves this deficiency. With `$self`, `m` could be declared as:

```rust
mod submod {
    static PRIVATE: &'static str = "PRIVATE_SUBMOD";

    #[macro_export]
    macro_rules! m {
        () => (println!("{}", $self::PRIVATE))
    }

    pub use m;
}

pub fn main() {
    submod::m!(); // `PRIVATE` unconditionally resolves to `submod::PRIVATE`
}
```

On expansion of `m`, `PRIVATE` unambiguously and unconditionally resolves as if
it were at the definition site, that is, to `submod::PRIVATE`.

[declarative macros 2.0]: https://github.com/rust-lang/rust/issues/39412 

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `$self` macro metavariable, like the `$crate` metavariable, can be used to
modify the hygeine of identifiers in a macro. `$self` works a lot like the
`self` in module paths: when used at the start of a path in a macro, the
succeeding path will be resolved as if it were in the module where the macro is
defined, regardless of where the macro is expanded. Different from `self` in
module paths, however, `$self` _also_ captures the visibility of the module path
at the definition site: the succeeding path will be visible in the expansion if
it is visible at the macro's definition site.

Said differently, `$self` _captures_ the module scope at the macro definition
site and applies it to the succeeding path upon expansion. As an example,
consider the definition of the macro `submod::m!`:

```rust
mod submod {
    static PRIVATE: &'static str = "PRIVATE_SUBMOD";

    #[macro_export]
    macro_rules! m {
        () => (println!("{}", $self::PRIVATE))
    }
}

pub fn main() {
    submod::m!(); // `PRIVATE` unconditionally resolves to `submod::PRIVATE`
}
```

Without `$self`, it would not be possible to reference `submod::PRIVATE` outside
of `submod`. Observe, too, that unlike `$crate`, `$self` _does_ have an effect
on visibility: while `submod::PRIVATE` in `main` would _not_ resolve, the
expansion including `$self::PRIVATE` does!

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

At its core, `$self` is `$crate` at the module-level as opposed to the crate
level. Macro metavariable naming collisions are handled in the same way as with
`$crate`. In particular, a declaration of `$self` in a macro shadows the `$self`
described here. The following works as expected, and importantly, as it does
today:

```rust
macro_rules! m {
    ($self:ident) => (println!("{}", $self))
}
```

Additionally, like `$crate`, a non-user-declared `$self` _must_ be followed by
`::`.

Notably different is that while `$crate` can be implemented as a purely
syntactic transformation, substituting `$crate` for the name of the crate in
which the macro is defined, `$self` must apply the full resolution context of
the macro's definition site to the succeeding path. When calling a macro using
`$self` cross-crate, this requires cross-crate hygiene. Thankfully, this was
recently added to the compiler in https://github.com/rust-lang/rust/pull/72121.

Thus, `$self` can be simply and without further caveats by specified as: for
every path in the expansion that begins with `$self`, the resolution context of
the path is set to resolution context of the `Span::source()` of `$self`.

In addition to the examples in the introductory text, consider the following:

```rust
mod a {
    static PRIVATE: &'static str = "B";

    #[macro_export]
    macro_rules! m1 {
        ($($var:tt)*) => (println!("{}, {}", $self::PRIVATE, $($var)*))
    }
}

mod b {
    static PRIVATE: &'static str = "A";

    #[macro_export]
    macro_rules! m2 {
        () => (m1!($self::PRIVATE))
    }
}

pub fn main() {
    m2!();
}
```

The resulting program prints `B, A`.

# Drawbacks
[drawbacks]: #drawbacks

As always, introducing new language-level features can add the cognitive
overhead. However, `$self`'s similarity to `$crate` means that it doesn't
introduce an entirely new concept. What's more, it is orthogonal to all existing
language features, which means users find one solution to the problem it
resolves.

`$self` as described here is backwards-compatible: there are no compatibility
hazards.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

1. Wait for Macros 2.0

   Self-explanatory. Unfortunately, the implementation and stabilization of
   macros 2.0 is in limbo.

2. Propagate Resolution Context for Items, Too

   The second `submod` example in the introductory text could be made to work.
   Unfortunately, this has the major drawback that it breaks existing code. That
   is, it is not backwards-compatible. Furthermore, it requires two expansions
   to achieve the same net-effect that this proposal allows in one.

3. Use some other syntax, like `#PRIVATE`, to capture hygiene

   Instead of `$self::PRIVATE`, `#PRIVATE` could yield the same effect. This
   introduces brand new syntax with no existing analogy, however, and so would
   be harder to teach.

# Prior art
[prior-art]: #prior-art

I am not aware of an existing `$self`-like mechanism in other languages. Rust's
own `$crate` is the inspiration for this feature. Other issues, notably going
back to https://github.com/rust-lang/rust/issues/22462, have also considered the
deficiency resolved by this proposal.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None.

# Future possibilities
[future-possibilities]: #future-possibilities

None. Macros 2.0 continues to be the eventual goal.
