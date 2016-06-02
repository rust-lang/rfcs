- Feature Name: `uptr_iptr`
- Start Date: 2016-06-02
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

`isize` and `usize` either must be a `size_t` like type, or `intptr_t` like
type. Most uses of `usize` and `isize` are as `size_t` or `ptrdiff_t`
like-types (`Vec<T>`, `[T]`, `.offset`, etc.). Therefore, remove the guarantee
that `usize` and `isize` are pointer-size, and make them exclusively
object-sized.

# Motivation
[motivation]: #motivation

We want to support the embedded space, and new CPUs like the CHERI. These CPUs
do not support `usize` == `uptr`, and, in the case of the CHERI, don't support
`uptr` at all. Most CPUs don't actually support the idea that `uptr == usize`:
just the currently popular ones.

We really want to support usecases where `uptr != usize`. The embedded space,
where segmented architectures are still very popular, is a very good case for
separating `uptr` and `usize`.

This also means that there's no more confusion when writing FFI code, where
`usize` and `size_t` aren't the same, but neither are `usize` and `uintptr_t`,
really. Creating this function:

```rust
extern fn takes_slice_from_c(ptr: *const i32, len: usize) {
  let slice = std::slice::from_raw_parts(ptr, len);
}
```

How do you convert this to C?

```C
void takes_slice_from_c(int32_t const* ptr, size_t len); // ?
```

or

```C
void takes_slice_from_c(int32_t const* ptr, uintptr_t len); // ?
```

One gives the right semantics, but is wrong by the standard; the other gives the
wrong semantics (you're not converting `len` to a pointer), but is more correct
by the standard (although it's still not fully correct: there's nothing in the C
standard that says that `uintptr_t` is the same size as a pointer, unlike in the
Rust reference). You could always take a `libc::size_t`, but Rust programmers
often don't seem to do that: they like their Rust types, and we don't even warn
anymore on using usize as an external type. Even `libc::size_t` is just 
`pub type size_t = usize`.

`usize` currently serves a dual purpose. When you use `usize`, you could be
using it as an object size, or you could be using it as a pointer integer, (or
you could be using it in another, unforseen way). Having an explicit `uptr` type
allows us to semantically differentiate these two purposes.

# Detailed design
[design]: #detailed-design

1) Remove the language from the reference (and any other official text)
referring to `usize` and `isize` being the same size as, or convertible to,
pointer types. This would mean that `usize` is instead the maximum size of an
object, like `size_t` in C, and that `isize` is the difference type for
pointers, like `ptrdiff_t`.  Note that this also changes `usize` to be the upper
bound of objects: implementations (like rustc) may continue to choose to use
`isize` as the upper bound, because of buggy code in the backend.

2) Issue a breaking-change report, and start a warning for any integer to
pointer cast or transmute which is from a type which isn't `uptr` or `iptr`. Do
not error: it's still valid, but bad style, and, as it states in section 5,
implementation defined.

3) Add a new #[cfg] constant to the language: `target_size_bits`, which would
give the size of `usize` and `isize` in bits.

4) Add two new primitive integer types: `iptr` and `uptr`. They would not be
defined on platforms like the CHERI where going from pointer -> int -> pointer
is not fully supported.

5) Casting an integer to a pointer results in an implementation defined value;
casting a pointer to an integer results in an implementation defined value.
These are already the rules, this is just making these rules explicit.

6) However, as an exception to the above rule: if `iptr` and `uptr` are defined,
then casting a pointer type to one of the two, then casting back to the original
pointer type from the resulting value, will result in a pointer which is
equivalent to the original pointer; i.e., as if it were a copy of the original
pointer. Casting the integer to the other mutability of the same pointer type,
i.e., `*const T -> uptr -> *mut T`, it shall be equivalent to `*const T -> *mut
T`. Casting the integer to a different type from the original, i.e. `*const T ->
uptr -> *const U`, shall result in implementation defined behavior (as in
section 5).

7) As a second exception to the above rule: casting a literal `0` of any integer
type to any pointer type shall result in a null pointer of that pointer type. This
must be done in a single cast from a literal to a pointer: `0 as *const i32` would
result in a null pointer; `let x = 0; x as *const i32` does not necessarily. This
also is an exception to 2; `0 as [pointer type]` would not warn.

Sources:

[CHERI](https://www.cl.cam.ac.uk/research/security/ctsrd/cheri/cheri-faq.html)

[Backend Bugs](http://trust-in-soft.com/objects-larger-than-ptrdiff_max-bytes/)

# Drawbacks
[drawbacks]: #drawbacks

More complexity.

# Alternatives
[alternatives]: #alternatives

Continue doing what we're doing.

Define `isize` as the upper bound always.

Make some guarantees about pointers which don't compare equal resulting in
`uptr`/`iptr` that don't compare equal, some guarantees around pointers
which are inside the same object, and a guarantee that if you convert a pointer
to a uptr and back to the original pointer type, it will compare equal. We would
then define it everywhere, and keep the language about casts back having
implementation defined behavior. This would allow us to write a CHERI Rust
compiler, for example, but also allow us to write a Hash for pointers that works
everywhere, and a memmove that works everywhere.

# Unresolved questions
[unresolved]: #unresolved-questions

What do we call `target_size_bits`?

How do we then allow hashing of pointers on these segmented platforms? C and C++
don't guarantee that pointers converted to integers will have the same value
each time they're converted (for example, for segmented architectures). We could
still have this guarantee, but it would make things expensive to convert, as
we'd have to do a normalization each time we converted.
