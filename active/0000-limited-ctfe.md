- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Implement compile time evaluation of a limited set of compiler-internal
functions:

- `size_of`
- `min_align_of`
- `pref_align_of`

# Motivation

Today it is not possible to use the size of a type in a constant expression.

C11 defines two "functions" that can be used in constant expressions:

- `sizeof`
- `_Alignof`

For example, glibc defines the following struct
```c
# define _SIGSET_NWORDS	(1024 / (8 * sizeof (unsigned long int)))
typedef struct
  {
    unsigned long int __val[_SIGSET_NWORDS];
  } __sigset_t;
```
which cannot be used in rust FFI without knowing the size of `long` at compile
time. This size could be defined in `liblibc`, but this is error prone an
stops working when the argument of `sizeof` is an aggregate type.

Another use case is the following type which could be used to store types whose
destructor should not run in all cases. (Note that there are other problems
with this implementation that might make this impossible.)

```rust
#[repr(C)]
struct ManuallyDrop<T> {
    data: [u8, ..size_of::<T>()],
    _ty: [T, ..0],
}
```

This could then be used in a `SmallVec` implementation:
```rust
struct SmallVec<T> {
    // Used when there are more than 5 elements
    ptr: *mut T,
    len: uint,
    cap: uint,
    small: [ManuallyDrop<T>, ..5],
}
```

Even better: You could use this to create a poor man's `SmallVec<N, T>`:
```rust
struct SmallVec<Size, T> {
    // ...
    small: [ManuallyDrop<T>, ..size_of::<Size>()],
}
// seven elements on the stack
let x: SmallVec<[u8, ..7], int> = SmallVec::new();
```

# Detailed design

Consider a constant expression which contains an expression `expr` of the form
```rust
PATH::<T>()
```
where `PATH` resolves to one of the following functions
```rust
extern "rust-intrinsic" {
    fn size_of<T>() -> uint;
    fn min_align_of<T>() -> uint;
    fn pref_align_of<T>() -> uint;
}
```
and where `T` is a `Sized` type or type parameter.

1. For each expression `expr` where `T` is not a type parameter, the compiler
   evaluates `expr` as if it had been evaluated at runtime and then behaves as
   if `expr` had been replaced by the fully qualified result of the
   computation.

1. If at least one of the `expr` contains a `T` which is a type parameter, the
   compiler behaves as if it were implemented in the following way:

   * Type checking: Check if the constant expression were valid if all `expr`
     were replaced by `0u`.
   * Store the constant expression in some implementation defined way such
     that the process in 1. can be applied after monomorphization.

# Examples

The following examples contain two marked lines of code each and the compiler
treats the second line as if it had been replaced by the first one in the
original source code.

### Example 1

```rust
static I8_SIZE: uint = 1u; // After CTFE
static I8_SIZE: uint = core::intrinsics::size_of::<i8>(); // Before CTFE
```

### Example 2

```rust
use core::intrinsics::{size_of};

static I8_SIZE: uint = 1u; // After CTFE
static I8_SIZE: uint = size_of::<i8>(); // Before CTFE
```

### Example 3

```rust
#![no_std]
extern "rust-intrinsic" {
    fn size_of<T>() -> uint;
}

static I8_SIZE: uint = 1u; // After CTFE
static I8_SIZE: uint = size_of::<i8>(); // Before CTFE
```

### Example 4

#### Original Code

```rust
#[repr(C)]
struct ManuallyDrop<T> {
    data: [u8, ..size_of::<T>()],
    _ty: [T, ..0],
}
```

#### After compilation

```rust
#[repr(C)]
struct ManuallyDrop<T> {
    data: [u8, ..ConstExpr<T>], // Will be evaluated after monomorphization..
    _ty: [T, ..0],
}
```

#### After monomorphization

```rust
#[repr(C)]
struct ManuallyDrop<u64> {
    data: [u8, ..8u],
    _ty: [u64, ..0],
}
```

### Example 5

Compile time error.

```rust
fn f<T>() {
    static i: int = size_of::<T>();
}
```
```
mismatched types: expected `int`, found `uint` (expected int, found uint)
    <anon>:2     static i: int = size_of::<T>();
                                 ^~~~~~~~~~~~~~
```

# Drawbacks

It is unlikely that this can be replaced by CTFE in a backwards compatible way
because `size_of` and friends are extern functions and calling them normally
requires an unsafe block. Unfortunately you cannot instead check if `PATH` in
the detailed description above resolves to `core::mem::size_of` because some
programs might wish to avoid even `libcore`.

Thus, this feature should live behind a feature gate.

# Alternatives

- Add new keywords `sizeof` and `alignof` to the language that are evaluated at
compile time.
- Add new language items `#[lang(sizeof)]` and `#[lang(alignof)]` that can be
applied to functions. When these functions are used in constant expressions,
instead of evaluating the functions themselves, some compiler internal function
is executed and the compiler behaves as described in the description above.
These language items would then be applied to `core::mem::{size_of, align_of}`.
If someone wishes to not use `libcore`, they can apply them to two dummy
functions.

# Unresolved questions

- These functions are still not as powerful as `sizeof` and `_Alignof` which can
take arbitrary expressions as arguments, e.g., `sizeof(1L) == sizeof(long)`.
Rust also has a function that can do this: `size_of_val<T>(val: T)` which then
calls `size_of::<T>()`. Unfortunately `size_of_val` is a real function and not
a `extern "rust-intrinsic"`. Thus you cannot use it in programs that wish to
avoid `libcore`. How could this be done? Note that this would not be a problem with
the two alternatives above.
- There are many more intrinsic (math) functions which could be evaluated at
compile time. Should these functions be allowed in constant expressions? Note that
this is not possible with the two alternatives above.
