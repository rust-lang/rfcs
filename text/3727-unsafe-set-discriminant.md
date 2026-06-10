- Feature Name: unsafe_set_discriminant
- Start Date: 2024-11-08
- RFC PR: [rust-lang/rfcs#3727](https://github.com/rust-lang/rfcs/pull/3727)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC proposes a way to write the discriminant of an enum when building it "from scratch". This introduces two new library components, an unsafe `set_discriminant` function, and a `discriminant_of!` macro, which can be used with any enum, regardless of `repr` or any other details.

# Motivation
[motivation]: #motivation

At the moment, enums are the only "plain old data" type that cannot be initialized in-place. This is discussed in [this blog post][pods_from_scratch] in more detail, and was [discussed on Zulip]. This RFC is aimed at removing that gap.

In-place construction is desirable for creating objects at their destination, often "by parts" (one piece or sub-component at a time) removing the need for potential on-stack construction, which can cause issues (particularly for large objects) with stack overflows, increased stack usage, or additional copies that cause performance impact.

In-place construction is useful for a number of niche applications, including in-place deserialization, construction of items on the heap, pinned initialization (including [in the Linux kernel]) or when using "outptr" patterns (for FFI or otherwise).

**Simple data types** (integers, booleans, etc.) can be created with `MaybeUninit::uninit` and initialized using pointer operations.

**Composite data types** (structs, tuples) can also be created with `MaybeUninit::uninit`, and `offset_of!` can be used to initialize each of the subcomponents.

With [this feature][feature_offset_of_enum] (currently proposed for FCP merge at the time of writing), it is possible to also use `offset_of!` to **initialize the variant** of an enum in-place, as with structs.

However, there is no stable way to set the discriminant of an enum in the general case, without fully creating the variant and writing it by-value to the destination. This means that any enum value must first be created on the stack, and written through to the final destination, even if the potentially large enum variant could be initialized in place.

For example, TODAY if we had the type `Option<[u8; 1024 * 1024]>`, and would like to initialize it as `Some([0u8; 1024 * 1024])`, this would require creation of the 1MiB array on the stack, before it is written to its destination. Although there are various workarounds to *hopefully* have the compiler elide this copy, they are not guaranteed.

With this RFC, it would be possible to zero-initialize the variant (meaning no 1MiB stack creation), and then set the discriminant to `Some`.

[in the Linux kernel]: https://docs.rs/pinned-init/latest/pinned_init/
[pods_from_scratch]: https://onevariable.com/blog/pods-from-scratch/
[discussed on Zulip]: https://rust-lang.zulipchat.com/#narrow/channel/213817-t-lang/topic/Prior.20discussions.20for.20manually.20building.20an.20enum.3F
[feature_offset_of_enum]: https://github.com/rust-lang/rust/issues/120141

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This introduces two new library components, a `discriminant_of!` macro, and an unsafe `set_discriminant` function.

## The `discriminant_of!` macro

For manual construction of an enum, it is necessary to obtain the discriminant of an enum, without necessarily having an instance of that enum.

This RFC proposes a macro that allows for creation of a [`Discriminant<T>`](https://doc.rust-lang.org/stable/std/mem/struct.Discriminant.html), similar to [`discriminant()`](https://doc.rust-lang.org/stable/std/mem/fn.discriminant.html), but without requiring a reference to the enum.

This would have the following form[^1]:

```rust
// macro: discriminant_of!(Type, Variant Name);

// Usable in normal let bindings
let discriminant: Discriminant<Option<u32>>
    = discriminant_of!(Option<u32>, Some);
//                     ^^^^^^^^^^^ --------> Type
//                                  ^^^^ --> Variant Name

// Also available in const context
const DISCRIMINANT: Discriminant<Result<u32, String>>
    = discriminant_of!(Result<u32, String>, Ok);
//                     ^^^^^^^^^^^^^^^^^^^ ------> Type
//                                          ^^ --> Variant Name
```

If the provided **Type** is not an enum, or the provided **Variant Name** is not present in the list of variants of **Type**, then a compile time error will occur.

```rust
discriminant_of!(u32, Some);       // Not allowed
discriminant_of!(Option<u32>, Ok); // Not allowed
```

Compiler errors could look like this:

```sh
discriminant_of!(u32, Some);
                 ^^^ -> ERROR: "u32" is not an enum

discriminant_of!(Option<u32>, Ok);
                              ^^ -> ERROR: type Option<u32> does not contain
                                    the variant "Ok". Variants are "Some" or
                                    "None".
```

[^1]: The syntax of [enum variant offsets][feature_offset_of_enum] is still in discussion, this RFC would adopt whatever syntax decisions made by that feature for consistency. The currently described syntax of that feature is used in this RFC as a placeholder.

## The `set_discriminant` function

This function is used for setting the discriminant of an enum, and has the following signature:

```rust
pub unsafe fn set_discriminant<T: ?Sized>(
    *mut T,             // The enum being constructed
    Discriminant<T>,    // The discriminant being set
); 
```

This function MUST be called AFTER fully initializing the values of the variant associated with this discriminant, and when called, MUST be called BEFORE use of the enum, for example calling `assume_init` on a `MaybeUninit<T>` or creation of a reference from a pointer to the uninitialized enum.

This could be used as follows:

```rust
let mut out: MaybeUninit<Option<[u32; 1024]>> = MaybeUninit::uninit();
// Make a decision on runtime values
let init_some: Option<u32> = get_user_input();

if let Some(val) = init_some {
    let opt_ptr: *mut Option<[u32; 1024]> = out.as_mut_ptr();
    // Tracking issue #120141 for enum variant offset_of! definition
    let val_offset: usize = offset_of!(Option<[u32; 1024]>, Some.0);
    let arr_ptr: *mut [u32; 1024] = opt_ptr.byte_add(val_offset).cast();
    let item_ptr: *mut u32 = arr_ptr.cast();
    // Initialize all items in the array to the user provided value
    for i in 0..1024 {
        // SAFETY: The variant body is always well aligned and valid for
        // the size of the type, uninit fields are only written.
        unsafe {
            item_ptr.add(i).write(val);
        }
    }
    // Obtain the discriminant
    let discrim = discriminant_of!(Option<[u32; 1024]>, Some);
    // Set the discriminant
    //
    // SAFETY: We have initialized all fields for this variant, and
    // this discriminant is correct for the type we are writing to.
    unsafe {
        set_discriminant(out.as_mut_ptr(), discrim);
    }
} else {
    // No value to write, just set the discriminant, leaving the
    // rest of the value uninitialized
    //
    // SAFETY: We have initialized all fields for this variant
    // (which is 'no fields'), and this discriminant is correct
    // for the type we are writing to.
    unsafe {
        // We can also use discriminant_of! without binding it
        set_discriminant(
            out.as_mut_ptr(),
            discriminant_of!(Option<[u32; 1024]>, None),
        );
    }
}

// This is now sound. We could also use `assume_init_ref` or
// `assume_init_mut` if we are explicitly avoiding potential
// copies by value, and the allocation is not local.
let out: Option<[u32; 1024]> = unsafe { out.assume_init() };
assert_eq!(out.is_some(), init_some.is_some());
if let Some(val) = init_some {
    assert!(out.as_ref().unwrap().iter().all(|x| *x == val));
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## The `discriminant_of!` macro

This macro is the "valueless" version of `std::mem::discriminant`. It needs to be a macro, as we cannot otherwise name the variant.

Unlike the `discriminant` function, it is possible at compile time to detect if this is called on a type that is not an `enum`, meaning that it is possible for it to be a compiler error in this case, rather than "unspecified but not UB" to use this with a non-enum type.

This macro returns by value the same opaque `Discriminant<T>` used by `std::mem::discriminant`.

This RFC does not propose allowing obtaining the discriminant of "nested" fields, and will only work with a "top level" item, but does not preclude adding this capability in the future. This addition could be specified in a later RFC. For now users are recommended to handle "one level at a time".

```rust
discriminant_of!(Option<Option<u32>>, Some);        // Allowed
discriminant_of!(Option<Option<u32>>, Some.0.Some); // Not Allowed
```

## The `set_discriminant` function

This function must be unsafe, as setting an incorrect discriminant could lead to undefined behavior by allowing safe code to observe incorrectly or incompletely initialized values.

This function takes an `*mut T` and has multiple requirements necessary for safety:

* The pointer `*mut T` must be non-null
* The enum `T` and the selected variant's fields must be well-aligned for reading and writing
* This function is allowed to write and read-back both the discriminant and body (whether they each exist or not, and whether the discriminant and body are separate or not). This function also may do this in an unsynchronized manner (not requiring locks or atomic operations), which means exclusive access is required

If the `T` used for `*mut T` or `Discriminant<T>` when calling this function is NOT an enum, then this function is specified as a no-op. This is not undefined behavior, but later calls to `assume_init` or usage of the value `T` may be undefined behavior if the `T` was not properly initialized. This is also allowed (but not required) to cause a debug assertion failure to aid in testing (implementor's choice).

When this function is called, it MUST be called AFTER fully initializing the variant of the enum completely, as in some cases it may be necessary to read back these values. This is discussed more in the next section.

Semantically, `set_discriminant` is specified to optionally write the discriminant (when necessary), and read-back the discriminant. If the read-back discriminant does not match the expected value, then the behavior is undefined.

### Alternate forms of `set_discriminant`

This RFC only specifies one form of `set_discriminant`, which takes an `*mut T` to the enum `T`. It is expected that additional versions of this function will also be added for convenience in the future, for example taking other pointer types like:

* `unsafe fn set_discriminant_mu(&mut MaybeUninit<T>, Discriminant<T>);`
* `unsafe fn set_discriminant_nn(NonNull<T>, Discriminant<T>);`

These forms are not specified as part of this RFC, and can be added without an additional RFC in the future. Alternate forms are expected to match the semantics of the pointer-based version of `set_discriminant`.

## Interactions with niche-packed types

This RFC is intended to work with ANY enum types, including those with niche representations. This includes types like `Option<&u32>` or `Option<NonZeroUsize>`, where excluded values (like `null`) are used for the `None` variant.

In these cases, the discriminant and variant body are fully overlapping, rather than being independent memory locations.

This RFC recommends users to explicitly call `set_discriminant`, even if the act of setting the value also sets the discriminant implicitly, for example by writing `null` to the body of `Option<&u32>` via pointer methods or casting.

```rust
let refr: &u32 = &123u32;
let mut out: MaybeUninit<Option<&u32>> = MaybeUninit::uninit();

let opt_ptr: *mut Option<&u32> = out.as_mut_ptr();
// Tracking issue #120141 for enum variant offset_of! definition
let val_offset: usize = offset_of!(Option<&u32>, Some.0);
let val_ptr: *mut &u32 = opt_ptr.byte_add(val_offset).cast();

unsafe {
    // Sets the value of the body
    val_ptr.write(refr);
    // Does not affect the contents of `out`
    set_discriminant(
        out.as_mut_ptr(),
        discriminant_of!(Option<&u32>, Some),
    );
}
let out: Option<&u32> = unsafe { out.assume_init() };
assert_eq!(out, Some(refr));
```

In the case of the niche variant, `set_discriminant` would be responsible for also initializing the whole body of the variant. For example, this would be sufficient initialization:

```rust
let mut out: MaybeUninit<Option<&u32>> = MaybeUninit::uninit();
unsafe {
    set_discriminant(
        out.as_mut_ptr(),
        discriminant_of!(Option<&u32>, None),
    );
}
let out: Option<&u32> = unsafe { out.assume_init() };
assert_eq!(out, None);
```

## Specifically known types vs unknown types

This RFC does not invalidate any currently-accepted ways of initializing enums manually without these new functions. For example, enums with a [primitive representation], or niche represented enums with [discriminant elision], can be soundly created today.

When specific types with these qualities are used, it is not required, but still allowed, to use the `set_discriminant` function to set the discriminant.

However, when authoring generic or macro code, which may potentially accept types that do not have these qualities, it is necessary to call `set_discriminant` to fully initialize the enum in the general case.

For example, if a macro was used to populate an `Option<U>` value, and users could chose `U: u32` (which does not have a niche repr), OR `U: &u32` (which does have a niche repr), then the author of the macro should call `set_discriminant` to soundly initialize the `Option<U>` in all cases.

[primitive representation]: https://doc.rust-lang.org/reference/items/enumerations.html#pointer-casting
[discriminant elision]: https://rust-lang.github.io/unsafe-code-guidelines/layout/enums.html#discriminant-elision-on-option-like-enums

# Drawbacks
[drawbacks]: #drawbacks

* This RFC increases the API surface of the standard library
* This RFC adds unsafe methods which must be considered, documented, and tested
* The abilities of this RFC could technically already be done today, either for carefully selected subsets of enums (specific reprs, hand-built enums using explicit discriminant and `union`'d value fields), or be done today less efficiently with explicit value creations and copies
* The features added to this RFC generally only benefit "power users", likely library authors already doing unsafe things

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This RFC is necessary to "complete" the ability to soundly create enums in-place.

Without this capability, downstream library authors are required to make a choice between:

* Suboptimal stack usage and performance, in the case of by-value initialization
* Restriction of functionality to a subset of enum reprs, or bespoke proxy types that emulate enums to allow for in-place construction

This is necessary to be part of the language and/or standard library as enum representations (at least for the default `repr(Rust)`) are unspecified, and particularly with concepts like niche-packed enums, it is not possible to soundly handle this in the general case of user defined enum types.

I am not aware of any other proposed designs for this functionality.

# Prior art
[prior-art]: #prior-art

The `rkyv` crate defines [proxy enums] with explict `#[repr(N)]` (where N is an unsigned integer) types, in order to allow in-place construction and access

[proxy enums]: https://rkyv.org/format.html

Syntax for `discriminant_of!` is inspired by the in-progress [offset_of_enum feature][feature_offset_of_enum].

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## Should we provide alternate forms of `set_discriminant`?

There was some discussion when writing this RFC what the type of the first argument to `set_discriminant` should be:

* `&mut MaybeUninit<T>`
* `NonNull<T>`
* `*mut T`

`*mut T` was chosen as the most general option, however it is likely desirable to accept other forms as well for convenience.

# Future possibilities
[future-possibilities]: #future-possibilities

## Should `discriminant_of!` support nested usage?

Discussed above, we said this was not allowed:

```rust
discriminant_of!(Option<Option<u32>>, Some);        // Allowed
discriminant_of!(Option<Option<u32>>, Some.0.Some); // Not Allowed
```

Should we allow this in the future?

## Should we support construction of unaligned enums?

If the user is writing to a packed format, they could potentially want the ability to set the discriminant in cases where the enum discriminant or fields are not well-aligned.

This could require the creation of a `set_discriminant_unaligned` function, that relaxes the well-aligned safety requirements of the proposed `set_discriminant`.

There is also currently no way to read the discriminant of an unaligned enum, so it may also be necessary to add unaligned versions of the `discriminant()` function as well.
