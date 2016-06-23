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
do not support `usize` == `uptr`.

I would assert that we want to support usecases where `uptr != usize`. The
embedded space, where segmented architectures are still very popular, is a very
good case for separating `uptr` and `usize`.

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
`isize` as the upper bound. This is like the `RSIZE_MAX` bound in the C11
standard.

2) Issue a breaking-change report, and start a warning for any integer to
pointer cast or transmute which is from a type which isn't `uptr` or `iptr`. Do
not error: it's still valid, but bad style, and, as it states in section 5,
implementation defined.

3) Add a new #[cfg] constant to the language: `target_size_bits`, which would
give the size of `usize` and `isize` in bits.

4) Add two new primitive integer types: `iptr` and `uptr`.

5) Casting an integer to a pointer results in an implementation defined value;
casting a pointer to an integer results in an implementation defined value.
These are already the rules, this is just making these rules explicit.

6) However, as an exception to the above rule: then casting a pointer type to
one of `iptr` or `uptr`, then casting back to the original pointer type from the
resulting value, will result in a pointer which compares equal to the original
pointer.

7) As a second exception to the above rule: casting a `0` of any integer type to
any pointer type shall result in a null pointer of that pointer type.

8) Casting the same pointer to `iptr` shall always result in the same value.

9) Casting the same pointer to `uptr` shall always result in the same value.

Sources:

[CHERI](https://www.cl.cam.ac.uk/research/security/ctsrd/cheri/cheri-faq.html)

# How do we teach this?

Currently, what we do is we teach `isize` and `usize` as the "pointer-sized
integer type[s]". 

The book teaches them as follows:

> Rust also provides types whose size depends on the size of a pointer of the
> underlying machine. These types have ‘size’ as the category, and come in
> signed and unsigned varieties. This makes for two types: isize and usize.

As far as I can tell, after reading this, I still have no idea what an `isize`
or a `usize` is. The new documentation would look like this:

> Rust also provides types whose size depends on the underlying machine. `usize`
> and `isize` are used to represent the size of things, for example, how many
> elements an array has. `uptr` and `iptr` are meant for use with pointers.

And further on, in "FFI"

```rust
#[link(name = "snappy")]
extern {
  fn snappy_max_compressed_length(source_length: size_t) -> size_t;
}
```

becomes

```rust
#[link(name = "snappy")]
extern {
  fn snappy_max_compressed_length(source_length: usize) -> usize;
}
```

with appropriate explanation:

> rustc guarantees that `usize` is the same as C's `size_t`.

And hopefully, we would add an example of a function taking `intptr_t` or
`uintptr_t`, which would become `iptr` or `uptr`.

# Drawbacks
[drawbacks]: #drawbacks

More complexity.

# Alternatives
[alternatives]: #alternatives

Continue doing what we're doing.

Define `isize` as the upper bound always.

# Unresolved questions
[unresolved]: #unresolved-questions

What do we call `target_size_bits`?

How do we then allow hashing of pointers on these segmented platforms? C and C++
don't guarantee that pointers converted to integers will have the same value
each time they're converted (for example, for segmented architectures). We could
still have this guarantee, but it would make things expensive to convert, as
we'd have to do a normalization each time we converted.
