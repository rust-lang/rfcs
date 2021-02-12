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

The struct [grammar][struct-grammar] is modified:

[struct-grammar]: https://doc.rust-lang.org/reference/items/structs.html

```diff
    | TupleStruct
 
 StructStruct :
-   struct IDENTIFIER  GenericParams? WhereClause? ( { StructFields? } | ; )
+   struct IDENTIFIER  GenericParams? WhereClause? ( { StructBody? } | ; )
 
 TupleStruct :
    struct IDENTIFIER  GenericParams? ( TupleFields? ) WhereClause? ;
 
-StructFields :
-   StructField (, StructField)* ,?
+StructBody :
+   StructBodyElement (, StructBodyElement)* ,?
+
+StructBodyElement :
+     StructField
+   | UnnamedStructElement
 
 StructField :
    OuterAttribute*
    Visibility?
    IDENTIFIER : Type
 
+UnnamedStructElement :
+   OuterAttribute*
+   Visibility?
+   `_` : Type
+
 TupleFields :
    TupleField (, TupleField)* ,?
```

UnnamedStructElements look syntactically like StructFields except that their
names must be `_` which cannot be the name of a StructField. Semantically, they
are not fields of the struct they are contained in.

UnnamedStructElements are used when determining the layout of the struct but are
otherwise ignored. Since they are not fields, they cannot be accessed, do not
appear in the construction of a struct, etc.

In the output of rustdoc, UnnamedStructElements are not distinguished from
StructFields except that their name is `_`.

The attribute `bitfield(N)` is added.  This attribute can only be applied to
StructBodyElements of `repr(C)` structs. Such a StructBodyElement is
called a *bit-field*. `N` must be an integer literal. `N` is called the *width*
of the bit-field. A bit-field must have one of the types

- `bool`,
- `u8`, `u16`, `u32`, `u64`, `u128`, `usize`,
- `i8`, `i16`, `i32`, `i64`, `i128`, `isize`

modulo type aliases.

`N` must be in the interval `[0, 8 * size_of::<T>()]` where `T` is the
type of the bit-field.

The attribute `bitfield(0)` can only be applied to UnnamedStructElements. An
UnnamedStructElement must be a bit-field.

Note that, despite being called a bit-*field*, a bit-field is not necessarily a
field. To disambiguate, when talking about bit-field fields, we explicitly call
them "bit-field fields".

Each field annotated with `bitfield(N)` occupies `N` bits of storage.

When reading and writing a bit-field field with type `bool`, `bool` is treated
like `uM` where `M = 8 * size_of::<bool>()` with `true` corresponding to `1` and
`false` corresponding to `0`.

When a field annotated with `bitfield(N)` is read, the value has the type
of the field and the behavior is as follows:

- The `N` bits of storage occupied by the bit-field are read.
- If the type of the field is signed (resp. unsigned), the bits are 1-extended
  (resp. 0-extended) to the size of the type of the field. (1-extended means
  that the new bits will have the value of the most significant bit. In
  particular, bit-fields with signed types with width 1 can only store the
  values `0` and `-1`.)
- The resulting bits are interpreted as a value of the type of the field. If the
  bits are not a valid object representation of the type, the behavior is
  undefined. This can only happen for bit-fields of type `bool`.

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
- For each bit-field field, the bits of the object representation used as its
  storage.

The language reference shall document for each target the layout of structs
containing bit-fields.

The intended behavior is that the layout is the same layout as produced by Clang
when compiling the corresponding C struct for the same target. The corresponding
C struct is constructed as follows:

- Translate the struct to the corresponding C struct as if the `bitfield`
  annotations were not present and as if `_` were a valid identifier for fields.
- For each StructBodyElement that has a `bitfield(N)` annotation in the Rust
  struct, append `: N` to the declaration in the C struct.
- For each field in the C struct whose name is `_`, delete the field name.

If the thus created C struct is not a valid C struct on the target, the layout
of the Rust struct is unspecified.

The `&` and `&mut` operators cannot be applied to bit-fields.

When a bit-field field is accessed, the abstract machine may also access
adjacent bit-field fields but not fields that are separated from the field by a
StructBodyElement that is not a bit-field field. (Note: This paragraph restricts
the kinds of loads and stores the compiler can perform when accessing a
bit-field. This paragraph does not need to be specially advertised to users as
the inability to take references to bit-field fields makes it impossible to
access adjacent bit-field fields in otherwise sound code.)

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
