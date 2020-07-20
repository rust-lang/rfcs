- Feature Name: `config_oneof`
- Start Date: 2020-07-18
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Addition of `oneof` to existing `all`, `any`, `not` configuration predicates in `#[cfg!()]` macros

# Motivation
[motivation]: #motivation

In a number of situations (particularly involving `no_std` and cross-target applications) it is useful to ensure that only one of a set of features are enabled.
It is important to enforce this due to the additive behaviour of features (i.e. dependencies may include the same sub-dependency with different features enabled. The result is a union of the enabled features), and difficult to specify for larger feature sets using existing predicates as the complexity for this increases exponentially with the number of exclusive features.

The desired outcome is the ability to specify `#[cfg(oneof(feature = "a", feature = "b"))] do_something()` and `#[cfg(not(oneof(feature = "a", feature = "b")))] compile_error!(...)` to specify exclusive features without manually defining all possible valid/invalid combinations for the exclusive subset of features, and simplify other configurations that benefit from the `oneof` predicate.
This allows authors of crates with exclusive feature sets specify this in a maintainable manner, and to communicate this to consumers without requiring users to infer the feature issue from compiler errors.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Additions to existing [guide](https://doc.rust-lang.org/reference/conditional-compilation.html)

``` 
ConfigurationOneof
   oneof ( ConfigurationPredicateList )
   
...

oneof() with a comma separated list of configuration predicates. It is true if at least one and only one predicate is true. In all other situations it is false.
```

## An Example

Using an existing implementation from [rust-rand-facade](https://github.com/ryankurte/rust-rand-facade/blob/master/src/lib.rs) expressing all possible combinations for a set of three exclusive features (`std`, `cortex_m`, and `os_rng`). This is a reasonably small example, it is worth noting that it is common for embedded crates to have [tens of exclusive features](https://github.com/stm32-rs/stm32f4xx-hal/blob/master/Cargo.toml#L67).

Using existing predicates:

```rust
#[cfg(all(feature = "std", feature = "cortex_m"))]
compile_error!("Only one of 'std', 'os_rng', or 'cortex_m' features may be enabled");

#[cfg(all(feature = "std", feature = "os_rng"))]
compile_error!("Only one of 'std', 'os_rng', or 'cortex_m' features may be enabled");

#[cfg(all(feature = "cortex_m", feature = "os_rng"))]
compile_error!("Only one of 'std', 'os_rng', or 'cortex_m' features may be enabled");

#[cfg(not(any(feature = "std", feature = "cortex_m", feature = "os_rng")))]
compile_error!("One of 'os_rng', 'std', 'cortex_m' features must be enabled");
```

Using the `oneof` predicate:
```rust
#[cfg(not(oneof(feature = "std", feature = "cortex_m", feature = "os_rng")))]
compile_error!("One of 'os_rng', 'std', 'cortex_m' features must be enabled");
```

The latter is significantly more simple, and does not introduce additional complexity when adding further exclusive feature(s).

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

- I do not believe this has significant interaction with other features, or requires any significant changes to the tooling or language
- Implementation should be straightforward, requiring the addition of a config predicate to `rustc` based on existing configuration predicates
- I do not believe there are significant corner cases to be considered

# Drawbacks
[drawbacks]: #drawbacks

N/A

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- This provides a useful improvement to the expressiveness of `#[cfg()]` macros and allows crate authors to provide actionable errors rather than depending on compiler failures
- An alternative to `not(oneof(...))` could be `notoneof(...)` or similar, however, this is less generally useful and less consistent with existing predicates
- Not implementing this means crate authors must either hand-define all possible variants (which is demonstrably neither commonplace or practicable with larger numbers of exclusive features) or rely on symbol errors to enforce feature exclusivity, and thus users will continue to need to infer the _cause_ of these failures without direct / explicit signals.

# Prior art
[prior-art]: #prior-art

A common mechanism for managing the _at least one_ requirement involves adding a meta-feature that is included by each target feature.
This is common in embedded-hal device implementations such as [stm32f4xx-hal](https://github.com/stm32-rs/stm32f4xx-hal/blob/master/Cargo.toml).
As this does not encompass _at most one_, enabling multiple exclusive features causes errors due to symbol duplication rather than a useful error message.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

N/A

# Future possibilities
[future-possibilities]: #future-possibilities

N/A
