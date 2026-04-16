- Feature Name: `repr_ordered_fields`
- Start Date: 2025-08-05
- RFC PR: [rust-lang/rfcs#3845](https://github.com/rust-lang/rfcs/pull/3845)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Introduce a few new `repr`s (all names can be bikeshedded if this RFC is accepted):
* `repr(ordered_fields)`
* `repr(C#editionCurr)`
* `repr(C#editionNext)`

And the meaning of `repr(C)` will change in the next edition (presumably `Edition 2027`). All the new reprs can be applied to `struct`, `enum`, and `union` types. 

`repr(ordered_fields)` provides a simple, predicable in memory layout for types. This RFC does *NOT* specify a stable ABI for `repr(ordered_fields)` - that should either be handled by a future RFC or another `repr`.
Layout-wise, this is the same as `repr(C)` on all current editions where both compile.

`repr(C#editionCurr)` is the same as `repr(C)` on all current editions. This will preserve the same layout and ABI as `repr(C)` on current editions. This repr is mainly targeted for use during the transition time. This way we can do an automated fix for `repr(C)` -> `repr(C#editionCurr)`, and you will know that this was done by an automated fix. If we used `repr(ordered_fields)` for this purpose, then it would be ambiguous if that was intentional or automated.

`repr(C#editionNext)` is the same as `repr(C)` on the next edition. This repr is mainly targeted for use during the transition time. This `repr` should *only* be used for FFI. This serves the dual purpose of `repr(C#editionCurr)`, it allows piecewise migration to the new edition while staying on the old edition.

`repr(C)`, this will be defined as the same representation as the platform C compiler, as specified by the target-triple. This `repr` should *only* be used for FFI.

Introduce a few new warnings and errors
1. A error when `repr(ordered_fields)` is used on enums without the tag type specified.
2. An edition migration warning, when updating to the next edition, that the meaning of `repr(C)` is changing. This will have an automated fix to `repr(C#editionCurr)`
3. A warn-by-default lint when `repr(C)` is used, and there are no `extern` blocks or functions in the crate (on all editions)
4. An idiom lint when `repr(C#editionNext)` is used on the next edition, with an automated fix to `repr(C)`.
5. An idiom lint when `repr(C#editionCurr)` is used on the next edition. No automated fix is provided, instead the lint should guide users to choose between `repr(C)` or `repr(ordered_fields)`

# Motivation
[motivation]: #motivation

Currently `repr(C)` serves two roles:
1. Provide a consistent, cross-platform, predictable layout for a given type
2. Match the target C compiler's struct/union layout algorithm and ABI

But in some cases, these two roles are in tension due to platform's C layout not matching the [simple, general algorithm](https://doc.rust-lang.org/stable/reference/type-layout.html#r-layout.repr.c.struct.size-field-offset) Rust always uses:
* On MSVC, a struct with a single field that is a zero-sized array has size 1. (https://github.com/rust-lang/rust/issues/81996)
* On MSVC, a `repr(C, packed)` with `repr(align)` fields has special behavior: the inner `repr(align)` takes priority, so fields can be highly aligned even in a packed struct. (https://github.com/rust-lang/rust/issues/100743)
  (Rust tries to avoid this by forbidding `repr(align)` fields in `repr(packed)` structs, but that check is easily bypassed with generics.)
* On MSVC-x32, `u64` fields are 8-aligned in structs, but the type is only 4-aligned when allocated on the stack. (https://github.com/rust-lang/rust/issues/112480)
* On AIX, `f64` fields sometimes but not always get 8-aligned in structs. (https://github.com/rust-lang/rust/issues/151910)

These are all niche cases, but they add up.
Furthermore, it is just fundamentally wrong for Rust to claim that `repr(C)` layout matches C code for the same target while also specifying an exact algorithm for `repr(C)`:
the C standard does not prescribe any particular struct layout, so any target/ABI is in principle free to come up with whatever bespoke rules they like.

However, fixing this is hard because of (unsafe) code that relies on the other role of `repr(C)`, giving a deterministic layout.
We therefore cannot just "fix" `repr(C)`, we need some sort of transition plan.
This code generally falls into one of these buckets:
* rely on the exact layout being consistent across platforms
	* for example, zero-copy deserialization (see [rkyv](https://crates.io/crates/rkyv))
* manually calculating the offsets of fields
	* This is common in code written before the stabilization of `offset_of` (currently only stabilized for `struct`)/`&raw const`/`&raw mut`
	* But sometimes this is still required if you are manually doing type-erasure and handling DSTs (for example, implementing [`erasable::Erasable`](https://docs.rs/erasable/1.3.0/erasable/trait.Erasable.html))
* manually calculating the layout for a DST, to prepare an allocation (see [slice-dst](https://crates.io/crates/slice-dst), specifically [here](https://github.com/CAD97/pointer-utils/blob/0fe399f8f7e519959224069360f3900189086683/crates/slice-dst/src/lib.rs#L162-L163))
* match layouts of two different types (or even, two different monomorphizations of the same generic type)
	* see [here](https://github.com/rust-lang/rust/pull/68099), where in `alloc` this is done for `Rc` and `Arc` to give a consistent layout for all `T`

So, providing any fix for role 2 of `repr(C)` would subtly break any users of role 1.
This breakage cannot be checked easily since it affects unsafe code making assumptions about data layouts, making it difficult to fix within a single edition/existing editions.

### Layout issues

Before we delve into the proposed solution, we go into a little more detail about the aforementioned platform layout issues.
Some of them cannot be solved with this RFC alone, but all of them have require some approach to split up the two roles of `repr(C)`.

## MSVC: zero-length arrays

On Windows MSVC, `repr(C)` doesn't always match what MSVC does for ZST structs (see this [issue](https://github.com/rust-lang/rust/issues/81996) for more details)

```rust
// should have size 8, but has size 0
#[repr(C)]
struct SomeFFI([i64; 0]);
```

Of course, making `SomeFFI` size 8 doesn't work for anyone using `repr(C)` for case 1. They want it to be size 0 (as it currently is). 

## MSVC: `repr(align)` inside `repr(packed)`

This also plays a role in [#3718](https://github.com/rust-lang/rfcs/pull/3718), where `repr(C, packed(N))` wants to allow fields which are `align(M)`.
On most targets, and with Rust's `repr(C, packed)` specification, the `packed` takes precedence:

```rust
#[repr(C, align(8))]
struct I(u8);

#[repr(C, packed)]
struct O {
  // At offset 0
  f1: u8,
  // At offset 1
  f2: I,
}
```

However, MSVC will put `f2` at offset 8, so arguably that is what `repr(C, packed)` should do on that target.
This is a footgun for normal uses of `repr(packed)`, so it would be better to relegate this strictly to the FFI use-case. However, since `repr(C)` plays two roles, this is difficult.

By splitting `repr(ordered_fields)`  off of `repr(C)`, we can allow `repr(C, packed(N))` to contain over-aligned fields (while making the struct less packed), and (continuing to) disallow `repr(ordered_fields, packed(N))` from containing aligned fields. This keeps the Rust-only case free of warts without compromising on FFI use-cases[<sup>1</sup>](#ordered_fields_align).

## MSVC-x32: u64 alignment

Splitting `repr(C)` also allows making progress on dealing with the MSVC "quirk" [rust-lang/rust/112480](https://github.com/rust-lang/rust/issues/112480).

The issue here is that MSVC is inconsistent about the alignment of `u64`/`i64` (and possibly `f64`). In MSVC, the alignment of `u64`/`i64` is reported to be 8 bytes by `alignof` and is correctly aligned in structs. However, when placed on the stack, MSVC doesn't ensure that they are aligned to 8 bytes, and may instead only align them to 4 bytes.
Our interpretation of this behavior is that `alignof` reports the *preferred* alignment (rather than the required alignment) for the type, and MSVC chooses to sometimes overalign `u64` fields in structs.

No matter the reason for this behavior, any proper solution to this issue will require reducing the alignment of `u64`/`i64` to 4 bytes, and adjusting `repr(C)` to treat `u64`/`i64`'s alignment as 8 bytes. This way, if you have references/pointers to `u64`/`i64` (for example, as out pointers), then the Rust side will not break when the C side passes a 4-byte aligned pointer (but not 8-byte aligned). This could happen if the C side put the integer on the stack, or was manually allocated at some 4-byte alignment.

## AIX: f64 alignment

For AIX, the issue is that `f64` is sometimes treated as aligned to 8 bytes and sometimes as aligned to 4 bytes (the comments indicate the desired layout as computed by a C compiler):
```rust
// Size: 24
#[repr(C)]
struct Floats {
  a: f64, // at offset 0
  b: u8, // at offset 8
  c: f64, // at offset 12
}
```
There is no way to obtain such a layout using Rust's `repr(C)` layout algorithm.
For more details, see this discussion on [irlo](https://internals.rust-lang.org/t/repr-c-aix-struct-alignment/21594/3).

Any fix for this requires splitting up `repr(C)`.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`repr(ordered_fields)` is a new representation that can be applied to `struct`, `enum`, and `union` to give them a consistent, cross-platform, and predictable in-memory layout.

`repr(C)` in edition <= 2024 is an alias for `repr(ordered_fields)` and in all other editions, it matches the default C compiler for the given target triple for structs, unions, and field-less enums. Enums with fields are laid out as a struct containing a tag and payload. With the payload being a union of structs of all the variants. (This is how they are currently laid out in `repr(C)`)

Using `repr(C)` in editions <= 2024 triggers a lint (seen below) as an edition migration compatibility lint with a machine-applicable fix that switches it to `repr(C#editionCurr)`.
* If you are using `repr(C)` for FFI, then you should switch to `repr(C#editionNext)`
* If you are not using `repr(C)` for FFI, then you should switch to `repr(ordered_fields)`

The machine-applicable fix is provided to allow users to do migrations on their own terms. This way a user can do `cargo fix`, update the edition themselves, then fix uses of `repr(C#editionCurr)` in the new edition.

```
warning: use of `repr(C)` in type `Foo`
  --> src/main.rs:14:10
   |
14 |     #[repr(C)]
   |       ^^^^^^^ help: consider switching to `repr(C#editionNext)` or `repr(ordered_fields)`
   |     struct Foo {
   |
   = note: `#[warn(edition_2024_repr_c)]` on by default
   = note: `repr(C)` is planned to change meaning in the next edition to match the target platform's layout algorithm. This may change the layout of this type on certain platforms. To keep the current layout, switch to `repr(C#editionNext)` or `repr(ordered_fields)`
```

Using `repr(C)` on all editions (including > 2024) when there are no extern blocks or functions in the crate will trigger a allow-by-default lint (`suspicious_repr_c`) suggesting to use `repr(ordered_fields)`.

This is allow by default, since the edition lint should do the heavy lifting. This reduces the noise, and provides a tool for interested users to reduce their reliance on `repr(C)` when it is probably not needed.

If *any* extern block or function (including `extern "Rust"`) is used in the crate, then the `suspicious_repr_c` lint will not be triggered. This way, we don't have too many false positives for this lint. However, the lint should *not* suggest adding a `extern` block or function, since the problem is likely the `repr`.

This does miss some potential use cases
1. where a crate provides a suite of FFI-capable types, but does not actually provide any `extern` functions or blocks.
2. the crate wants to interact with hardware, and using `repr(C)` is the correct repr
3. the crate wants is using shared memory with another process, and using `repr(C)` is the correct repr.

Since this is an allow-by-default lint, I think this is fine.

The `suspicious_repr_c` lint takes precedence over `edition_2024_repr_c` (i.e. `edition_2024_repr_c` shouldn't be emitted if `suspicious_repr_c` is emitted).

```
warning: use of `repr(C)` in type `Foo`
  --> src/main.rs:14:10
   |
14 |     #[repr(C)]
   |       ^^^^^^^ help: consider switching to `repr(ordered_fields)`
   |     struct Foo {
   |
   = note: `#[warn(suspicious_repr_c)]` on by default
   = note: `repr(C)` is intended for FFI, and since there are no `extern` blocks or functions, it's likely that you meant to use `repr(ordered_fields)` to get a stable and consistent layout for your type.
```

After enough time has passed, and the community has switched over:
This makes it easier to tell *why* the `repr` was applied to a given struct. If `repr(C)`, it's about FFI and interop. If `repr(ordered_fields)`, then it's for a dependable layout.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## `repr(C#editionCurr)`

see the current Rust reference entry for `repr(C)`: https://doc.rust-lang.org/stable/reference/type-layout.html#the-c-representation

## `repr(C#editionNext)`

> The `C#editionNext` representation is designed for one purpose: creating types that are interoperable with the C Language.
> 
> This representation can be applied to structs, unions, and enums. The exception is [zero-variant enums](https://doc.rust-lang.org/stable/reference/items/enumerations.html#zero-variant-enums) for which the `C#editionNext` representation is an error.
> 
> - edited version of the [reference](https://doc.rust-lang.org/stable/reference/type-layout.html#the-c-representation) on `repr(C)`

The exact algorithm is deferred to whatever the default target C compiler does with default settings (or if applicable, the most commonly used settings). `rustc` may grow extra flags to control the behavior of `repr(C)`, in order to match certain flags in the default C compiler, however those will need to be their own proposals. This RFC does not specify any extra control over `repr(C)`.

## `repr(C)`

This repr's meaning depends on the edition of the crate:
* on editions < `editionNext`, this means `repr(C#editionCurr)`
* on editions >= `editionNext`, this means `repr(C#editionNext)`

## `repr(ordered_fields)` 

> The `ordered_fields` representation is designed for one purpose: to create types that you can soundly perform operations on that rely on data layout, such as reinterpreting values as a different type
> 
> This representation can be applied to structs, unions, and enums.
> 
> - edited version of the [reference](https://doc.rust-lang.org/stable/reference/type-layout.html#the-c-representation) on `repr(C)`

### struct

When applying `repr(ordered_fields)` structs are laid out in memory in declaration order, with padding bytes added as necessary to preserve alignment.
The alignment of a struct is the same as the alignment of the most aligned field.

```rust
// assuming that u32 is aligned to 4 bytes
// size 16, align 4
#[repr(ordered_fields)]
struct FooStruct {
    a: u8,
    b: u32,
    c: u16,
    d: u32,
}
```

Would be laid out in memory like so

```
a...bbbbcc..dddd
```

### union

When applying `repr(ordered_fields)`, unions would be laid out as follows:
* the same alignment as their most aligned field
* the same size as their largest field, rounded up to the next multiple of the union's alignment
* all fields are at offset 0

```rust
// assuming that u32 is aligned to 4 bytes
// size 4, align 4
#[repr(ordered_fields)]
union FooUnion {
    a: u8,
    b: u32,
    c: u16,
    d: u32,
}
```

`FooUnion` has the same layout as `u32`, since `u32` has both the biggest size and alignment.

### enum

When applying `repr(ordered_fields)` to an enum, the tag type must be specified. i.e. `repr(ordered_fields, u8)` or `repr(ordered_fields, i32)`. If a tag type is not specified, then this results in a hard error, which should suggest the smallest integer type that can hold the discriminant values (preferring signed integers to break ties).

For discriminants, this means that it will follow the given algorithm for each variant in declaration order of the variants:
* if a variant has an explicit discriminant value, then that value is assigned
* else if this is the first variant in declaration order, then the discriminant is zero
* else the discriminant value is one more than the previous variant's discriminant (in declaration order)

If an enum doesn't have any fields, then it is represented exactly by its discriminant. 
```rust
// tag = i16
// represented as i16
#[repr(ordered_fields, i16)]
enum FooEnum {
    VarA = 1,
    VarB, // discriminant = 2
    VarC = 500,
    VarD, // discriminant = 501
}

// tag = u16
// represented as u16
#[repr(ordered_fields, u16)]
enum FooEnumUnsigned {
    VarA = 1,
    VarB, // discriminant = 2
    VarC = 500,
    VarD, // discriminant = 501
}
```

Enums with fields will be laid out as if they were a struct containing the tag and a union of structs containing the data.
NOTE: This is different from `repr(iN)`/`repr(uN)` which are laid out as a union of structs, where the first field of the struct is the tag.
These two layouts are *NOT* compatible, and adding `repr(ordered_fields)` to `repr(iN)`/`repr(uN)` changes the layout of the enum!

For example, this would be laid out the same as the union below
```rust
#[repr(ordered_fields, i8)]
enum BarEnum {
    VarFieldless,
    VarTuple(u8, u32),
    VarStruct {
        a: u16,
        b: u32,
    },
}
```

```rust
#[repr(ordered_fields)]
struct BarEnumRepr {
	tag: BarTag,
	data: BarEnumData,
}

#[repr(ordered_fields)]
union BarEnumData {
    var1: VarFieldless,
    var2: VarTuple,
    var3: VarStruct,
}

#[repr(ordered_fields, i8)]
enum BarTag {
    VarFieldless,
    VarTuple,
    VarStruct,
}

#[repr(ordered_fields)]
struct VarFieldless;

#[repr(ordered_fields)]
struct VarTuple(u8, u32);

#[repr(ordered_fields)]
struct VarStruct {
	a: u16,
	b: u32
}
```

In Rust, the algorithm for calculating the layout is defined precisely as follows:

```rust
/// Takes in the layout of each field (in declaration order)
/// and returns the offsets of each field, and the layout of the entire struct
fn get_layout_for_struct(field_layouts: &[Layout]) -> Result<(Vec<usize>, Layout), LayoutError> {
    let mut layout = Layout::new::<()>();
    let mut field_offsets = Vec::new();
    
    for &field in field_layouts {
        let (next_layout, offset) = layout.extend(field)?;
        
        field_offsets.push(offset);
        layout = next_layout;
    }
    
    Ok((field_offsets, layout.pad_to_align()))
}

fn layout_max(a: Layout, b: Layout) -> Result<Layout, LayoutError> {
    Layout::from_size_align(
        a.size().max(b.size()),
        a.align().max(b.align()),
    )
}

/// Takes in the layout of each field (in declaration order)
/// and returns the layout of the entire union
/// NOTE: all fields of the union are located at offset 0
fn get_layout_for_union(field_layouts: &[Layout]) -> Result<Layout, LayoutError> {
    let mut layout = Layout::new::<()>();
    
    for &field in field_layouts {
        layout = layout_max(layout, field)?;
    }
    
    Ok(layout.pad_to_align())
}

/// Takes in the layout of each variant (and their fields) (in declaration order), and returns the layout of the entire enum
/// the offsets of all fields of the enum are left as an exercise for the readers
/// NOTE: the enum tag is always at offset 0
fn get_layout_for_enum(
    // the discriminants may be negative for some enums
    // or u128::MAX for some enums, so there is no one primitive integer type that works. So BigInteger
    discriminants: &[BigInteger],
    variant_layouts: &[&[Layout]]
) -> Result<Layout, LayoutError> {
    assert_eq!(discriminants.len(), variant_layouts.len());

    let variant_data_layouts = variant_layouts.iter()
        // each variant's fields are represented as a struct
        .map(|variant_fields_layout| get_layout_for_struct(variant_fields_layout).map(|x| x.1))
        .collect::<Result<Vec<_>, _>>()?;

    // then the set of all variants is represented as a union
    let variant_data_layout = get_layout_for_union(variant_data_layouts)?;

    let tag_layout = get_layout_for_tag(discriminants);

    // the tag is then prepended to that union
    let (_, layout) = get_layout_for_struct(&[
        tag_layout,
        variant_data_layout
    ])?;

    Ok(layout)
}
```

## Migration Plan

The migration will be handled as follows:
* after the reprs outlined in this RFC are implemented
    * at this point `repr(C)` and `repr(C#editionCurr)` will have identical behavior
    * add an edition migration lint for `repr(C)` (`edition_2024_repr_c`)
    	* this warning should be advertised publicly (maybe on the Rust Blog?), so that as many people use it. Since even if you are staying on edition <= 2024, it is helpful to switch to `repr(ordered_fields)` to make your intentions clearer
    * add the `suspicious_repr_c` lint to help people migrate away from `repr(C)`. 
* Once the next edition rolls around (2027?)
    * at this point all of `repr(C)` and `repr(C#editionNext)` will have identical behavior
    * `repr(C)` on the new edition will *not* warn. Instead, the meaning will have changed to mean *only* compatibility with C. The docs should be adjusted to mention this edition wrinkle.
    * The warning for previous editions will continue to be in effect
    * The two idiom lints will come into effect to provide an off ramp for `repr(C#editionCurr)` and  `repr(C#editionNext)`
* Once the following edition rolls around (2030?)
    * `repr(C#editionCurr)` and `repr(C#editionNext)` are no longer valid. You may only use `repr(ordered_fields)` or `repr(C)`

# Drawbacks
[drawbacks]: #drawbacks

* This will cause a large amount of churn in the Rust ecosystem
	* This is only necessary for those who are updating to the new edition. Which is as little churn as we can make it
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
* Should this `repr` be versioned?
    * This way we can evolve the repr (for example, by adding new niches)
* Should we change the meaning of `repr(C)` in editions <= 2024 after we have reached edition 2033 or some other later edition? Yes, it's a breaking change. But at that point, it will likely only be breaking code no one uses.
    * Leaning towards no
* Is the ABI of `repr(ordered_fields)` specified (making it safe for FFI)? Or not?
    * discussion: https://github.com/rust-lang/rfcs/pull/3845#discussion_r2291506953
* What should `repr(C)` do when a given type wouldn't compile in the corresponding `C` compiler (like fieldless structs in MSVC)? 
	* discussion: https://github.com/rust-lang/rfcs/pull/3845#discussion_r2319138105
* <a id="ordered_fields_align"></a>Should `repr(ordered_fields, packed(N))` allow `align(M)` types where `M > N` (overaligned types).
	* discussion: https://github.com/rust-lang/rfcs/pull/3845#discussion_r2319098177
	* One option is to allow it and cap those fields to be aligned to `N`. This seems consistent with the handling of other over-aligned types. (i.e. putting a `u32` in a `repr(packed(2))` type)
* ~~Should unions expose some niches?~~ [no](https://github.com/rust-lang/rfcs/pull/3845#discussion_r3088073911)
    * For example, if all variants of the union are structs that have a common prefix, then any niches of that common prefix could be exposed (i.e. in the enum case, making a union of structs behave more like an enum).
    * This must be answered before stabilization, as it is set in stone after that
* Should we warn on `repr(ordered_fields)` applied to enums when explicit tag type is missing (i.e. no `repr(u8)`/`repr(i32)`)
	* Since it's likely they didn't want the same tag type as `C`, and wanted the smallest possible tag type
* What should the lints look like? (can be decided after stabilization if needed, but preferably this is hammered out before stabilization and after this RFC is accepted)
* The name of the new repr `repr(ordered_fields)` is a mouthful (intentionally for this RFC), maybe we could pick a better name? This could be done after the RFC is accepted.
    * `repr(linear)`
    * `repr(ordered)`
    * `repr(sequential)`
    * `repr(serial)`
    * `repr(consistent)`
    * `repr(declaration_order)`
    * something else?
* The name of the new repr `repr(C#editionCurr)` and `repr(C#editionNext)` are bad (intentionally for this RFC), maybe we could pick a better name? This could be done after the RFC is accepted.
    * `repr(C#edition2024)`/`repr(C#edition2027)`
    * `repr(e24#C)`/`repr(e27#C)`
    * `repr(e2024#C)`/`repr(e2027#C)`
    * ~~`repr(C24)`/`repr(C27)`~~ - likely too confusing to actual C standards
    * something else?

# Future possibilities
[future-possibilities]: #future-possibilities

* Add more reprs for each target C compiler, for example `repr(C_gcc)` or  `repr(C_msvc)`, etc.
    * This would allow a single Rust app to target multiple compilers robustly, and would make it easier to specify `repr(C)`
    * This would also allow fixing code in older editions
* https://internals.rust-lang.org/t/consistent-ordering-of-struct-fileds-across-all-layout-compatible-generics/23247
