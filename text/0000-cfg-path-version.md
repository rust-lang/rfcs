- Feature Name: `cfg_path_version`
- Start Date: 2018-08-12
- RFC PR: _
- Rust Issue: _

# Summary
[summary]: #summary

Permit users to `#[cfg(..)]` on whether:

+ they are on a `nightly` compiler (`#[cfg(nightly)]`).
+ they have a certain minimum Rust version (`#[cfg(version = "1.27")]`).
+ a certain external path exists (`#[cfg(accessible(::std::mem::ManuallyDrop))]`).

# Motivation
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

...not much focus has been given to how you can improve the situation can be
improved by enhancing conditional compilation to extend how many versions back
a crate supports. This becomes critically important if and when we gain LTS
channels as proposed by [RFC 2483].

[version_check]: https://crates.io/crates/version_check

The current support for such conditional compilation is lacking.
While [it is possible][version_check] to check if you are on a nightly
compiler or to check if you are above a certain compiler version,
such facilities are not particularly ergonomic at the moment.
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

A minor use case this RFC supports is to work around compiler bugs by
checking if we are on a particular version.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## `#[cfg(nightly)]` and `#[cfg(accessible($path))]`

Consider for a moment that we would like to use the `Iterator::flatten`
method of the standard library if it exists, but otherwise fall back to
`Itertools::flatten`. We can do that with the following snippet:

```rust
#![cfg_attr(nightly, feature(iterator_flatten))]

#[cfg(accessible(::std::iter::Flatten))]
fn make_iter(limit: u8) -> impl Iterator<Item = u8> {
    (0..limit).map(move |x| (x..limit)).flatten()
}

#[cfg(not(accessible(::std::iter::Flatten)))]
fn make_iter() {
    use itertools::Itertools;
    (0..limit).map(move |x| (x..limit)).flatten()
}

fn main() {
    println!("{:?}", make_iter(10).collect::<Vec<_>>());
}
```

What this snippet does is the following:

1. If you happen to be on a nightly compiler, but not otherwise,
   the feature `iterator_flatten` will be enabled.

2. If the path `::std::iter::Flatten` exists, the compiler will compile
   the first version of `make_iter`. If the path does not exist,
   the compiler will instead compile the second version of `make_iter`.

The result of 1. and 2. is that your crate can opt into using `Iterator::flatten`
on nightly compilers but use `Itertools::flatten` on stable compilers meanwhile.
Once the standard library has stabilized `iter::Flatten`, future stable compilers
will start using the first version of the function. As a crate author, you 
don't have to publish any new versions of your crate for the compiler to
switch to the libstd version when it is available in the future.

[`proptest`]: https://github.com/altsysrq/proptest
[adding support]: https://github.com/AltSysrq/proptest/blob/67945c89e09f8223ae945cc8da029181822ce27e/src/num.rs#L66-L76

In this case we used the `nightly` and `accessible` flags to handle a problem
that the addition of `Iterator::flatten` caused for us if we had used
`Itertools::flatten`. We can also use these mechanisms for strictly additive
cases as well. Consider for example the [`proptest`] crate [adding support]
for `RangeInclusive`:

```rust
#[cfg_attr(nightly, feature(inclusive_range))]

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

## `#[cfg(version = "1.27")]`

Until now, we have only improved our support for library features but never
any language features. By checking if we are on a certain minimum version of
Rust or any version above it, we can conditionally support such new features.
For example:

```rust
#[cfg_attr(version = "1.27", must_use)]
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
#[cfg(version = "1.28")]
// or: #[cfg(accessible(::std::alloc::System))]
use std::alloc::System;

#[cfg_attr(version = "1.28", global_allocator)]
static GLOBAL: System = System;

fn main() {
    let mut v = Vec::new();
    // This will allocate memory using the system allocator.
    // ^--- But only on Rust 1.28 and beyond!
    v.push(1);
}
```

Note that you won't be able to make use of `#[cfg(version = "..")]` for these 
particular features since they were introduced before this RFC's features
get stabilized. This means that you can't for example add `version = "1.28"`
to your code and expect Rust 1.28 compilers to enable the code.
However, there will be features in the future to use this mechanism on.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## `#[cfg(nightly)]`

To the `cfg` attribute , a `nightly` flag is added.

If and only if a Rust compiler permits a user to specify `#![feature(..)]`
will the `nightly` flag be considered active.

## `#[cfg(version = "<semver>")]`

To the `cfg` attribute, a `version` flag is added.
This flag requires that a string literal be specified in it inside parenthesis.
The string literal must have the format:

```
semver : \d(.\d)?(.\d)? ;
```

[caret requirements]: https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#caret-requirements

If and only if a Rust compiler considers itself to have a version which is
greater or equal to the version in the `semver` string will the
`#[cfg(version = "<string>")]` flag be considered active.
Greater or equal is defined in terms of [caret requirements].

## `#[cfg(accessible($path))]`

To the `cfg` attribute, a `accessible` flag is added.
This flag requires that a `path` fragment be specified in it inside parenthesis
but not inside a string literal. The `$path` must start with leading `::`
and may not refer to any parts of the own crate (e.g. with `::crate::foo`,
`::self::foo`, or `::super::foo` if such paths are legal).
This restriction exists to ensure that the user does not try to
conditionally compile against parts of their own crate because that crate
has not been compiled when the `accessible` flag is checked on an item.

If and only if the path referred to by `$path` does exist and is public
will the `#[cfg(accessible($path))]` flag be considered active.
In checking whether the path exists or not, the compiler will consider
feature gated items to exist if the gate has been enabled.

If a path refers to an item inside an inherent implementation,
the path will be considered to exist if any configuration of generic
parameters can lead to the item. To check whether an item exists for
an implementation with a specific sequence of concrete types applied to
a type constructor, it is possible to use the `::foo::bar::<T>::item` syntax.

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
    accessible(foobar::Person::ssn),
    accessible(foobar::Shape::Triangle::sides),
    accessible(foobar::Shape::MaybeUninit::value)
))]
fn do_stuff() {
    ...
}
```

## `cfg_attr` and `cfg!`

Note that the above sections also apply to the attribute `#[cfg_attr(..)]`
as well as the special macro `cfg!(..)` in that `nightly`, `version = ".."`,
and `accessible(..)` are added to those as well.

# Drawbacks
[drawbacks]: #drawbacks

One argument is that hypothetically, if the standard library removed
some unstable item, then we might "not notice" if everyone uses it through
`#[cfg(accessible(..))]`.

## Incremental garbage code and its collection

It sometimes happens that feature gates never make it to stable and
that they instead get scrapped. This occurs infrequently.
However, when this does happen, code that is conditionally compiled under
`#[cfg(accessible(::std::the::obsoleted::path))]` will become garbage that
just sits around. Over time, this garbage can grow to a non-trivial amount.

However, if we provide LTS channels in the style of [RFC 2483],
then there are opportunities to perform some "garbage collection"
of definitions that won't be used when the LTS version changes.

# Rationale and alternatives
[alternatives]: #rationale-and-alternatives

## `accessible(..)`

The primary rationale for the `accessible` mechanism is that when you
want to support some library feature, it is some path you are thinking of
rather than what version it was added. For example, if you want to use
`ManuallyDrop`, you can just ask if it exists. The `version` is instead a
proxy for the feature. Instead of detecting if the path we want is available
or not via an indirection, we can just check if the path exists directly.
This way, a user does not have to look up the minimum version number for
the feature.

You may think that `version = ".."` subsumes `accessible(..)`.
However, we argue that it does not. This is the case because at the time of
enabling the `nightly` feature that enables the path in the standard library,
we do not yet know what minimum version it will be supported under.
If we try to support it with `version = ".."`, it is possible that we may
need to update the minimum version some small number of times.
However, doing so even once means that you will need to release new versions
of your crate. If you instead use `accessible(..)` you won't need to use
it even once unless the name of the path changes in-between.

### Preventing relative paths

The reason why we have enforced that all paths must start with `::` inside
`accessible(..)` is that if we allow relative paths, and users write
`accessible(self::foo)`, then they can construct situations such as:

```
#[cfg(accessible(self::bar)]
fn foo() {}

#[cfg(accessible(self::foo)]
fn bar() {}
```

One way around this is to collect all items before `cfg`-stripping,
but this can cause problems with respect to stage separation.
Therefore, we prevent this from occurring with a simple syntactic check.

### `#[cfg(accessible(..))` or `#[cfg(accessible = ..)`

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

```abnf
named_value : "=" lit ;

meta_or_lit : meta | lit ;
meta_or_lit_list : meta_or_lit "," meta_or_lit_list ","? ;
meta_list : "(" meta_or_lit_list ")" ;
meta : path ( named_value | meta_list )? ;
```

into:

```abnf
lit_or_path : path | lit ;
named_value : "=" lit_or_path ;

meta_or_lit : meta | lit ;
meta_or_lit_list : meta_or_lit "," meta_or_lit_list ","? ;
meta_list : "(" meta_or_lit_list ")" ;
meta : path ( named_value | meta_list )? ;
```

### The bikeshed

One might consider other names for the flag instead of `accessible`.
Some contenders are:

+ `path_accessible`
+ `can_use`
+ `path_exists`
+ `have_path`
+ `have`
+ `have_item`
+ `path_reachable`
+ `item_reachable`
+ `item_exists`

Currently `accessible` is the choice because it clearly signals the intent
while also being short enough to remain ergonomic to use.
In particular, while `path_accessible` might be somewhat more unambiguous,
we argue that from the context of seeing `accessible(::std::foo::bar)`
it is clear that it is paths we are talking about because the argument
is a path and not something else.

While `can_use` is also a strong contender, we reject this option because
it may imply to the user that only things that you may `use $path;` can
go in there. Meanwhile, you may `#[cfg(accessible(::foo::MyTrait::my_method))`
which is *not* possible as `use ::foo::MyTrait::my_method;`. This also applies
to other associated items and inherent methods as well as `struct` fields.

## `#[cfg(nightly)`

[dbg]: https://crates.io/crates/dbg

One reason for the inclusion of `#[cfg(nightly)]` is that it is useful on its
own to conditionally compile based on nightly/not as opposed to providing
an `unstable` feature in `Cargo.toml`. An example of this is provided by the
[dbg] crate which currently uses [version_check] to provide this automation.

However, as we've argued and demonstrated in the [guide-level-explanation],
the ability to `#[cfg(nightly)]` really shines when used in conjunction with
`#[cfg(accessible($path))]`.

### Alternative `#![if_possible_feature(<feature>)]`

As an alternative to `#[cfg_attr(nightly, feature(<feature>))]`
we could permit the user to write `#![if_possible_feature(<feature>)]`.
The advantage of this is that it is quite direct with respect to intent.
However, adding this in terms of `nightly` already has precedent in
[version_check]. In addition, `nightly` also composes with other flags
using `any`, `not`, and `all`.

## `#[cfg(version = "..")`

When it comes to `version = ".."`, it is needed to support conditional compilation
of language features as opposed to library features as previously shown.
Also, as we've seen, `version = ".."` does not subsume `accessible(..)` but is
rather a complementary mechanism.

One problem specific to `version = ".."` is that it might get too `rustc` specific.
It might be difficult for other Rust implementations than `rustc` to work with
this version numbering as libraries will compile against `rustc`s release
numbering. However, it is possible for other implementations to follow
`rustc` in the numbering and what features it provides. This is probably not
too unreasonable as we can expect `rustc` to be the reference implementation
and that other ones will probably lag behind. Indeed, this is the experience
with `GHC` and alternative Haskell compilers.

### The bikeshed - Argument syntax

We have two options with respect to how the `version` flag may be specified:

1. `version = "<semver>"`
2. `version("<semver>")`

The syntax in 2. is currently an error in `#[cfg(..)]` as you may witness with:

```rust
// error[E0565]: unsupported literal
#[cfg(abracadabra("1.27"))] fn bar() {}
```

We could allow this syntax. However, we have chosen the syntax in 1.
This with consistency with flags such as `target_feature = "bmi"`.
Another reason to go with `version = ".."` is that `version("..")`
looks like a list.

### The bikeshed - Attribute name

Naturally, there are other possible names for the flag. For example:

+ `rustc_version`
+ `compiler_version`
+ `min_version`

We pick the current naming because we believe it is sufficiently clear
while also short and sweet. However, `min_version` is a good alternative
to consider because it telegraphs the `>=` nature of the flag.

As for the `version_string` syntax, it could also be adjusted such that
you could write `version = ">= 1.27"`. We could also support exact version
checking (`==`) as well as checking if the compiler is below a certain version
(`<=`). However, as a first iteration, `version = "1.27"` is simple and covers
most use cases.

## [version_check] as an alternative

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

The [version_check] crate also supports testing for a minimum `version = ".."` with:

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
This causes the mechanism to scale poorly as compared to `version = "1.27"`
which we argue is simple and intuitive.

## Conditional compilation on feature gates

An alternative to `version = ".."` and `accessible(..)` is to allow users
to query where a certain feature gate is stable or not.
However, it has been argued that allowing this would essentially stabilize
the names of the gates which we've historically not done.

We also argue that `accessible(..)` is more intuitive because it is more
natural to think of a feature in terms of how you would make use of it
(via its path) rather than the sometimes somewhat arbitrarily named feature gate.

# Prior art
[prior-art]: #prior-art

## Crates

[rustc_version]: https://crates.io/crates/rustc_version

As previously mentioned, the [version_check] crate provides precedent for
doing the desired conditional compilation in this RFC. There is also the
[rustc_version] crate. Together, these crates have 18 + 67 direct reverse
dependencies. This suggests that the feature is both desired and used.

## Haskell

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

# Unresolved questions
[unresolved]: #unresolved-questions

The ability to have optional cargo dependencies is out of scope for this RFC.

1. Could we somehow have an allow-by-default lint that says
   *"these paths don't exist"* which could be enabled on `cfg_attr(nightly)`?
   This would be done to mitigate the accumulation of garbage code as
   discussed in the [drawbacks].

2. Is it technically feasible to implement `accessible(..)`?
   For example it could be hard if cfg-stripping runs before resolving things.

   @eddyb has indicated that:

   > The good news is that we should be able to resolve that during macro
   > expansion nowadays. The bad news is I don't know how hard early stability
   > checking would be although, no, we should be able to easily add a
   > `DefId -> Option<Stability>` method somewhere, with enough information to
   > check against feature-gates (assuming the set of `#![feature(...)]`s in
   > the local crate is known at `cfg`-stripping time).

3. Should we allow referring to fields of type definitions in `accessible(..)`?