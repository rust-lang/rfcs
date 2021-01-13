- Feature Name: bitfield
- Start Date: 2021-01-12
- RFC PR: [rust-lang/rfcs#3064](https://github.com/rust-lang/rfcs/pull/3064)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC adds support for bit-fields in `repr(C)` structs by

- introducing a new attribute `bitfield(N)` that can be applied to integer
  fields,
- allowing such annotated fields to be unnamed.

# Motivation
[motivation]: #motivation

The Linux kernel user-space API contains over 400 bit-fields. Writing the
corresponding types in Rust poses significant problems because the layout of
structs that contain bit-fields varies between architectures and is hard to
calculate manually. Consider the following examples which were compiled with
Clang for the `X-unknown-linux-gnu` target:

-   Effects of unnamed bit-fields on the alignment:

    ```c
    struct X {
        char a;
        int :3;
        char c;
    };
    ```

    | Arch | `sizeof(X)` | `alignof(X)` |
    | - | - | - |
    | x86_64 | 3 | 1 |
    | aarch64 | 4 | 4 |

-   Location of bit-fields in the memory layout:
    
    ```c
    struct X {
        char a;
        char B:3;
        char c:2;
        char d;
    };
    ```
    
    | Arch | Bits of the memory layout of `X` |
    | - | - |
    | x86_64 | `aaaaaaaaa __cccBBB dddddddd` |
    | mips64 | `aaaaaaaaa BBBccc__ dddddddd` |

-   Dependence of the correct Rust type on the layout of previous fields:

    ```c
    struct X {
        char a[N];
        int b:9;
        char c;
    };
    ```

    ```rust
    struct X {
        a: [c_char; N],
        b: ?,
        c: c_char,
    }
    ```

    On `x86_64`, for `N = 1`, the correct Rust type is `[u8; 2]`. For `N = 3`,
    the correct Rust type is `u16`. In either case,  the struct requires an
    additional `repr(align(4))` annotation


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This feature allows you to write C-compatible structs by applying the `bitfield`
attribute to fields:

```rust
#[repr(C)]
struct X {
    #[bitfield(1)] a: u8,
    #[bitfield(8)] b: u8,
    #[bitfield(0)] _: u8,
    #[bitfield(2)] d: u8,
}
```

corresponds to the C struct

```c
struct X {
    unsigned char a: 1;
    unsigned char b: 8;
    unsigned char  : 0;
    unsigned char d: 2;
};
```

As you can see in the example, unnamed bit-fields in C are written with the
reserved identifier `_` in Rust. When you have a struct with an unnamed
bit-field, you cannot access that field. On the other hand, you also do not have
to define a value for that field when constructing the struct:

```rust
let x = X { a: 1, b: 1, d: 1 };
let X { a, b, d } = x;
```

Just like in C, you cannot take the address of a bit-field:

```rust
fn f(x: &X) -> &u8 {
    &x.a // !!! does not compile
}
```

The value of the `N` in `bitfield(N)` must be a non-negative integer literal.
This literal must not be larger than the bit-size of the type of the field.

In debug builds, when you write to a bit-field, Rust performs overflow checks.
If the value to be stored does not fit in the bit-field, the overflow check
fails and the operation panics:

```rust
fn f(x: &mut X) {
    x.a = 6; // !!! overflow check fails
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The attribute `bitfield(N)` is added. `N` must be an integer literal. `N` is
called the *width* of the bit-field. This attribute can only be applied to fields
of a `repr(C)` struct. Such a field is called a *bit-field*. A bit-field must
have one of the types

- `bool`,
- `u8`, `u16`, `u32`, `u64`, `u128`, `usize`,
- `i8`, `i16`, `i32`, `i64`, `i128`, `isize`

modulo type aliases.

The width of a bit-field with type `T` must be in the range `[0,
bit_width_of(T)]`. For the integer types, `bit_width_of` is defined as `8 *
size_of::<T>()`. For `bool`, `bit_width_of` is 1.

A bit-field can have the name `_`. Such fields are only used to modify the layout
of the struct. Consider the example

```rust
#[repr(C)]
struct X {
    a: u32,
    #[bitfield(6)] _: u32,
    #[bitfield(1)] b: u32,
    #[bitfield(5)] _: u32,
    #[bitfield(1)] c: u32,
}
```

This struct behaves like

```rust
#[repr(C)]
#[insert_unnamed_bitfield_after(0, 6, u32)]
#[insert_unnamed_bitfield_after(1, 5, u32)]
struct X {
    a: u32,
    #[bitfield(1)] b: u32,
    #[bitfield(1)] c: u32,
}
```

if the attribute `insert_unnamed_bitfield_after` existed and caused the layout
to be modified in the same way as by the unnamed bit-field as described below.

In particular, such a field cannot be accessed, need not and cannot appear in
the construction of such a struct, cannot appear in the deconstruction of such a
struct.

Nevertheless, such fields appear like regular fields in the rustdoc output if
they have the `pub` visibility.

If the width of the bit-field is `0`, then the name of the field must be `_`.

A field named `_` is called an *unnamed* field. All other fields are called
*named* fields. Each named field annotated with `bitfield(N)` occupies `N` bits
of storage. Unnamed fields do not occupy any storage.

When reading and writing a bit-field with type `bool`, `bool` is treated like
`u1` with `true` corresponding to `1` and `false` corresponding to `0`.

When a field annotated with `bitfield(N)` is read, the value has the type
of the field and the behavior is as follows:

- The `N` bits of storage occupied by the bit-field are read.
- If the type of the field is signed (resp. unsigned), the bits are 1-extended
  (resp. 0-extended) to the size of the type of the field. (1-extended means
  that the new bits will have the value of the most significant bit. In
  particular, bit-fields with signed types with width 1 can only store the
  values `0` and `-1`.)
- The resulting bits are interpreted as a value of the type of the field.

When a field annotated with `bitfield(N)` is written, the value to be written
must have the type of the field and the behavior is as follows:

- If overflow checks are enabled and the value is outside the range of values
  that can be read from the field, the overflow check fails.
- The bitmask `(1 << N) - 1` is applied to the value and the remaining `N`
  significant bits are written to the storage of the bit-field.

If the overflow check is performed at compile time, the behavior is subject to
the `arithmetic_overflow` lint.

By the *layout* of a struct containing bit-fields, we mean the following
properties:

- The size and alignment of the struct.
- The offsets of all non-bit-field fields.
- For each named bit-field, the bits of the object representation used as its
  storage.

The language reference shall document for each target the layout of structs
containing bit-fields.

The intended behavior is that the layout is the same layout as produced by Clang
when compiling the corresponding C struct for the same target. The corresponding
C struct is constructed as follows:

- Translate the struct to the corresponding C struct as if the `bitfield`
  annotations were not present and as if `_` were a valid identifier for regular
  fields.
- For each field that has a `bitfield(N)` annotation in the Rust struct, append
  `: N` to the declaration in the C struct.
- For each field in the C struct whose name is `_`, delete the field name.

The `&` and `&mut` operators cannot be applied to bit-fields.

This implies that, in order to (mutably) access a bit-field, one must have a
(mutable) reference to the struct. Therefore, no concurrent accesses of the
struct are possible unless all accesses are immutable. Therefore, unlike in C,
the compiler can implement access to bit-fields with any kind of load/write
instruction, even if such an instruction overlaps the memory locations of other
fields in the struct.

# Drawbacks
[drawbacks]: #drawbacks

- Bit-fields cannot be assigned without overflow checks.
- The annotation feels somewhat far away from the type itself.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Alternative: Using `u32: N` to specify the width of a bit-field. I've chosen
  to use an annotation instead because it seems more likely to be accepted.

- Alternative: Using a dedicated type instead of an annotation:

    ```rust
    #[repr(C)]
    struct X {
        a: BitField<u32, 6>,
    }
    ```

    where `BitField` would be some kind of lang item that ensures that

    - the address of a field with such a type cannot be taken
    - the type and width parameters are correct.

    This type would then have the following API:

    ```rust
    impl<T, const N: usize> BitField<T, N> {
        fn new(t: T) -> Self;
        fn wrapping_new(t: T) -> Self;
        fn get(self) -> T;
        fn set(&mut self, t: T);
        fn wrapping_add(self, t: T) -> Self;
        ...
    }

    impl<T, const N: usize> AddAssign<T> for BitField<T, N> {
    }
    ```

    The layout of `BitField<T, N>` would be the layout of `T` when used outside
    a struct.

    This could then be used like so:

    ```rust
    fn f(x: X) {
        x.a.set(1);
        x.a += 1;
        x.a = x.a.wrapping_add(12);
        println!("{}", x.a.get());
    }
    ```

    There is some magic happening here:

    - `x.a += 1` cannot be written as `AddAssign::add(&mut x.a, 1)` because the
      address of `x.a` cannot be taken. The compiler would figure it out when
      the lhs is a struct field.
    - When the assignment happens, the compiler writes the bits on the rhs to
      the correct positions in the struct.

    This is a more structured solution but would need to be fleshed out.

# Prior art
[prior-art]: #prior-art

- C
- The author of Zig [proposes][zig-proposes] a different syntax that basically
  boils down to arbitrarily sized integer types:

  ```rust
  struct X {
      x: i9,
  }
  ```

  But this proposal is flawed because, in C, both the width and the underlying
  type influence the layout. The Zig proposal throws the underlying type away.

  ```c
  struct X {
      char a[3];
      int b:9;
  };

  struct Y {
      char a[3];
      long b:9;
  };
  ```

  These structs have different layouts on `x86_64-unknown-linux-gnu`.

[zig-proposes]: https://andrewkelley.me/post/a-better-way-to-implement-bit-fields.html


# Unresolved questions
[unresolved-questions]: #unresolved-questions

- On Windows, Clang and GCC produce different layouts for packed structs:

    ```c
    #include <stdio.h>
    
    struct __attribute__((packed, ms_struct)) X {
    	char a;
    	int b:1;
    	char c;
    };
    
    int main() {
    	printf("%ld\n", sizeof(struct X));
    }
    ```

    ```
    ~$ gcc test.c && ./a.out
    6
    ~$ clang test.c && ./a.out   
    12
    ```

    (`ms_struct` is the default on Windows.)

# Future possibilities
[future-possibilities]: #future-possibilities

- Adding some way to perform wrapping assignment.
- Relaxing the requirement that `N` must be a literal.
- Bit-fields with types that are enums with integer representation.
- Bit-fields with types that are transparent wrapper structs around valid
  bit-field types.
