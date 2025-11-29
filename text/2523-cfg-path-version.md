- Feature Name: `cfg_version` and `cfg_accessible`
- Start Date: 2018-08-12
- RFC PR: [rust-lang/rfcs#2523](https://github.com/rust-lang/rfcs/pull/2523)
- Rust Issue: [rust-lang/rust#64796](https://github.com/rust-lang/rust/issues/64796) and [rust-lang/rust#64797](https://github.com/rust-lang/rust/issues/64797)

## Summary
[summary]: #summary

Permit users to `#[cfg(..)]` on whether:

+ they have a certain minimum Rust version (`#[cfg(version(1.27.0))]`).
+ a certain external path is accessible
  (`#[cfg(accessible(::std::mem::ManuallyDrop))]`).

## Motivation
[motivation]: #motivation

[stability_stagnation]: https://blog.rust-lang.org/2014/10/30/Stability.html
[what_is_rust2018]: https://blog.rust-lang.org/2018/07/27/what-is-rust-2018.html

A core tenet of Rust's story is
[*"stability without stagnation"*][stability_stagnation].
We have made great strides sticking to this story while continuously
improving the language and the community. This is especially the case with
the coming [Rust 2018 edition][what_is_rust2018].

However, while the situation for evolving the language is doing well,
the situation for library authors is not as good as it could be.
Today, crate authors often face a dilemma: - *"Shall I provide more features
and implementations for later versions of Rust, or should I stay compatible
with more versions of the compiler"*.

[cargo_version_selection]: http://aturon.github.io/2018/07/25/cargo-version-selection/

While [much thought][cargo_version_selection] has been given to how we can
reduce "dependency hell" by enhancing cargo for:

+ the **control** users have over their dependencies.
+ the **compatibility** of crates with each other.
+ reducing the **maintainability** burden of having to make sure that
  versions work with each other.

[RFC 2483]: https://github.com/rust-lang/rfcs/pull/2483

...not much focus has been given to how conditional compilation can be improved
to extend how many versions back a crate supports. This becomes critically
important if and when we gain LTS channels as proposed by [RFC 2483].

[version_check]: https://crates.io/crates/version_check

The current support for such conditional compilation is lacking.
While [it is possible][version_check] to check if you are above a certain
compiler version, such facilities are not particularly ergonomic at the moment.
In particular, they require the setting up of a `build.rs` file and
declaring up-front which versions you are interested in knowing about.
These tools are also unable to check, without performing canary builds
of simple programs with `use ::std::some::path;`, if a certain path exists
and instead force you to know which version they were introduced in.

*We can do better.* In this RFC we aim to rectify this by giving library
authors the tools they need in the language itself. With the features
proposed in the [summary] we aim to make retaining *compatibility* and
supporting more compiler versions *pain-free* and to give authors a lot
of *control* over what is supported and what is not.

[rust-lang-nursery/error-chain#101]: https://github.com/rust-lang-nursery/error-chain/issues/101

Another use case this RFC supports is to work around compiler bugs by
checking if we are on a particular version. An example where this occurred
is documented in [rust-lang-nursery/error-chain#101].

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

### `#[cfg(accessible($path))]`

Consider for a moment that we would like to use the `Iterator::flatten` method
of the standard library if it exists (because it has become soon in a certain 
Rust version), but otherwise fall back to `Itertools::flatten`.
We can do that with the following snippet:

```rust
#[cfg(accessible(::std::iter::Flatten))]
fn make_iter(limit: u8) -> impl Iterator<Item = u8> {
    (0..limit).map(move |x| (x..limit)).flatten()
}

#[cfg(not(accessible(::std::iter::Flatten)))]
fn make_iter(limit: u8) -> impl Iterator<Item = u8> {
    use itertools::Itertools;
    (0..limit).map(move |x| (x..limit)).flatten()
}

// Even better
fn make_iter(limit: u8) -> impl Iterator<Item = u8> {
    #[cfg(not(accessible(::std::iter::Flatten)))]
    use itertools::Itertools;
    (0..limit).map(move |x| (x..limit)).flatten()
}

fn main() {
    println!("{:?}", make_iter(10).collect::<Vec<_>>());
}
```

What this snippet does is the following:

1. If the path `::std::iter::Flatten` exists, the compiler will compile
   the first version of `make_iter`. If the path does not exist,
   the compiler will instead compile the second version of `make_iter`.

The result of 1. is that your crate will use `Iterator::flatten` on newer
versions of Rust and `Itertools::flatten` on older compilers.
The result of this is that as a crate author, you don't have to publish any
new versions of your crate for the compiler to switch to the libstd version
when people use a newer version of Rust.

[`proptest`]: https://github.com/altsysrq/proptest
[adding support]: https://github.com/AltSysrq/proptest/blob/67945c89e09f8223ae945cc8da029181822ce27e/src/num.rs#L66-L76

Once the standard library has stabilized `iter::Flatten`,
future stable compilers will start using the first version of the function.

In this case we used the `accessible` flag to handle a problem that the addition
of `Iterator::flatten` caused for us if we had used `Itertools::flatten`.
We can also use these mechanisms for strictly additive cases as well.
Consider for example the [`proptest`] crate [adding support] for `RangeInclusive`:

```rust
// #[cfg_attr(feature = "unstable", feature(inclusive_range))]
// ^-- If you include this line; then `cargo build --features unstable`
//     would cause nightly compilers to activate the feature gate.
//     Note that this has some inherent risks similar to those for
//     `#[cfg(nightly)]` (as discussed later in this RFC).

macro_rules! numeric_api {
    ($typ:ident) => {
        ...

        #[cfg(accessible(::core::ops::RangeInclusive))]
        impl Strategy for ::core::ops::RangeInclusive<$typ> {
            type Tree = BinarySearch;
            type Value = $typ;

            fn new_tree(&self, runner: &mut TestRunner) -> NewTree<Self> {
                Ok(BinarySearch::new_clamped(
                    *self.start(),
                    $crate::num::sample_uniform_incl(runner, *self.start(), *self.end()),
                    *self.end()))
            }
        }

        ...
    }
}

macro_rules! unsigned_integer_bin_search {
    ($typ:ident) => {
        pub mod $typ {
            use rand::Rng;

            use strategy::*;
            use test_runner::TestRunner;

            int_any!($typ);
        }
    }
}

unsigned_integer_bin_search!(u8);
unsigned_integer_bin_search!(u16);
...
```

This means that `proptest` can continue to evolve and add support for
`RangeInclusive` from the standard library and the `x..=y` syntax in the
language without having to release a new breaking change version.
Dependents of `proptest` simply need to be on a compiler version where
`::core::ops::RangeInclusive` is defined to take advantage of this.

So far we have only used `accessible(..)` to refer to paths in the standard 
library. However, while it will be a less likely use case, you can use the flag
to test if a path exists in some library in the ecosystem. This can for example
be useful if you need to support lower minor versions of a library but also
add support for features in a higher minor version.

### `#[cfg(version(1.27.0))]`

Until now, we have only improved our support for library features but never
any language features. By checking if we are on a certain minimum version of
Rust or any version above it, we can conditionally support such new features.
For example:

```rust
#[cfg_attr(version(1.27), must_use)]
fn double(x: i32) -> i32 {
    2 * x
}

fn main() {
    double(4);
    // warning: unused return value of `double` which must be used
    // ^--- This warning only happens if we are on Rust >= 1.27.
}
```

Another example is opting into the system allocator on Rust 1.28 and beyond:

```rust
#[cfg(version(1.28))]
// or: #[cfg(accessible(::std::alloc::System))]
use std::alloc::System;

#[cfg_attr(version(1.28), global_allocator)]
static GLOBAL: System = System;

fn main() {
    let mut v = Vec::new();
    // This will allocate memory using the system allocator.
    // ^--- But only on Rust 1.28 and beyond!
    v.push(1);
}
```

Note that you won't be able to make use of `#[cfg(version(..))]` for these 
particular features since they were introduced before this RFC's features
get stabilized. This means that you can't for example add `version(1.28)`
to your code and expect Rust 1.28 compilers to enable the code.
However, there will be features in the future to use this mechanism on.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

### `#[cfg(version(<semver>))]`

To the `cfg` attribute, a `version` flag is added.
This flag has the following grammar (where `\d` is any digit in `0` to `9`):

```rust
flag : "version" "(" semver ")" ;
semver : digits ("." digits ("." digits)?)? ;
digits : \d+ ;
```

[caret requirements]: https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#caret-requirements

If and only if a Rust compiler considers itself to have a version which is
greater or equal to the version in the `semver` string will the
`#[cfg(version(<semver>)]` flag be considered active.
Greater or equal is defined in terms of [caret requirements].

### `#[cfg(accessible($path))]`

To the `cfg` attribute, an `accessible` flag is added.

#### Syntactic form

This flag requires that a `path` fragment be specified in it inside parenthesis
but not inside a string literal. The `$path` must start with leading `::`
and may not refer to any parts of the own crate (e.g. with `::crate::foo`,
`::self::foo`, or `::super::foo` if such paths are legal).
This restriction exists to ensure that the user does not try to
conditionally compile against parts of their own crate because that crate
has not been compiled when the `accessible` flag is checked on an item.

#### Basic semantics

If and only if the path referred to by `$path` does exist and is public
will the `#[cfg(accessible($path))]` flag be considered active.

#### `#![feature(..)]` gating 

In checking whether the path exists or not, the compiler will consider
feature gated items to exist if the gate has been enabled.

**NOTE:** In the section on `#[cfg(nightly)]` and in the
[guide level explanation][guide-level-explanation] we note that there are
some risks when combining `cfg(feature = "unstable")` and `accessible(..)` to
add conditional support for an unstable feature that is expected to stabilize.
With respect to such usage:

1. User-facing documentation, regarding `accessible(..)` should highlight risky
   scenarios, including with examples, with respect to possible breakage.

2. Our stability policy is updated to state that breakage caused due to misuse
   of `accessible(..)` is _allowed_ breakage. Consequently, rust teams will not
   delay releases or un-stabilize features because they broke a crate using
   `accessible(..)` to gate on those features.

#### Inherent implementations

If a path refers to an item inside an inherent implementation,
the path will be considered to exist if any configuration of generic
parameters can lead to the item. To check whether an item exists for
an implementation with a specific sequence of concrete types applied to
a type constructor, it is possible to use the `::foo::bar::<T>::item` syntax.

#### Fields

It is also possible to refer to fields of `struct`s, `enum`s, and `unions`.
Assuming that we have the following definitions in the `foobar` crate:

```rust
pub struct Person { pub ssn: SSN, age: u16 }

pub enum Shape<Unit> {
    Triangle { pub sides: [Unit; 3] },
    ...
}

pub union MaybeUninit<T> { uninit: (), pub value: T }
```

We can then refer to them like so:

```rust
#[cfg(all(
    accessible(::foobar::Person::ssn),
    accessible(::foobar::Shape::Triangle::sides),
    accessible(::foobar::Shape::MaybeUninit::value)
))]
fn do_stuff() {
    ...
}
```

#### Macros

Finally, bang macros, derive macros, attributes of all sorts including
built-in, user provided, as well as latent derive helper attributes,
will be considered when determining if a path is accessible.

### `cfg_attr` and `cfg!`

Note that the above sections also apply to the attribute `#[cfg_attr(..)]` as
well as the special macro `cfg!(..)` in that `version(..)` and `accessible(..)`
are added to those as well.

## Drawbacks
[drawbacks]: #drawbacks

One argument is that hypothetically, if the standard library removed
some unstable item, then we might "not notice" if everyone uses it through
`#[cfg(accessible(..))]`.

### Incremental garbage code and its collection

It sometimes happens that feature gates never make it to stable and
that they instead get scrapped. This occurs infrequently.
However, when this does happen, code that is conditionally compiled under
`#[cfg(accessible(::std::the::obsoleted::path))]` will become garbage that
just sits around. Over time, this garbage can grow to a non-trivial amount.

However, if we provide LTS channels in the style of [RFC 2483],
then there are opportunities to perform some "garbage collection"
of definitions that won't be used when the LTS version changes.

## Rationale and alternatives
[alternatives]: #rationale-and-alternatives

### `accessible(..)`

The primary rationale for the `accessible` mechanism is that when you
want to support some library feature, it is some path you are thinking of
rather than what version it was added. For example, if you want to use
`ManuallyDrop`, you can just ask if it exists. The `version` is instead a
proxy for the feature. Instead of detecting if the path we want is available
or not via an indirection, we can just check if the path exists directly.
This way, a user does not have to look up the minimum version number for
the feature.

You may think that `version(..)` subsumes `accessible(..)`.
However, we argue that it does not. This is the case because at the time of
enabling the `feature = "unstable"` feature that enables the path in libstd,
we do not yet know what minimum version it will be supported under.
If we try to support it with `version(..)`, it is possible that we may
need to update the minimum version some small number of times.
However, doing so even once means that you will need to release new versions
of your crate. If you instead use `accessible(..)` you won't need to use
it even once unless the name of the path changes in-between.

Another use case `accessible(..)` supports that `version(..)` doesn't is checking
support for atomic types, e.g. `accessible(::std::sync::atomic::AtomicU8)`.
This subsumes the proposed `#[cfg(target_has_atomic = "..")]` construct.

#### Preventing relative paths

The reason why we have enforced that all paths must start with `::` inside
`accessible(..)` is that if we allow relative paths, and users write
`accessible(self::foo)`, then they can construct situations such as:

```rust
#[cfg(accessible(self::bar)]
fn foo() {}

#[cfg(accessible(self::foo)]
fn bar() {}
```

One way around this is to collect all items before `cfg`-stripping,
but this can cause problems with respect to stage separation.
Therefore, we prevent this from occurring with a simple syntactic check.

One mechanism we could use to make relative paths work is to use a different
resolution algorithm for `accessible(..)` than for `use`. We would first
syntactically reject `self::$path`, `super::$path`, and `crate::$path`.
The resolution algorithm would then need to deal with situations such as:

```rust
#[cfg(accessible(bar)]
fn foo() {}

#[cfg(accessible(foo)]
fn bar() {}
```

by simply not considering local items and assuming that `bar` and `foo` are 
crates. While that would make `accessible($path)` a bit more ergonomic by
shaving off two characters, chances are, assuming the `uniform_paths` system,
that it would lead to surprises for some users who think that `bar` and `foo`
refer to the local crate. This is problematic because it is not immediately
evident for the user which is which since a different crate is needed to observe
the difference.

Also do note that requiring absolute paths with leading `::` is fully
forward-compatible with not requiring leading `::`. If we experience that
this restriction is a problem in the future, we may remove the restriction.

#### `#[cfg(accessible(..))` or `#[cfg(accessible = ..)`

We need to decide between the syntax `accessible(..)` or `accessible = ..`.
The reason we've opted for the former rather than the latter is that the
former syntax looks more like a question/query whilst the latter looks more
like a statement of fact.

In addition, if we would like to enable `accessible = $path` we would need to
extend the meta grammar. We could justify that change in and of itself by
observing that crates such as `serde_derive` permit users to write things like
`#[serde(default = "some::function")]`. By changing the grammar we can allow
users to instead write: `#[serde(default = some::function)]`.
However, in this case, `accessible($path)` seems the optimal notation.

If we would like to extend the meta grammar, we could do so by changing:

```
named_value : "=" lit ;

meta_or_lit : meta | lit ;
meta_or_lit_list : meta_or_lit "," meta_or_lit_list ","? ;
meta_list : "(" meta_or_lit_list ")" ;
meta : path ( named_value | meta_list )? ;
```

into:

```
lit_or_path : path | lit ;
named_value : "=" lit_or_path ;

meta_or_lit : meta | lit ;
meta_or_lit_list : meta_or_lit "," meta_or_lit_list ","? ;
meta_list : "(" meta_or_lit_list ")" ;
meta : path ( named_value | meta_list )? ;
```

#### The bikeshed

One might consider other names for the flag instead of `accessible`.
Some contenders are:

+ `reachable`
+ `path_accessible`
+ `usable`
+ `can_use`
+ `path_exists`
+ `have_path` (or `has_path`)
+ `have`
+ `have_item`
+ `path_reachable`
+ `item_reachable`
+ `item_exists`

##### `accessible`

Currently `accessible` is the choice because it clearly signals the intent
while also being short enough to remain ergonomic to use.
In particular, while `path_accessible` might be somewhat more unambiguous,
we argue that from the context of seeing `accessible(::std::foo::bar)`
it is clear that it is paths we are talking about because the argument
is a path and not something else.

##### `reachable`

The word `reachable` is also a synonym of `accessible` and is one character 
shorter. However, it tends to have a different meaning in code. Examples include:

+ `std::hint::unreachable_unchecked`
+ `std::unreachable`

All in all, we have chosen to go with `accessible` instead as the
more intuitive option.

##### `usable`

While `can_use` and `usable` are also strong contenders, we reject these options
because they may imply to the user that only things that you may `use $path;` can
go in there. Meanwhile, you may `#[cfg(accessible(::foo::MyTrait::my_method))`
which is *not* possible as `use ::foo::MyTrait::my_method;`. This also applies
to other associated items and inherent methods as well as `struct` fields.

##### `has_path`

Another strong contender is `has_path` or `have_path`.

However, this variant is vague with respect to what "having" something means.
In other words, it does not say whether it refers to being accessible and public,
or whether it is usable, and so on.

As we previously noted, having `path` in the
name is also somewhat redundant because it is clear that `::std::bar` is a path.

Another small wrinkle is that it is unclear whether it should be `have` or `has`.
That choice depends on what one things the subject is. For example, if one 
considers a module to be an "it", then it should probably be `has`.

One upside to `has_path` is that it has precedent from the `clang` compiler.
For example, a user may write: `#if __has_feature(cxx_rvalue_references)`
or `__has_feature(c_generic_selections)`.

Another benefit is that `has_` gives us the opportunity to introduce a family
of `has_path`, `has_feature`, and `has_$thing` if we so wish.

### `#[cfg(version(..))`

When it comes to `version(..)`, it is needed to support conditional compilation
of language features as opposed to library features as previously shown.
Also, as we've seen, `version(..)` does not subsume `accessible(..)` but is
rather a complementary mechanism.

One problem specific to `version(..)` is that it might get too `rustc` specific.
It might be difficult for other Rust implementations than `rustc` to work with
this version numbering as libraries will compile against `rustc`s release
numbering. However, it is possible for other implementations to follow
`rustc` in the numbering and what features it provides. This is probably not
too unreasonable as we can expect `rustc` to be the reference implementation
and that other ones will probably lag behind. Indeed, this is the experience
with `GHC` and alternative Haskell compilers.

#### The bikeshed - Argument syntax

We have roughly two options with respect to how the `version` flag may be specified:

1. `version = "<semver>"`
2. `version(<semver>)`

The syntax in 2. is currently an error in `#[cfg(..)]` as you may witness with:

```rust
// error[E0565]: unsupported literal
#[cfg(abracadabra(1.27))] fn bar() {}
                  ^^^^
```

[attr_grammar]: https://github.com/rust-lang/rust/blob/097c40cf6e1defc2fc49d521374254ee27f5f1fb/src/libsyntax/parse/attr.rs#L141-L149

However, the attribute grammar is [technically][attr_grammar]:

```rust
attribute  : "#" "!"? "[" path attr_inner? "]" ;
attr_inner : "[" token_stream "]"
           | "(" token_stream ")"
           | "{" token_stream "}"
           | "=" token_tree
           ;
```

Note in particular that `#[my_attribute(<token_stream>)]` is a legal production
in the grammar wherefore we can support `#[cfg(version(1.27.0))]` if we so wish.

[@eddyb]: https://github.com/eddyb

Given that syntax 2. is possible, we have decided to use it because as [@eddyb]
has noted, the `cfg` flags that use the `flag = ".."` syntax are all static as
opposed to dynamic. In other words, the semantics of `cfg(x = "y")` is that of
checking for a membership test within a fixed set determined ahead of time.
This set is also available through `rustc --print=cfg`.

What a user may infer from how other `cfg(flag = "..")` flags work is that
`version = ".."` checks for an *exact* version. But as we've seen before,
this interpretation is not the one in this RFC.

However, one reason to pick syntax 1. is that `version(..)` looks like a list.

#### The bikeshed - Attribute name

Naturally, there are other possible names for the flag. For example:

+ `rustc_version`
+ `compiler_version`
+ `min_version`

We pick the current naming because we believe it is sufficiently clear
while also short and sweet. However, `min_version` is a good alternative
to consider because it telegraphs the `>=` nature of the flag.

As for the `<semver>` syntax, it could also be adjusted such that
you could write `version(>= 1.27)`. We could also support exact version
checking (`==`) as well as checking if the compiler is below a certain version
(`<=`). There are also the "tilde requirements" and "wildcard requirements"
that Cargo features that we could add. However, as a first iteration,
`version(1.27.0)` is simple and covers most use cases.

### [version_check] as an alternative

Using the crate `version_check` we may conditionally compile using a `build.rs`
file. For example, the [dbg] crate does this:

```rust
// src/lib.rs:
// -----------------------------------------------------------------------------

#![cfg_attr(use_nightly, feature(core_intrinsics, specialization))]

// Deal with specialization:
// On nightly: typeof(expr) doesn't need to be Debug.
#[allow(dead_code)]
#[doc(hidden)]
pub struct WrapDebug<T>(pub T);
use std::fmt::{Debug, Formatter, Result};

#[cfg(use_nightly)]
impl<T> Debug for WrapDebug<T> {
    default fn fmt(&self, f: &mut Formatter) -> Result {
        use ::std::intrinsics::type_name;
        write!(f, "[<unknown> of type {} is !Debug]",
            unsafe { type_name::<T>() })
    }
}

...

// build.rs:
// -----------------------------------------------------------------------------

//!
//! This build script detects if we are nightly or not
//!

extern crate version_check;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if let Some(true) = version_check::is_nightly() {
        println!("cargo:rustc-cfg=use_nightly");
    }
}
```

The [version_check] crate also supports testing for a minimum `version(..)` with:

```rust
extern crate version_check;

if let Some((true, _)) = version_check::is_min_version("1.13.0") {
    println!("cargo:rustc-cfg=MIN_COMPILER_1_13");
}
```

However, this is quite verbose in comparison and requires you to invent
ad-hoc and crate-specific names for your `#[cfg(..)]` flags such as
`MIN_COMPILER_1_13` that will not be the same for every crate.
You will also need to repeat this per version you want to support.
This causes the mechanism to scale poorly as compared to `version(1.27)`
which we argue is simple and intuitive.

### Conditional compilation on feature gates

An alternative to `version(..)` and `accessible(..)` is to allow users
to query where a certain feature gate is stable or not.
However, it has been argued that allowing this would essentially stabilize
the names of the gates which we've historically not done.

We also argue that `accessible(..)` is more intuitive because it is more
natural to think of a feature in terms of how you would make use of it
(via its path) rather than the sometimes somewhat arbitrarily named feature gate.

## Prior art
[prior-art]: #prior-art

### Crates

[rustc_version]: https://crates.io/crates/rustc_version

As previously mentioned, the [version_check] crate provides precedent for
doing the desired conditional compilation in this RFC. There is also the
[rustc_version] crate. Together, these crates have 18 + 67 direct reverse
dependencies. This suggests that the feature is both desired and used.

### Haskell

Using the Glasgow Haskell Compiler (GHC), it is possible to conditionally
compile using it's provided preprocessor:

```haskell
{-# LANGUAGE CPP #-}

module Main where

version :: String
#if __GLASGOW_HASKELL__ >= 706
version = "Version 7.0.6"
#else
version = "Below."
#endif

main :: IO ()
main = putStrLn version
```

### Clang

[clang_check]: https://clang.llvm.org/docs/LanguageExtensions.html#feature-checking-macros

The `clang` compiler gives you a [suite of feature checking macros][clang_check] 
with which you can for example check whether a certain feature, extension,
or attribute is supported. An example of this is:

```cpp
#if __has_feature(cxx_rvalue_references)

// This code will only be compiled with the -std=c++11 and -std=gnu++11
// options, because rvalue references are only standardized in C++11.

#endif
```

This would be analogous to checking for the existence of a feature gate in Rust.

[clang_include]: https://clang.llvm.org/docs/LanguageExtensions.html#include-file-checking-macros

Clang also supports checking whether an [include][clang_include] will succeed.
For example, you may write:

```cpp
#if __has_include("myinclude.h") && __has_include(<stdint.h>)
#include "myinclude.h"
#endif
```

This is similar in spirit to `accessible($path)`.

## Unresolved questions
[unresolved]: #unresolved-questions

The ability to have optional cargo dependencies is out of scope for this RFC.

1. Is it technically feasible to implement `accessible(..)`?
   For example it could be hard if cfg-stripping runs before resolving things.

   @eddyb has indicated that:

   > The good news is that we should be able to resolve that during macro
   > expansion nowadays. The bad news is I don't know how hard early stability
   > checking would be although, no, we should be able to easily add a
   > `DefId -> Option<Stability>` method somewhere, with enough information to
   > check against feature-gates (assuming the set of `#![feature(...)]`s in
   > the local crate is known at `cfg`-stripping time).

2. Should we allow referring to fields of type definitions in `accessible(..)`?

3. In the [reference-level-explanation], we note that:
   > If and only if a Rust compiler considers itself to have a version which is
   > greater or equal to the version in the `semver` string will the
   > `#[cfg(version(<semver>)]` flag be considered active.

   However, it is currently not well specified what "considers itself" exactly
   means. To be more precise, if querying a mid-cycle nightly compiler with
   `rustc --version` results in `rustc 1.29.0-nightly (31f1bc7b4 2018-07-15)`,
   but 1.29.0 has not been released on the stable channel,
   will then `version(1.29.0)` be active for this nightly or will it not?

   The reason this question matters is because on one 1.29.0-nightly compiler,
   a feature may not have been stabilized. Some days later, but before 1.29.0
   hits a beta or stable compiler, a feature does get stabilized.

   To resolve this question, there are broadly 3 approaches:

   1. Answer the question in the affirmative.
      This entails that some breakage might sometimes occur when
      using a nightly compiler.

   2. Answer it in the negative by changing the date when the version constant
      is bumped in the compiler. That is, a version would only be bumped when
      releasing new stable or beta compilers and nightly compilers would always
      be versioned as the latest stable/beta. This also means that given
      `#[stable(feature = "foobar", since = "1.42.0")]` for some feature
      `foobar`, the feature would not be available first when the feature
      actually reaches stable/beta.

   3. As 2. but separate versions reported by `rustc --version` and to
      `version(..)`. This would for example mean that if the last
      stable compiler is `1.42.0`, then that would be used by `version(..)`
      while `rustc --version` would report `1.43.0-nightly`.
      This approach could be technically achieved by for example
      maintaining one version constant that tracks the last stable/beta
      compiler as `x.y.z` and then `--version` would report
      `x.(y + 1).0-nightly`.

   Two arguments in favour of either 2. or 3. is that they would be more
   principled as we have not really stabilized something until it reaches
   stable or beta.

   We consider this unresolved question to be a matter of implementation detail
   which may be resolved during implementation.

## Possible future work
[possible future work]: #possible-future-work

### `#[cfg(rust_feature(..))]`

[GAT]: https://github.com/rust-lang/rust/issues/44265

One possible extension we might want to do in the future is to allow users
to check whether a certain `rustc` feature gate is enabled or not.
For example, we might write `#[cfg(rustc_feature(generic_associated_types))]`
to check whether the [GAT] feature is supported in the compiler or not.

The main benefit of such an approach is that it is more direct than checking
for a particular version. Also note that `clang` uses this approach as noted
in the [prior art][prior-art].

However, there are some drawbacks as well:

1. The names of feature gates are not always aptly named and usually do not
   follow a coherent naming system. As a frequent author of RFCs, the author
   of this one knows that they do not have a principled approach to naming
   RFCs. The feature name that is then used in the compiler is usually drawn
   directly from the RFC, so we would either need to accept the random naming
   of feature gates, or we would need to impose some system.

2. Permitting dependence on the names of feature gates on stable would
   require us to be more principled with feature gates.
   For example, `rustc`, or any other Rust compiler, would be unable to
   remove gates or drastically change their implementations without changing
   their names. Being more principled could potentially add an undue burden
   on the library and compiler teams.

### `#[cfg(has_attr($attribute))]`

One possible extension would be to introduce a `has_attr(..)` feature.
`has_attr` would check if the specified attribute would be usable on the
item the `cfg` (or `cfg_attr`) directive is attached to. For instance:

```rust
#[cfg_attr(have_attr(must_use), must_use)]
fn double(x: i32) -> i32 {
    2 * x
}
```

This would allow code to detect the availability of an attribute before using it,
while not failing if the attribute did not exist.

Using `has_attr` in a `cfg` block may be useful for conditionally compiling
code that only makes sense if a given attribute exists (e.g. `global_allocator`), 
while using `has_attr` in a `cfg_attr` block may be useful for adding an
attribute to an item if supported but still support compilers that don't
support that attribute.

As previously discussed, currently, the names of feature gates do not tend to
appear in code targeting stable versions of Rust. Allowing code to detect the
availability of specified feature gates by name would require committing to stable names for these features, and would require that those names refer to
a fixed set of functionality. This would require additional curation.
However, as attribute names already have to be standardized,
`has_attr(..)` would not suffer the same problems wherefore
it may be the better solution.

### `#[cfg(nightly)]`

In a previous iteration of this RFC, a `#[cfg(nightly)]` flag was included.
However, this flag has since been removed from the RFC.
We may still add such a feature in the future if we wish.
Therefore, we have outlined what `nightly` would have meant
and some upsides and drawbacks to adding it.

#### Technical specification

To the `cfg` attribute, a `nightly` flag is added.

If and only if a Rust compiler permits a user to specify `#![feature(..)]`
will the `nightly` flag be considered active.

#### Drawbacks: Combining `nightly` and `accessible(..)`

Consider that a popular library writes:

```rust
#![cfg_attr(nightly, feature(some_feature))]
#[cfg(accessible(::std::foo:SomeFeature))]
use std::foo::SomeFeature;

#[cfg(not(accessible(::std::foo:SomeFeature)))]
struct SomeFeature { ... }
```

One potential hazard when writing this migrating construct is that
once `SomeFeature` finally gets stabilized, it may have been shipped
in a modified form. Such modification may include changing the names
of `SomeFeature`'s methods, their type signatures, or what trait
implementations exist for `SomeFeature`.

This problem only occurs when you combine `nightly` and `accessible(..)`
or indeed `nightly` and `version(..)`. However, there is a risk of breaking
code that worked on one stable release of Rust in one or more versions after.

A few mitigating factors to consider are:

+ It is possible to check if the methods of `SomeFeature` are `accessible`
  or not by using their paths. This reduces the risk somewhat.

+ If a crate author runs continuous integration (CI) builds that include
  testing the crate on a nightly toolchain, breakage can be detected
  well before any crates are broken and a patch release of the crate
  can be made which either removes the nightly feature or adjusts the
  usage of it. The remaining problem is that dependent crates may have
  `Cargo.lock` files that have pinned the patch versions of the crate.

+ Users should take care not to use this mechanism unless they are fairly
  confident that no consequential changes will be made to the library.
  A risk still exists, but it is opt-in.

However, at the end, compared to `feature = "unstable"`,
which reverse dependencies may opt out of, `nightly` can't be opted out of
(unless we add a mechanism to Cargo to perform such an override,
but this would be anti-modular).
This is the fundamental reason that for the time being,
we have not included `nightly` in the proposal.

#### Upsides

[dbg]: https://crates.io/crates/dbg

One reason for the inclusion of `#[cfg(nightly)]` is that it is useful on its
own to conditionally compile based on nightly/not as opposed to providing
an `unstable` feature in `Cargo.toml`. An example of this is provided by the
[dbg] crate which currently uses [version_check] to provide this automation.

#### Alternative `#![if_possible_feature(<feature>)]`

As an alternative to `#[cfg_attr(nightly, feature(<feature>))]`
we could permit the user to write `#![if_possible_feature(<feature>)]`.
The advantage of this is that it is quite direct with respect to intent.
However, adding this in terms of `nightly` already has precedent in
[version_check]. In addition, `nightly` also composes with other flags
using `any`, `not`, and `all`.

This alternative also suffers from the problems previously noted.

#### Naming of the attribute

If this flag were to be proposed again, it would probably be proposed under
a different name than `nightly`. Instead, a more apt name with respect to intent
would be `unstable_features`.
