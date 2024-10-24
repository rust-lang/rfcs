- Feature Name: `layout_packed_aligned`
- Start Date: 2024-10-24
- RFC PR: [rust-lang/rfcs#3718](https://github.com/rust-lang/rfcs/pull/3718)
- Rust Issue: [rust-lang/rust#100743](https://github.com/rust-lang/rust/issues/100743)

# Summary
[summary]: #summary

This RFC makes it legal to have `#[repr(C)]` structs that are:
- Both packed and aligned.
- Packed, and transitively contains`#[repr(align)]` types.

It also introduces `#[repr(system)]` which is designed for interoperability with operating system APIs.
It has the same behavior as `#[repr(C)]` except on `*-pc-windows-gnu` targets where it uses the msvc layout
rules instead.

# Motivation
[motivation]: #motivation

This RFC enables the following struct definitions:

```rs
#[repr(C, packed(2), align(4))]
struct Foo { // Alignment = 4, Size = 8
    a: u8,   // Offset = 0
    b: u32,  // Offset = 2
}
```

This is commonly needed when Rust is being used to interop with existing C and C++ code bases, which may contain
unaligned types. For example in `clang` it is possible to create the following type definition, and there is
currently no easy way to create a matching Rust type:

```cpp
struct  __attribute__((packed, aligned(4))) MyStruct {
    uint8_t a;
    uint32_t b;
};
```

Currently `#[repr(packed(_))]` structs cannot transitively contain #[repr(align(_))] structs due to differing behavior between msvc and gcc/clang.
However, in most cases, the user would expect `#[repr(C)]` to produce a struct layout matching the same type as defined by the current target. 

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## `#[repr(C)]`
When `align` and `packed` attributes exist on the same type, or when `packed` structs transitively contains `align` types,
the resulting layout matches the current compilation target.

For example, given:
```c
#[repr(C, align(4))]
struct Foo(u8);
#[repr(C, packed(1))]
struct Bar(Foo);
```
`align_of::<Bar>()` would be 4 for `*-pc-windows-msvc` and 1 for everything else.


## `#[repr(system)]`
When `align` and `packed` attributes exist on the same type, or when `packed` structs transitively contains `align` types,
the resulting layout matches the current compilation target.

For example, given:
```c
#[repr(C, align(4))]
struct Foo(u8);
#[repr(C, packed(1))]
struct Bar(Foo);
```
`align_of::<Bar>()` would be 4 for `*-pc-windows-msvc` and `*-pc-windows-gnu`. It would be 1 for everything else.



# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

In the following paragraphs, "Decreasing M to N" means:
```
if M > N {
    M = n
}
```

"Increasing M to N" means:
```
if M < N {
    M = N
}
```


`#[repr(align(N))]` increases the base alignment of a type to be N.

`#[repr(packed(M))]` decreases the alignment of the struct fields to be M. Because the base alignment of the type
is defined as the maximum of the alignment for any fields, this also has the indirect result of decreasing the base
alignment of the type to be M.

When the align and packed modifiers are applied on the same type as `#[repr(align(N), packed(M))]`,
the alignment of the struct fields are decreased to be M. Then, the base alignment of the type is
increased to be N.

When a `#[repr(packed(M))]` struct transitively contains a field with `#[repr(align(N))]` type,
- The field is first `pad_to_align`. Then, the field is added to the struct with alignment decreased to M. The packing requirement overrides the alignment requirement. (GCC, `#[repr(Rust)]`, `#[repr(C)]` on gnu targets, `#[repr(system)]` on non-windows targets)
- The field is added to the struct with alignment increased to N. The alignment requirement overrides the packing requirement. (MSVC, `#[repr(C)]` on msvc targets, `#[repr(system)]` on windows targets)

# Drawbacks
[drawbacks]: #drawbacks

Historically the meaning of `#[repr(C)]` has been somewhat ambiguous. When someone puts `#[repr(C)]` on their struct, their intention could be one of three things:
1. Having a target-independent and stable representation of the data structure for storage or transmission.
2. FFI with C and C++ libraries compiled for the same target.
3. Interoperability with operating system APIs.

Today, `#[repr(C)]` is being used for all 3 scenarios because the user cannot create a `#[repr(C)]` struct with ambiguous layout between targets. However, this also means
that there exists some C layouts that cannot be specified using `#[repr(C)]`.

This RFC addresses use case 2 with `#[repr(C)]` and use case 3 with `#[repr(system)]`. For use case 1, people will have to seek alternative solutions such as `crABI` or
protobuf. However, it could be a footgun if people continue to use `#[repr(C)]` for use case 1.



# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This RFC clarifies that:
- `repr(C)` must interoperate with the C compiler for the target.
- `repr(system)` must interoperate with the operating system APIs for the target.
- Similiar to Clang, `repr(C)` does not guarantee consistent layout between targets.

Alternatively, we can also create syntax that allows the user to specify exactly which semantic to use when packed structs transitively contains aligned fields.
For example, a new attribute: #[repr(align_override_packed(N))] that can be used when the behavior of the child overriding the parent alignment is desired.

#[repr(align(N))] #[repr(packed)] can be used together to get the opposite behavior, parent/outer alignment wins.

Explicitly specifying the pack/align semantic has the drawback of complicating FFI. For example, you might need two different definition files depending on the target.

Therefore, a stable layout across compilation target should be relegated as future work.




# Prior art
[prior-art]: #prior-art

Clang matches the Windows ABI for `x86_64-pc-windows-msvc` and matches the GCC ABI for `x86_64-pc-windows-gnu`.

MinGW always uses the GCC ABI.

We already have both `C` and `system` [calling conventions](https://doc.rust-lang.org/beta/nomicon/ffi.html#foreign-calling-conventions)
to support differing behavior on `x86_windows` and `x86_64_windows`.


This issue was introduced in the [original implementation](https://github.com/rust-lang/rust/issues/33158) of `#[repr(packed(N))]` and have since underwent extensive community discussions:
- [#[repr(align(N))] fields not allowed in #[repr(packed(M>=N))] structs](https://github.com/rust-lang/rust/issues/100743)
- [repr(C) does not always match the current target's C toolchain (when that target is windows-msvc)](https://github.com/rust-lang/unsafe-code-guidelines/issues/521)
- [repr(C) is unsound on MSVC targets](https://github.com/rust-lang/rust/issues/81996)
- [E0587 error on packed and aligned structures from C](https://github.com/rust-lang/rust/issues/59154)
- [E0587 error on packed and aligned structures from C (bindgen)](https://github.com/rust-lang/rust-bindgen/issues/1538)
- [Support for both packed and aligned (in repr(C)](https://github.com/rust-lang/rust/issues/118018)
- [bindgen wanted features & bugfixes (Rust-for-Linux)](https://github.com/Rust-for-Linux/linux/issues/353)
- [packed type cannot transitively contain a #[repr(align)] type](https://github.com/rust-lang/rust-bindgen/issues/2179)
- [structure layout using __aligned__ attribute is incorrect](https://github.com/rust-lang/rust-bindgen/issues/867)


# Unresolved questions
[unresolved-questions]: #unresolved-questions

None for now.


# Future possibilities
[future-possibilities]: #future-possibilities

People intending for a stable struct layout consistent across targets would be directed to use `crABI`.
