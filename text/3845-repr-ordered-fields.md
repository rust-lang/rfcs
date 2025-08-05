- Feature Name: `repr_ordered_fields`
- Start Date: 2025-08-05
- RFC PR: [rust-lang/rfcs#3845](https://github.com/rust-lang/rfcs/pull/3845)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Introduce a new `repr` (let's call it `repr(ordered_fields)`, but that can be bikeshedded if this RFC is accepted) that can be applied to `struct`, `enum`, and `union` types, which guarantees a simple and predictable layout. Then provide an initial migration plan to switch users from `repr(C)` to `repr(ordered_fields)`.

# Motivation
[motivation]: #motivation

Currently `repr(C)` serves two roles
1. Provide a consistent, cross-platform, predictable layout for a given type
2. Match the target C compiler's struct/union layout algorithm and ABI

But in some cases, these two cases are in tension due to platform weirdness (even on major platforms like Windows MSVC)
* https://github.com/rust-lang/unsafe-code-guidelines/issues/521
* https://github.com/rust-lang/rust/issues/81996

Providing any fix for case 2 would subtly break any users of case 1, which makes this difficult to fix within a single edition. 

As an example of this tension: on Windows MSVC, `repr(C)` doesn't always match what MSVC does for  ZST structs (see this [issue](https://github.com/rust-lang/rust/issues/81996) for more details)

```rust
// should have size 8, but has size 0
#[repr(C)]
struct SomeFFI([i64; 0]);
```

Of course, making `SomeFFI` size 8 doesn't work for anyone using `repr(C)` for case 1. They want it to be size 0 (as it currently is). 

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Introduce a new `repr(ordered_fields)` which can be applied to `struct`, `enum`, and `union`. On all editions, `repr(ordered_fields)` would behave the same as `repr(C)` on edition 2024. (see reference level explanation for details). 

In editions 2024 (maybe <= 2024), any use of `repr(C)` will trigger a new warning, `edition_2024_repr_c` which will be warn by default.
This warning suggests a machine-applicable fix to switch `repr(C)` to `repr(ordered_fields)`, which is a no-op in the current edition, but helps prepare for changes to `repr(C)` early. This gives time for the community to update their code as needed.

For the FFI crates, they can safely ignore the warning by applying `#![allow(edition_2024_repr_c)]` to their crate root.
For crates without any FFI, they can simply run the machine-applicable fix.
For crates with a mix, they will need to do some work to figure out which is which. But this is unavoidable to solve the stated motivation.

For example, the warning could look like this:
```
warning: use of `repr(C)` in type `Foo`
  --> src/main.rs:14:10
   |
14 |     #[repr(C)]
   |       ^^^^^^^ help: consider switching to `repr(ordered_fields)`
   |     struct Foo {
   |
   = note: `#[warn(edition_2024_repr_c)]` on by default
   = note: `repr(C)` is planned to change meaning in the next edition to match the target platform's layout algorithm. This may change the layout of this type on certain platforms. To keep the current layout, switch to `repr(ordered_fields)`
```


On editions > 2024, `repr(ordered_fields)` may differ from `repr(C)`, so that `repr(C)` can match the platform's layout algorithm.

On all editions, once people have made the switch, this will make it easier to tell *why* the `repr` was applied to a given struct. If `repr(C)`, it's about FFI and interop. If `repr(ordered_fields)`, then it's for a dependable layout. This is more clear than today, where `repr(C)` fills both roles.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This feature only touches `repr(C)`, other reprs are left as is. It introduces exactly one new repr, `repr(ordered_fields)`, to take on one of the roles that `repr(C)` used to take.

`repr(ordered_fields)` will use the same layout algorithm that `repr(C)` currently uses. Details can be found in the [reference](https://doc.rust-lang.org/reference/type-layout.html?highlight=repr#reprc-structs). I will give an informal overview here.

For structs, `repr(ordered_fields)` lays out each field in memory according to the declaration order of the fields.

```rust
#[repr(ordered_fields)]
struct Foo {
	a: u32,
	b: u8,
	c: u32,
	d: u16,
}
```
Would be laid out like so (where `.` are padding bytes)
```
#####...######..
a   b   c   d
```

For unions, each field is laid out at offset 0, and never has niches.

For enums, the discriminant size is left unspecified (unless another repr specifies it like `repr(ordered_fields, u8)`), but is guaranteed to be stable across Rust versions for a given set of variants and fields in each variant.

Enums are defined as a union of structs, where each struct corresponds to each variant of the enum, with the discriminant prepended as the first field.

For example, `Foo` and `Bar` have the same layout in the example below modulo niches.

```rust
#[repr(ordered_fields, u32)]
enum Foo {
	Variant1,
	Variant2(u8, u64),
	Variant3 {
		name: String,
	}
}

#[repr(ordered_fields)]
union Bar {
    variant1: BarVariant1,
    variant2: BarVariant2,
    variant3: BarVariant3,
}

#[repr(ordered_fields)]
struct BarVariant1 {
    discr: u32,
}

#[repr(ordered_fields)]
struct BarVariant2(u32, u8, u64);

#[repr(ordered_fields)]
struct BarVariant3 {
    discr: u32,
    name: String,
}
```

Introduce a new `repr(ordered_fields)` which can be applied to `struct`, `enum`, and `union`. On all editions, `repr(ordered_fields)` would behave the same as `repr(C)` on edition 2024. 

On editions > 2024, `repr(ordered_fields)` may differ from `repr(C)`, so that `repr(C)` can match the platform's layout algorithm. For an extreme example, we could stop compiling `repr(C)` for ZST if the target C compiler doesn't allow ZSTs, or we could bump the size to 1 byte if the target C compiler does that by default (this is just an illustrative example, and not endorsed by RFC).

As mentioned in the guide-level explanation, in edition 2024 (maybe <= 2024), any use of `repr(C)` would trigger a new warn by default diagnostic, `edition_2024_repr_c`. This warning could be phased out after at least two editions have passed. This gives the community enough time to migrate any code over to `repr(ordered_fields)` before the next edition after 2024, but doesn't burden Rust forever.

The warning should come with a machine-applicable fix to switch `repr(C)` to `repr(ordered_fields)`, and this fix should be part of `cargo fix`. 
# Drawbacks
[drawbacks]: #drawbacks

* This will cause a large amount of churn in the Rust ecosystem
* If we don't end up switching `repr(C)` to mean the system layout/ABI, then we will have two identical reprs, which may cause confusion.
# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

* `crabi`: http://github.com/rust-lang/rfcs/pull/3470
	* Currently stuck in limbo since it has a much larger scope. doesn't actually serve to give a consistent cross-platform layout, since it defers to `repr(C)` (and it must, for its stated goals)
* https://internals.rust-lang.org/t/consistent-ordering-of-struct-fileds-across-all-layout-compatible-generics/23247
	* This doesn't give a predictable layout that can be used to match the layouts (or prefixes) of different structs
* https://github.com/rust-lang/rfcs/pull/3718
	* This one is currently stuck due to a larger scope than this RFC
* do nothing
	* We keep getting bug reports on Windows (and other platforms), where `repr(C)` doesn't actually match the target C compiler, or we break a bunch of subtle unsafe code to match the target C compiler.
# Prior art
[prior-art]: #prior-art

See Rationale and Alternatives as well

* https://rust-lang.zulipchat.com/#narrow/channel/213817-t-lang/topic/expand.2Frevise.20repr.28.7BC.2Clinear.2C.2E.2E.2E.7D.29.20for.202024.20edition

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* The migration plan, as a whole, needs to be ironed out
	* Currently, it is just a sketch, but we need timelines, dates, and guarantees to switch `repr(C)` to match the layout algorithm of the target C compiler.
	* Before this RFC is accepted, t-compiler will need to commit to fixing the layout algorithm sometime in the next edition.
* The name of the new repr `repr(ordered_fields)` is a mouthful (intentionally for this RFC), maybe we could pick a better name? This could be done after the RFC is accepted.
	* `repr(linear)`
	* `repr(ordered)`
	* `repr(sequential)`
	* something else?

# Future possibilities
[future-possibilities]: #future-possibilities

* Add more reprs for each target C compiler, for example `repr(C_gcc)` or  `repr(C_msvc)`, etc.
	* This would allow a single Rust app to target multiple compilers robustly, and would make it easier to specify `repr(C)`
	* This would also allow fixing code in older editions
* https://internals.rust-lang.org/t/consistent-ordering-of-struct-fileds-across-all-layout-compatible-generics/23247
