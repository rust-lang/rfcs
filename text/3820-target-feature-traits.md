- Feature Name: (`target_feature_traits`)
- Start Date: (2025-05-25)
- RFC PR: [rust-lang/rfcs#3820](https://github.com/rust-lang/rfcs/pull/3820)
- Rust Issue: [rust-lang/rust#139368](https://github.com/rust-lang/rust/issues/139368)

# Summary
[summary]: #summary

This RFC proposes the addition of an `unsafe` variant of the existing `target_feature` attribute, and to modify the behaviour of the existing non-`unsafe` attribute to make it more compatible with traits.

See https://github.com/rust-lang/rust/issues/139368 for the discussion preceding this RFC and overall rationale.

### New behaviour of `#[target_feature(enable = "x")]` in traits

- Target features can be enabled on trait *definition* methods (safe or unsafe). This imposes on the caller the same requirements that calling a concrete function with those features would impose.
- Target features can be enabled on trait *impl* methods, as long as the corresponding definition enables a superset of the enabled features.
- Provided methods in traits count as both a definition and an impl, and behave accordingly (i.e. overridden impls can enable any subset of the features specified on the provided method).

Compared to the current status, this:
- allows to use target features in safe trait methods
- disallows using features in a trait impl that were not expected in the trait definition

For the second case, we plan to emit a lint and to make this into a hard-error in a future edition.

### New `#[unsafe(target_feature(force = "x"))]` attribute

This `unsafe` attribute can be applied to free functions, trait method implementations or trait method definitions.

It comes with the following soundness requirement: a function with the signature of the function the attribute is applied to must _only be callable if the force-enabled features are guaranteed to be present_ (this can be done, for example, if the function takes arguments that carry appropriate safety invariants).

Because of the soundness requirement, applying this attribute does not impose additional the requirements for calling this function on the callers (which is why the attribute can be applied to arbitrary trait methods).

The effect of the attribute is otherwise equivalent to `#[target_feature(enable = "x")]`.

Note that this version of the attribute can be used in the use-cases where `#[target_feature(enable = "x")]` would no longer be allowed.

Also note that the safety requirements of `#[unsafe(target_feature(force = "x"))]` only depend on the function's *signature*. This implies that an implementation (whether it is overriding a provided method or implementing a required method) should also be allowed to enable a feature (without needing to discharge any further safety requirements) if the corresponding definition had the `#[unsafe(target_feature(force = "x"))]` applied to it.
