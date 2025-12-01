- Feature Name: `repr_ordered_fields`
- Start Date: 2025-08-05
- RFC PR: [rust-lang/rfcs#3845](https://github.com/rust-lang/rfcs/pull/3845)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Introduce a new `repr` (let's call it `repr(ordered_fields)`, but that can be bikeshedded if this RFC is accepted) that can be applied to `struct`, `enum`, and `union` types, which guarantees a simple and predictable layout. Then provide an initial migration plan to switch users from `repr(C)` to `repr(ordered_fields)`. This allows restricting the meaning of `repr(C)` to just serve the FFI use-case.

Introduce two new warnings
1. An edition migration warning, when updating to the next edition, that the meaning of `repr(C)` is changing
2. A warn-by-default lint when `repr(ordered_fields)` is used on enums without the tag type specified. Since this is likely not what the user wanted
3. A warn-by-default lint when `repr(C)` is used, and there are no `extern` blocks or functions in the crate (on all editions)

# Motivation
[motivation]: #motivation

Currently `repr(C)` serves two roles
1. Provide a consistent, cross-platform, predictable layout for a given type
2. Match the target C compiler's struct/union layout algorithm and ABI

But in some cases, these two cases are in tension due to platform weirdness (even on major platforms like Windows MSVC)
* https://github.com/rust-lang/unsafe-code-guidelines/issues/521
* https://github.com/rust-lang/rust/issues/81996

Code in case 1 generally falls into one of these buckets:
* rely on the exact layout being consistent across platforms
	* for example, zero-copy deserialization (see [rkyv](https://crates.io/crates/rkyv))
* manually calculating the offsets of fields
	* This is common in code written before the stabilization of `offset_of` (currently only stabilized for `struct`)/`&raw const`/`&raw mut`
	* But sometimes this is still required if you are manually doing type-erasure and handling DSTs (for example, implementing [`erasable::Erasable`](https://docs.rs/erasable/1.3.0/erasable/trait.Erasable.html))
* manually calculating the layout for a DST, to prepare an allocation (see [slice-dst](https://crates.io/crates/slice-dst), specifically [here](https://github.com/CAD97/pointer-utils/blob/0fe399f8f7e519959224069360f3900189086683/crates/slice-dst/src/lib.rs#L162-L163))
* match layouts of two different types (or even, two different monomorphizations of the same generic type)
	* see [here](https://github.com/rust-lang/rust/pull/68099), where in `alloc` this is done for `Rc` and `Arc` to give a consistent layout for all `T`

So, providing any fix for case 2 would subtly break any users of case 1. This breakage cannot be checked easily since it affects unsafe code making assumptions about data layouts. Making it difficult to fix within a single edition/existing editions.

Here are some examples of the tension and some other RFCs which could benefit from splitting up `repr(C)`'s two cases.

1. Windows MSVC ZSTs
2. the RFC [#3718](https://github.com/rust-lang/rfcs/pull/3718) for `repr(C, packed(N))` containing overaligned fields
3. A Windows MSVC bug
4. An [AIX](https://internals.rust-lang.org/t/repr-c-aix-struct-alignment/21594) layout bug

Examples 3 and 4 cannot be solved with this RFC alone, but any fix would require splitting up `repr(C)`.

## MSVC ZST

As an example of this tension: on Windows MSVC, `repr(C)` doesn't always match what MSVC does for ZST structs (see this [issue](https://github.com/rust-lang/rust/issues/81996) for more details)

```rust
// should have size 8, but has size 0
#[repr(C)]
struct SomeFFI([i64; 0]);
```

Of course, making `SomeFFI` size 8 doesn't work for anyone using `repr(C)` for case 1. They want it to be size 0 (as it currently is). 

## RFC #3718

This also plays a role in [#3718](https://github.com/rust-lang/rfcs/pull/3718), where `repr(C, packed(N))` wants allow fields which are `align(M)` (while making the `repr(C, ...)` struct less packed). This is a footgun for normal uses of `repr(packed)`, so it would be better to relegate this strictly to the FFI use-case. However, since `repr(C)` plays two roles, this is difficult.

By splitting `repr(ordered_fields)`  off of `repr(C)`, we can allow `repr(C, packed(N))` to contain over-aligned fields (while making the struct less packed), and (continuing to) disallow `repr(ordered_fields, packed(N))` from containing aligned fields. Thus keeping the Rust-only case free of warts, without compromising on FFI use-cases[<sup>1</sup>](#ordered_fields_align).

## MSVC bug

Splitting `repr(C)` also allows making progress on a workaround for the MSVC bug [rust-lang/rust/112480](https://github.com/rust-lang/rust/issues/112480). 

The issue here is that MSVC is inconsistent about the alignment of `u64`/`i64` (and possibly `f64`). In MSVC, the alignment of `u64`/`i64` is reported to be 8 bytes by `alignof` and is correctly aligned in structs. However, when placed on the stack, MSVC doesn't ensure that they are aligned to 8 bytes, and may instead only align them to 4 bytes.

Any proper workaround will require reducing the alignment of `u64`/`i64` to 4 bytes, and adjusting what `repr(C)` to treat `u64`/`i64`'s alignment as 8 bytes. This way, if you have references/pointers to `u64`/`i64` (for example, as out pointers), then the Rust side will not break when the C side passes a 4-byte aligned pointer (but not 8-byte aligned). This could happen if the C side put the integer on the stack, or was manually allocated at some 4-byte alignment.

For AIX, the issue is that `f64` is treated as aligned to 4 bytes if it is not the first field in a struct. i.e.
```C
struct Foo {
	char a;
	double b;
}
```
Field `b` would be laid out at offset 4, which is under-aligned (since `f64` has alignment 8 in Rust). Again, any proper workaround will require reducing the alignment of `f64`, and adjusting `repr(C)`.

## AIX layout bug

For more details, see this discussion on [irlo](https://internals.rust-lang.org/t/repr-c-aix-struct-alignment/21594/3).

In AIX, the following struct `Floats` has the following field offsets: `[0, 8, 12]` (in bytes) and a size of 24 bytes. Since the first field has a natural alignment of 8 bytes - AKA the size is 8 bytes.

```C
struct Floats {
    double a;
    char b;
    double c;
};
```

This is because
> In aggregates, the first member of this data type is aligned according to its natural alignment \[its size\] value; subsequent members of the aggregate are aligned on 4-byte boundaries.
> - [IBM Documentation](https://www.ibm.com/docs/en/xl-c-and-cpp-aix/16.1?topic=data-using-alignment-modes) (Table 1, Note 1, which applies to `double` and `long double` data types)

On AIX `__alignof__(double)` is 8, but field `c` is laid out at a 4-byte boundary. This is fine because `__alignof__` designates the *preferred* alignment, not the *required* alignment. Note that in Rust, we only ever use the *required* alignment and don't have a concept of a *preferred* alignment. So in Rust, we have designated the alignment of `f64` to be 8 bytes.

Any fix for this would require splitting up `repr(C)`, since reducing the alignment of `f64` would reduce the size of `Floats` from `24` to `20`, which also doesn't match `C`, and we cannot special case the alignment of `Floats` to be larger since that doesn't match the algorithm currently specified for `repr(C)` (making it a breaking change).

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`repr(ordered_fields)` is a new representation that can be applied to `struct`, `enum`, and `union` to give them a consistent, cross-platform, and predictable in-memory layout.

`repr(C)` in edition <= 2024 is an alias for `repr(ordered_fields)` and in all other editions, it matches the default C compiler for the given target for structs, unions, and field-less enums. Enums with fields will be laid out as if they are a union of structs with the corresponding fields.

Using `repr(C)` in editions <= 2024 triggers a lint to use `repr(ordered_fields)` as an edition migration compatibility lint with a machine-applicable fix. If you are using `repr(C)` for FFI, then you may silence this lint. If you are using `repr(C)` for anything else, please switch over to `repr(ordered_fields)` so updating to future editions doesn't change the meaning of your code.

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

Using `repr(C)` on all editions (including > 2024) when there are no extern blocks or functions in the crate will trigger a warn-by-default lint suggesting to use `repr(ordered_fields)`. Since the most likely reason to do this is if you haven't heard of `repr(ordered_fields)` or are upgrading to the most recent Rust version (which now contains `repr(ordered_fields)`).

If *any* extern block or function (including `extern "Rust"`) is used in the crate, then this lint will not be triggered. This way, we don't have too many false positives for this lint. However, the lint should *not* suggest adding a `extern` block or function, since the problem is likely the `repr`.

This does miss one potential use case, where a crate provides a suite of FFI-capable types, but does not actually provide any `extern` functions or blocks. This should be an extremely small minority of crates, and they can silence this warning crate-wide.

The `suspicious_repr_c` lint takes precedence over `edition_2024_repr_c`.

```
warning: use of `repr(C)` in type `Foo`
  --> src/main.rs:14:10
   |
14 |     #[repr(C)]
   |       ^^^^^^^ help: consider switching to `repr(ordered_fields)`
   |     struct Foo {
   |
   = note: `#[warn(suspicious_repr_c)]` on by default
   = note: `repr(C)` is intended for FFI, and since there are no `extern` blocks or functions, it's likely that you meant to use `repr(ordered_fields)` to get a stable and consistent layout for your type
```

After enough time has passed, and the community has switched over:
This makes it easier to tell *why* the `repr` was applied to a given struct. If `repr(C)`, it's about FFI and interop. If `repr(ordered_fields)`, then it's for a dependable layout.
# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## `repr(C)`

> The `C` representation is designed for one purpose: creating types that are interoperable with the C Language.
> 
> This representation can be applied to structs, unions, and enums. The exception is [zero-variant enums](https://doc.rust-lang.org/stable/reference/items/enumerations.html#zero-variant-enums) for which the `C` representation is an error.
> 
> - edited version of the [reference](https://doc.rust-lang.org/stable/reference/type-layout.html#the-c-representation) on `repr(C)`

The exact algorithm is deferred to whatever the default target C compiler does with default settings (or if applicable, the most commonly used settings). 
## `repr(ordered_fields)` 

> The `ordered_fields` representation is designed for one purpose: to create types that you can soundly perform operations on that rely on data layout, such as reinterpreting values as a different type
> 
> This representation can be applied to structs, unions, and enums.
> 
> - edited version of the [reference](https://doc.rust-lang.org/stable/reference/type-layout.html#the-c-representation) on `repr(C)`
### struct
Structs are laid out in memory in declaration order, with padding bytes added as necessary to preserve alignment.
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
Unions would be laid out with the same size as their largest field, and the same alignment as their most aligned field. 

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
The enum's tag type is the same type that is used for `repr(C)` in edition <= 2024, and the discriminants are assigned the same way as `repr(C)` (in edition <= 2024).  This means the discriminants are assigned such that each variant without an explicit discriminant is exactly one more than the previous variant in declaration order.
This does mean that the tag type will be platform-specific. To alleviate this concern, using `repr(ordered_fields)` on an enum without an explicit `repr(uN)`/`repr(iN)` will trigger a warning (name TBD). This warning should suggest the smallest integer type that can hold the discriminant values (preferring signed integers to break ties).

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

    let variant_data_layout = variant_layouts.iter()
        .try_fold(
            Layout::new::<()>(),
            |acc, variant_layout| Ok(layout_max(acc, get_layout_for_struct(variant_layout)?.1)?)
        )?;
    
    let tag_layout = get_layout_for_tag(discriminants);

    let (_, layout) = get_layout_for_struct(&[
        tag_layout,
        variant_data_layout
    ])?;

    Ok(layout)
}
```
### Migration to `repr(ordered_fields)`

The migration will be handled as follows:
* after `repr(ordered_fields)` is implemented
    * add an edition migration lint for `repr(C)`
    	* this warning should be advertised publicly (maybe on the Rust Blog?), so that as many people use it. Since even if you are staying on edition <= 2024, it is helpful to switch to `repr(ordered_fields)` to make your intentions clearer
    * at this point both `repr(ordered_fields)` and `repr(C)` will have identical behavior
    * the warning will come with a machine-applicable fix
        * Any crate that does not have FFI can just apply the autofix
        * Any crate which uses `repr(C)` for FFI can ignore the warning crate-wide
        * Any crate that mixes both must do extra work to figure out which is which. (This is likely a tiny minority of crates)
* Once the next edition rolls around (2027?), `repr(C)` on the new edition will *not* warn. Instead, the meaning will have changed to mean *only* compatibility with C. The docs should be adjusted to mention this edition wrinkle.
    * The warning for previous editions will continue to be in effect

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
* The name of the new repr `repr(ordered_fields)` is a mouthful (intentionally for this RFC), maybe we could pick a better name? This could be done after the RFC is accepted.
    * `repr(linear)`
    * `repr(ordered)`
    * `repr(sequential)`
    * `repr(consistent)`
    * `repr(declaration_order)`
    * something else?
* Is the ABI of `repr(ordered_fields)` specified (making it safe for FFI)? Or not?
* Should unions expose some niches?
    * For example, if all variants of the union are structs that have a common prefix, then any niches of that common prefix could be exposed (i.e. in the enum case, making a union of structs behave more like an enum).
    * This must be answered before stabilization, as it is set in stone after that
* Should this `repr` be versioned?
    * This way we can evolve the repr (for example, by adding new niches)
* Should we change the meaning of `repr(C)` in editions <= 2024 after we have reached edition 2033? Yes, it's a breaking change. But at that point, it will likely only be breaking code no one uses.
    * Leaning towards no
* Should we warn on `repr(ordered_fields)` applied to enums when explicit tag type is missing (i.e. no `repr(u8)`/`repr(i32)`)
	* Since it's likely they didn't want the same tag type as `C`, and wanted the smallest possible tag type
* What should the lints look like? (can be decided after stabilization if needed, but preferably this is hammered out before stabilization and after this RFC is accepted)
* <a id="ordered_fields_align"></a>Should `repr(ordered_fields, packed(N))` allow `align(M)` types where `M > N` (overaligned types).
	* discussion: https://github.com/rust-lang/rfcs/pull/3845#discussion_r2319098177
	* One option is to allow it and cap those fields to be aligned to `N`. This seems consistent with the handling of other over-aligned types. (i.e. putting a `u32` in a `repr(packed(2))` type)
* What should `repr(C)` do when a given type wouldn't compile in the corresponding `C` compiler (like fieldless structs in MSVC)? 
	* discussion: https://github.com/rust-lang/rfcs/pull/3845#discussion_r2319138105
# Future possibilities
[future-possibilities]: #future-possibilities

* Add more reprs for each target C compiler, for example `repr(C_gcc)` or  `repr(C_msvc)`, etc.
    * This would allow a single Rust app to target multiple compilers robustly, and would make it easier to specify `repr(C)`
    * This would also allow fixing code in older editions
* https://internals.rust-lang.org/t/consistent-ordering-of-struct-fileds-across-all-layout-compatible-generics/23247
