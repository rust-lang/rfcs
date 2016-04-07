- Feature Name: N/A
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Giving Rust a memory model. This allows us to understand what exactly is
undefined behavior, and what is not, in unsafe code.

# Motivation
[motivation]: #motivation

To allow unsafe code to be written without worrying about whether the compiler
will miscompile your code.  Our current system is ill-defined, and far too
cautious (and in other cases, completely undefined; for example, what are the
semantics of raw pointer aliasing?).

# Detailed design
[design]: #detailed-design

This is the complicated part :)

## Using a value

Anything which touches a value, and is not either a move or copy from the value,
move or copy into an lvalue, or taking a reference to the value, is a use of the
value. Examples of uses are: arithmetic, match, indexing. Examples of things
which are not uses are: returning, passing a value to a function, `let x = y`,
and `let x = &y`. A reborrow is not a use.

## Type Representation

Each type `T` shall be equivalent to a byte array: `[u8; size_of::<T>()]`. Each
byte in this byte array shall be in one of three states: "Defined", "Undefined",
or "Implementation Defined". "Defined" bytes are in a defined state at all
times; they do not depend on which compiler, nor which platform one is on.
"Undefined" bytes are also easy to understand; they do not have a defined value
ever; examples are `std::mem::uninitialized()`, and padding bytes.
"Implementation Defined" bytes are a little more difficult to understand; these
are either "Defined" or "Undefined" depending on implementation details, like
layout of structs. An "Implementation Defined" byte is only legal to read as a
member of the original type it was a part of, in the original place it was in,
or through reading fields of the original type.

A value of type `T` can be in one of two states: "Valid", or "Invalid". Using an
"Invalid" value is Undefined Behavior. Note that the definition of "Valid" and
"Invalid" do not mean that a given value is correct for all uses; only that it
is representationally valid. One good example of this is references; just
because they are "Valid" does not mean that you can dereference them, only that
they are not null.

### Invalid and Valid Values

All integer, floating point, and raw pointer to Sized values will be "Valid" if
each byte is "Defined".

--

Reference to Sized values will be "Valid" if each byte is "Defined", and are
not equal to the null pointer.

--

Function pointer values will be "Valid" if each byte is "Defined", and are not
equal to zero. Note that the size of function pointers is implementation
defined, and not guaranteed equal to `*const ()`.

--

`bool` values will be "Valid" if the byte is "Defined", and equals either `0x1`,
or `0x0`.

--

`char` values will be "Valid" if each byte is "Defined", and, if read as a
`u32`, would be within the range `[0x0,0xD7FF]∪[0xE000,0x10FFFF]`

--

Struct (including tuple) values will be "Valid" if each field is "Valid".

Each field shall be an offset into the byte array which makes up the value of
the struct.

A rust representation struct will be made of "Implementation Defined" bytes; a C
representation struct will be made of whatever the inner types are made of, in
order, and "Undefined" bytes for the padding; a packed struct shall be made of
whatever the inner types are made of, in order, with no padding.

--

Enum values will be "Valid" if the Discriminant is "Valid", and is one of the
valid discrimants for the enum, and the discriminated value is also "Valid".

Each discriminated value shall be at an offset into the byte array which makes
up the value of the enum.

An enum without associated values shall be equivalent to the discriminant, and
shall have all "Defined" bytes. If the enum has associated values, all bytes
shall be "Implementation Defined".

--

Union values will be "Valid" if the initialized field is "Valid".

Each field shall be at an offset into the byte array which makes up the value of
the union.

A rust representation union will be made of "Implementation Defined" bytes; a C
representation union will follow the C ABI of the platform, using inner bytes
for where the inner types should go, and "Undefined" bytes where padding should
go.

--

Pointer to !Sized values will be "Valid" if the pointer part of the !Sized
pointer is "Valid", and the metadata part is "Valid"

Each byte in a pointer to !Sized value shall be "Implementation Defined".

## Pointer Rules

These are only valid for Sized pointers; !Sized pointers will work the same way
except that only the pointer part of the !Sized pointer is used.

`ptr::read` and `ptr::write` will be the basis of the Rust pointer rules. They
are both defined as a use.

To `ptr::read` or `ptr::write` a value of type `T` from or to a raw pointer, the
alignment of the raw pointer must be greater than or equal to `align_of::<T>()`

To `ptr::read` a value of type `T` from a raw pointer, there must be
`size_of::<T>()` bytes of storage readable behind the raw pointer

To `ptr::write` a value of type `T` to a raw pointer, there must be
`size_of::<T>()` bytes of storage writeable behind the raw pointer

### Pointer Write Aliasing

If a pointer refers to a value, then an aliased pointer is one where there is
overlap in the referred to byte arrays; for example:

```rust
{
  let x = [u32; 5];
  let ref1 = &x[0..3];
  let ref2 = &x[2..4];
  // ref1 and ref2 are aliased
}
```

A derived pointer shall be an in bounds pointer, calculated as a defined offset
from another pointer value. Derived pointers shall be a tree; each derived
pointer D derived from pointer D' shall also be derived from the pointers that
D' is derived from.

```rust
// one common way to do this is with a reborrow
{
  let mut x = 0;
  let ref1 = &mut x;
  let ref2 = &mut *ref1;
}

// another is with field access
{
  let mut x = (0, 1);
  let ref1 = &mut x;
  let ref2 = &mut ref1.1;
}

// this is a tree
{
  let mut x = (0, (1, 2));
  let ref1 = &mut x;
  let ref2 = &mut ref1.1; // this is a derived pointer of ref1
  let ref3 = &mut ref2.1; // this is a derived pointer of both ref1 and ref2
}
```

A `ptr::read` or `ptr::write` of a reference makes any pointer derived from that
reference non-derived.

```rust
{
  let mut x = 0;
  let ref1 = &mut x;
  let ptr = ref1 as *mut i32; // ptr is now a "derived pointer"
  ptr::write(ptr, 1); // fine, ptr is a derived pointer of ref1
  ptr::read(ref1) // okay, ptr is no longer "derived", so don't touch it from
                  // this point on
}
```

Any move of a reference is a rederivation.

```rust
// UB if ref_ aliases ptr
fn foo(ref_: &mut i32, ptr: *mut i32) -> i32 {
  *ref_ = 0;
  *ptr = 1;
  *ref_
}

{
  let mut x = 0;
  let ref_ = &mut x;
  let ptr = ref_ as *mut i32;
  foo(ref_, ptr) // Undefined Behavior!!! ref_ is reborrowed with this move, so
                 // the reference inside the function call doesn't see ptr as
                 // derived
}
```

To `ptr::read` or `ptr::write` a value of type `T` from or to a reference, in
addition to following the rules of the raw pointer `ptr::read` or `ptr::write`:
from the time of the creation of the reference, to when the reference goes out
of scope, there shall be no aliasing `ptr::write` of any pointer which is not
derived from that reference. Additionally, there shall be no `ptr::read` of an
aliased pointer in the case of a `ptr::write`.

```rust
// the following is defined, as neither ref2 nor ref1 are written through
{
  let mut x: i32 = 0;
  let ref1: &mut i32 = unsafe { &mut *(&mut x as *mut i32) };
  let ref2: &mut i32 = &mut x;
  // ref1 and ref2 can be assumed not to alias, but they are treated as &i32s,
  // and &i32s are allowed to do this
  ptr::read(ref2);
  ptr::read(ref1)
}
// the following is defined, as ref1 is never touched
{
  let mut x: i32 = 0;
  let ref1: &mut i32 = unsafe { &mut *(&mut x as *mut i32) };
  let ref2: &mut i32 = &mut x;
  // ref1 and ref2 can be assumed not to alias, but ref1 isn't ever read through
  // or written through
  ptr::write(ref2, 8);
  ptr::read(ref2)
}
// the following is Undefined Behavior, as ref2 is written through *even after
// ref1 is read from*, before ref1 goes out of scope
{
  let mut x: i32 = 0;
  let ref1: &mut i32 = unsafe { &mut *(&mut x as *mut i32) };
  let ref2: &mut i32 = &mut x;
  // UB as ref1 and ref2 can be assumed not to alias
  let ret = ptr::read(ref1);
  ptr::write(ref2, 5);
  ret
}
// the following is Undefined Behavior, as both ref1 and ref2 are written
// through
{
  let mut x: i32 = 0;
  let ref1: &mut i32 = unsafe { &mut *(&mut x as *mut i32) };
  let ref2: &mut i32 = &mut x;
  // this is UB as ref1 and ref2 can be assumed not to alias
  ptr::write(ref2, 3);
  ptr::write(ref1, 5); // UB
}
// the following is Undefined Behavior, as the raw pointer is read in the case
// of a reference write
{
  let mut x: i32 = 0;
  let ref1: &mut i32 = &mut x;
  let ptr: *mut i32 = ref1 as *mut i32; // derived from ref1
  let ref2: &mut i32 = ref1; // ptr is not derived from ref2
  // This is UB as ref2 and ptr can be assumed not to alias
  ptr::write(ref2, 15);
  ptr::read(ptr)
}
// the following is defined, as two *mut Ts may alias, and they are both derived
// from the first &mut T
{
  let mut x: i32 = 0;
  let ref_: &mut i32 = &mut x;
  let ptr1: *mut i32 = &mut *ref_;
  let ptr2: *mut i32 = &mut *ref_;
  ptr::write(ptr1, 3);
  ptr::write(ptr2, 8);
  ptr::read(ref_) // defined to return 8
}
// the following is defined, for the same reason as above; derived pointers are
// a tree, remember
{
  let mut x: i32 = 0;
  let ref_: &mut i32 = &mut x;
  let ptr: *mut i32 = &mut *ref_;
  let ptr1: *mut i32 = &mut *ptr;
  let ptr2: *mut i32 = &mut *ptr;
  ptr::write(ptr1, 3);
  ptr::write(ptr2, 8);
  ptr::read(ref_) // defined to return 8
}
```
In other words: references may not observe aliased writes, and a reference only
observes when it is actually used to `ptr::write` or `ptr::read`.

Raw pointers may observe all aliased writes (assuming single threaded code), and
it shall have a defined behavior, and the outcome shall be the same as if all
writes and reads happened in order.

## Typecasting

Typecasting through pointers is fine. The clear example of this is `transmute`.
However, "Implementation Defined" bytes are only readable as either the source
type, or in an implementation defined way (see `std::repr`). Otherwise, if the
type read is valid, then the type read is valid. The following are examples:

```rust
// The following is completely valid
{
  let i32_ptr: *const i32 = &(-5);
  let u32_ptr = i32_ptr as *const u32;
  ptr::read(u32_ptr)
}
// The following is also completely valid
{
  let i32_ptr: *const i32 = &0;
  let f32_ptr = i32_ptr as *const f32;
  ptr::read(f32_ptr)
}
// The following results in an invalid reference, which will result in UB if
// used. However, it is fine to return it.
{
  let isize_ptr: *const isize = &0;
  let ref_ptr = isize_ptr as *const &i32;
  ptr::read(ref_ptr)
}
// The following is Undefined Behavior, as the tuple is larger than the original
// type; in other words, i32_ptr points to 4 bytes of memory, while you are
// reading at least 8
{
  let i32_ptr: *const i32 = &0;
  let tuple_ptr = i32_ptr as *const (i32, i32);
  ptr::read(tuple_ptr)

}
```

`transmute` shall be defined very simply; equivalent to:

```rust
pub const unsafe fn transmute<T, U>(t: T) -> U
    where size_of::<T>() == size_of::<U>() {
    // assuming we get where bounds on values at some point (and const size_of)
  let u = ptr::read(&t as *const T as *const U);
  mem::forget(t);
  u
}
```

# Drawbacks
[drawbacks]: #drawbacks

More complicated rules. These are less easy to explain to people, and don't have
the nice property of being proven (although I believe that they are closer to
reality).

Threading isn't defined in this document; it's only concerned with single
threaded code. The current definitions are good enough, as far as I can tell,
and I don't understand threading well enough to write the standard.

# Alternatives
[alternatives]: #alternatives

Keeping most unsafe code in the dark; currently, "It is an open question to what
degree raw pointers have alias semantics. However it is important for these
definitions to be sound that the existence of a raw pointer does not imply some
kind of live path." This isn't good enough.

# Unresolved questions
[unresolved]: #unresolved-questions

What is the exact definition of using a value?

How do you define a valid discriminant value?

Are signaling NaNs "Invalid"?

What's the deal with `UnsafeCell`? Probably something similar to raw pointers,
except that it only applies to the array of bytes that make up the `UnsafeCell`.
