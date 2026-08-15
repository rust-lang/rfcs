- Feature Name: `freeze_bytes`
- Start Date: 2024-02-13
- RFC PR: [rust-lang/rfcs#3605](https://github.com/rust-lang/rfcs/pull/3605)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)
- Feature Name: `freeze_uninit`

# Summary
[summary]: #summary

Defines a language primitive for freezing uninitialized bytes, and library functions that use this language primitive.

# Motivation
[motivation]: #motivation

In Rust, it is generally undefined behaviour to read uninitialized memory. Most types disallow uninitialized bytes as part of their validity invariant. 
While in most cases, this is not a problem, as a temporarily held (or externally checked) uninitialized value can be stored as `MaybeUninit<T>`, in some rare cases it can be useful or desireable to handle uninitialized data as a "Don't Care" value, while still doing typed operations, such as arithmetic. 
Freeze allows these limited Rust Programs to convert uninitialized data into useless-but-initialized bytes.

Examples of uses:
1. The major use for freeze is to read padding bytes of structs. This can be used for a [generic wrapper around standard atomic types](https://docs.rs/atomic/latest/atomic/struct.Atomic.html). 
2. SIMD Code using masks can load a large value by freezing the bytes, doing lanewise arithmetic operations, then doing a masked store of the initialized elements. With additional primitives not specified here, this can allow for efficient partial load operations which prevent logical operations from going out of bounds (such a primitive could be defined to yield uninit for the lane, which could then be frozen).
3. Low level libraries, such as software floating-point implementations, used to provide operations for compilers where uninit is considered a valid value for the provided operations.
    * Along the same lines, a possible fast floating-point operation set that yields uninit on invalid (such as NaN or Infinite) results, stored as `MaybeUninit`, then frozen upon return as `f32`/`f64`.
    * Note that such operations require compiler support, and these operations are *not* defined by this RFC.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Two new library interfaces are defined for unsafe code users:

```rust
// in module `core::ptr`
pub unsafe fn read_freeze<T>(ptr: *const T) -> T;

impl<T> *const T{
    pub unsafe fn read_freeze(self) -> T;
}
impl<T> *mut T{
    pub unsafe fn read_freeze(self) -> T;
}

// in module `core::mem`
impl<T> MaybeUninit<T>{
    pub fn freeze(self) -> Self;
}
```

`read_freeze` is an operation that loads a number of bytes from (potentially uninitialized) memory, replacing every byte in the source that is uninitialized with an arbitrary-but-initialized value.  
The function has similar safety constraints to `core::ptr::read`, in that the source pointer must be *dereferenceable* and well aligned, as well as pointing to a valid value of type `T` (after replacing uninitialized bytes).  
The same-name functions on the raw pointer types are identical to the free function version, and are provided for convience. 
Only the bytes of the return value are frozen. The bytes behind the pointer argument are unmodified.  
**The `read_freeze` operation does not freeze any padding bytes of `T` (if any are present), and those are set to uninit after the read as usual.**

`MaybeUninit::freeze` is a safe, by-value version of `read_freeze`. It takes in a `MaybeUninit` and yields an initialized value, which is either exactly `self` if it is already initialized, or some arbitrary value if it is not. 

The result of `core::ptr::read_freeze` or `MaybeUninit::freeze` is not guaranteed to be valid for `T` if `T` has a more complex validity invariant than an array of `u8` (for example, `char`, `bool`, or a reference type). 
However, the result is guaranteed to be valid for an integer or floating point type, or an aggregate (struct, union, tuple, or array) only containing (recursively) those types. 

Note that frozen bytes are arbitrary, not random. 
Rust code must not rely on frozen uninitialized bytes (or unfrozen uninitialized bytes) as a source of entropy, only as values that it does not care about and is fine with garbage-but-consistent computational results from. 

For example, the following function:
```rust
pub fn gen_random() -> i32{
    unsafe{MaybeUnit::uninit().freeze().assume_init()}
}
```

can be validily optimized to:
```rust
pub fn gen_random() -> i32{
    4 // Compiler chose a constant by fair dice roll
}
```

(See also [XKCD 221](https://xkcd.com/221/))

Note that the value `4` was chosen for expository purposes only, and the same optimization could be validly replace by any other constant, or not at all.

## Relationship to `read_volatile`

`read_volatile` and `read_freeze` are unrelated operations (except insofar as they both `read` from a memory location). 
`read_volatile` performs an observable side effect (that compilers aren't allowed to remove), but will otherwise act (mostly) the same as `read`. `read_volatile` does not freeze bytes read. 
In contrast, `read_freeze` is not a side effect (thus can be freely optimized by the compiler to anything equivalent).

It is possible in the future that `read_volatile` may carry a guarantee of freezing (non-padding) bytes, but this RFC does not provide that guarantee.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Uninit Bytes

Each byte in memory can either be initialized, or uninitialized. When a byte is uninitialized, it is not considered valid as part of any scalar type, the discriminant of an enum type, or pointer type (including reference types). 

Uninitialized memory is a distinct state from initialized memory, and the distinction is used by the defined Language Primitive and thus the library functions. 


## Language Primitive

The Guide-level explanation defines the public APIs for this RFC. To implement these public APIs, we define a new language primitive, in the form of an intrinsic function, to perform the operation defined.

For the purposes of the remainder of this RFC, it is chosen that the defined primitive is the `read_freeze` function, however it is believed that the choice is arbitrary and a valid implementation could choose to implement either as the compiler intrinsic. 
An example implementation of the `read_freeze` function using `MaybeUninit::freeze` as the language primitive is provided later.

```rust 
// in module core::intrinsics
pub unsafe extern "rust-intrinsic" fn read_freeze<T>(ptr: *const T) -> T;
```

(The above prototype and definition is for *exposition-only*, and is not intended to be exposed directly in a stable form)

The `read_freeze` intrinsic does a typed copy from `ptr` as `T`, but for every non-padding byte read from `ptr`, if the byte read is uninit, it is instead replaced by a non-determinstic initialized byte (the uninit byte is "frozen"). If the byte is init, then it is copied as-is.  
If `T` contains any pointers, we do not attach any valid provenance to bytes that are "frozen" (and thus, the entire pointer doesn't have any provenance when frozen from uninit bytes). Unsafe code cannot dereference such pointers or perform inbounds offsets (`core::ptr::offset`) on those pointers, except as otherwise considered valid for pointers with either no provenance or dangling provenance. 

Other than the validity property, `read_freeze` has the same preconditions as the `read_by_copy` operation (used to implement `core::ptr::read`), or a read from a place obtained by dereferencing `ptr`. 
In particular, it must be aligned for `T`, nonnull, dereferenceable for `T` bytes, and the bytes not be covered by a mutable reference that conflicts with `ptr`. 
The validity invariant of `T` is enforced after the bytes read by the intrinsic are frozen. 

The intrinsic is not proposed to be `const`, however there is not believed to be a fundamental reason why it could not be defined `const` in the future if there are sufficient use cases for that. This RFC leaves this to a separate stabilization step regardless.

Any initialized byte value chosen nondeterminstically may be chosen arbitrarily by the implementation (and, in particular, the implementation is permitted to deliberately pick a value that will violate the validity invariant of `T` or otherwise cause undefined behaviour, if such a choice is available). 


## Library Functions

### `core::ptr::read_freeze` 
```rust
// in module core::ptr
pub unsafe fn read_freeze<T>(ptr: *const T) -> T{
    core::intrinsics::read_freeze(ptr)
}
```

The `core::ptr::read_freeze` function directly invokes the `read_freeze` intrinsic and returns the result. 

The majority of the operation is detailed [above](#language-primitive), so the definition is not repeated here.

The `read_freeze` functions on `*const T` and `*mut T` are method versions of the free function and may be defined in the same manner.

### `MaybeUninit::<T>::freeze`

```rust 
// in module core::mem
impl<T> MaybeUninit<T>{
    pub fn freeze(self) -> Self{
        unsafe{core::ptr::read_freeze(core::ptr::addr_of!(self))}
    }
}
```

`MaybeUninit::freeze` is a safe version of the `read_freeze` function, defined in terms of the language intrinsic. It is safe because the value is kept as `MaybeUninit<T>`, which is trivially valid. 
To be used, Rust code would need to unsafely call `MaybeUninit::assume_init` and assert that the frozen bytes are valid for `T`.

The remaining behaviour of the function is equivalent to the `read_freeze` intrinsic.

### Example Alternative Implementation

If an implementation decides to define `MaybeUninit::freeze` (or equivalent) as the compiler intrinsic, it is possible to implement `core::ptr::read_freeze` as follows:
```rust 
pub unsafe fn read_freeze(ptr: *const T) -> T {
    ptr.cast::<MaybeUninit<T>>()
        .read()
        .freeze()
        .assume_init()
}
```


# Drawbacks
[drawbacks]: #drawbacks

The main drawbacks that have been identified so far:
* It is potentially [considered desireable](https://rust-lang.zulipchat.com/#narrow/stream/136281-t-opsem/topic/Arguments.20for.20freeze/near/377333420) to maintain the property that sound (e.g. correct) code cannot meaningfully read uninitialized memory
    * It is generally noted that safe/unsafe is not a security or privilege boundary, and it's fully possible to write unsound code (either deliberately or inadvertanly) that performs the read. If the use of uninitialized memory is within the threat model of a library that, for example, handles cryptographic secrets, that library should take additional steps to santize memory that contains those secrets.
    * Undefined behaviour does not prevent malicious code from accessing any memory it physically can.
    * That said, making such code UB could still be useful as it makes it unambiguously a bug to expose the contents of uninitialized memory to safe code, which can avoid accidental information leaks. If this RFC gets accepted, we should find ways to make it clear that even if doing so would be technically *sound*, this is still not something that Rust libraries are "supposed" to do and it must always be explicitly called out in the documentation.


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

* The intrinsic could be defined as a by-value operation used by `MaybeUninit::freeze`, instead of the read operation used by `core::ptr::read_freeze`
    * As noted above, it is believed that the two intrinsic choices are functionally identical, and thus the choice between them is completely arbitrary.
* Either one of the two functions could be provided on their own
    * Both functions are provided for maximum flexibility, and can be defined in terms of each other. The author does not believe there is significant drawback to providing both functions instead of just one
* An in-place, mutable `freeze` could be offered, e.g. `MaybeUninit::freeze(&mut self)`
    * While this function would seem to be a simple body that llvm could replace with a runtime no-op, in reality it is possible for [virtual memory](https://man7.org/linux/man-pages/man2/madvise.2.html#MADV_FREE) that has been freshly allocated and has not been written to exhibit properties of uninitialized memory (rather than simply being an abstract phenomenon the compiler tracks that disappears at runtime). Thus, such an operation would require a potentially expensive in-place copy. Until such time as an optimized version is available, we should avoid defining the in-place version, and require users to spell it explicitly as `*self = core::mem::replace(&mut self, uninit()).freeze()`.
    * If intermediate representations are cooperative, it may be beneficial to provide the operation in the future, as it could perform only the writes required to ensure the backing memory is in a stable state (such as 1 write every 4096 bytes)
    * Note that while `MaybeUninit::freeze(&mut self)` is possible to write, there is no `MaybeUninit::freeze(&self)`. This is because freezing uninit bytes requires performing writes in the abstract machine, overwriting uninitialized bytes with initialized ones, which are incompatible with the immutable `&Self` reciever.
* `MaybeUninit<Int>` (and maybe `MaybeUninit<Float>`) could have arithmetic that is defined as `uninit (op) x` or `x (op) uninit` is `uninit`.
    * While this can partially solve the second use case (by following it through `Simd`) and the third use case, this does not help the first use case
    * This RFC does not preclude these operations from being defined in the future, and may even make them more useful.
* `MaybeUninit::freeze` could instead be `pub unsafe fn freeze(self) -> T`, like `assume_init`.
    * While this would allow `mu.freeze().assume_init()` to be written in fewer lines of code, maintaining the value as `MaybeUninit` may make some code possible using careful offsetting and `MaybeUninit::as_ptr()`/`MaybeUninit::as_mut_ptr()`.
    * Most uses of `MaybeUninit` would require immediately using `.assume_init()` on the result, however, this is a potential footgun if `T` has any invalid initialized values, or is a user-defined type with a complex safety invariant. It is hoped that the extra verbosity, on top of helpful documentation, will allow intermediate, advanced, or expert unsafe code users making use of `MaybeUninit::freeze` to recognize this potential footgun and to carefully validate the types involved. 

# Prior art
[prior-art]: #prior-art

[LLVM](https://llvm.org/docs/LangRef.html#freeze-instruction) supports freeze by-value. 


* GCC may support an equivalent operation via the `SAVE_EXPR` GIMPLE code.
* See <https://rust-lang.zulipchat.com/#narrow/stream/136281-t-opsem/topic/GCC.20and.20freeze>


# Unresolved questions
[unresolved-questions]: #unresolved-questions

* Which of the library functions should recieve the direct language intrinsic, between `ptr::read_freeze` and `MaybeUninit::freeze`
* Should the `ptr::read_freeze` and `MaybeUninit::freeze` functions be `const`

# Future possibilities
[future-possibilities]: #future-possibilities

* With the project-portable-simd, the `Simd` type could support `Simd::load_partial<T, const N: usize>(x: *const T) -> Simd<[MaybeUninit<T>;N]>` (signature to be bikeshed) which could then be frozen lanewise into `Simd<[T;N]>`. With proper design (which is not the subject of this RFC), this could allow optimized loads at allocation boundaries by allowing operations that may physically perform an out-of-bounds read, but instead logically returns uninit for the out-of-bounds portion. This can be used to write an optimized implementation of `memchr`, `strchr`, or `strlen`, or even optimize `UTF-8` encoding and processing.
* `project-safe-transmute` could, in the future, offer some form of `MaybeUninit::assume_init_freeze` that statically checks for all-init-values being valid
    * Until such time as this becomes an option, this functionality could be provided by an external crate, such as the [bytemuck crate](https://lib.rs/bytemuck)
