- Feature Name: `aligned`
- Start Date: 2022-09-24
- RFC PR: [rust-lang/rfcs#3319](https://github.com/rust-lang/rfcs/pull/3319)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

Add an `Aligned` marker trait to `core::marker`, as a supertrait of the `Sized` trait. `Aligned` is implemented for all types with an alignment determined at compile time. This includes all `Sized` types, as well as slices and records containing them. Relax `core::mem::align_of<T>()`'s trait bound from `T: Sized` to `T: ?Sized + Aligned`.

# Motivation

Data structures and containers that wish to store unsized types are easier to implement if they can produce a dangling, well-aligned pointer to the unsized type. Being able to determine a type's alignment without a value of the type allows doing this.

In addition, this RFC allows implementing certain object-safe traits for slices, where this was not possible before.

# Guide-level explanation

`Aligned` is a marker trait defined in `core::marker`. It's automatically implemented for all types with an alignment determined at compile time. This includes all `Sized` types (`Aligned` is a supertrait of `Sized`), as well as slices and records containing them. Trait objects are not `Aligned`.

You can't implement `Aligned` yourself.

To get the alignment of a type that implements `Aligned`, call `core::mem::align_of<T>()`.

Implied `Sized` bounds also imply `Aligned`, because `Aligned` is a supertrait of `Sized`. To bound a type parameter by `Aligned` only, write `?Sized + Aligned`.

# Reference-level explanation

`Aligned` is not object-safe. Trait methods bounded by `Self: Aligned` can't be called from a vtable, but don't affect the object safety of the trait as a whole, just like `Self: Sized` currently.
Relaxing `Self: Sized` bounds to `Self: Aligned` allows implementing certain object-safe traits for slices, where this was not previously possible.

# Drawbacks

- Slightly compicates situation around implied `Sized` bounds.
- May make certain object safety diagnostics more confusing, as they will now refer to the new, lesser-known `Aligned` trait instead of `Sized`.

# Rationale and alternatives

`core::mem::align_of<T>()` for slices could be implemented with a library. However, a library would be unable to support records that contain a slice as the last field. Also, relaxing the trait dyn safety requirements can only be done with a language feature.

# Prior art

None that I am aware of.

# Unresolved questions

- Should `Aligned` be `#[fundamental]`? `Sized` is.

# Future possibilities

- Relaxing `NonNull::<T>::dangling()`'s trait bound from `T: Sized` to `T: ?Sized + Aligned + Pointee<Metadata: ~const Default>` may be desirable once the necessary library and language features are stabilized.
- `extern type`s may want to be able to implement `Aligned`.
- `Aligned` may warrant an addition the next edition's prelude.
