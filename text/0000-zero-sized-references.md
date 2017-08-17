- Feature Name: zero_sized_references
- Start Date: 2017-06-23
- RFC PR: [#2040](https://github.com/rust-lang/rfcs/pull/2040)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

References to Zero-Sized Types (both shared and mutable) have been historically the size of `usize`.
The proposed change is to make them also ZST.

# Motivation
[motivation]: #motivation

References to any type in rust are represented as a pointer. Usually the pointer is smaller and faster to move around.
However for Zero-Sized Types that only have a single value (for example `()` ) moving around is a no-op, and can be optimized away.
Reading and writing the value is a no-op since it has only a single value anyway and therefore it carries no extra information.
However, currently the compiler can't optimize away the pointer from data structures.

Zero-Sized Types are useful for functions, lifetime guarantees and destructors,
and references to them can be used to show these types "exist" (for references) or "you are the only one using it" (for mutable references).
The actual value is meaningless and the representation should be optimized to be of size 0 as well.

In addition, references to Zero-Sized Types often appear in polymorphic code, where they handle non-ZST as well.

In both of these cases, it will be an advantage for the references to be Zero-Sized.

# Detailed design
[design]: #detailed-design

### Calculating size
[calculating-size]: #calculating-size

Disclaimer: The writer of this RFC is not familiar with the inworkings of the compiler.


Finding if a reference points to a ZST or not may not always be trivial.
A struct with a reference to itself will not know it's size until after it knows the size to the reference.

However, a couple of notes:

```rust
struct A<'a> (&'a A<'a>);
```
This struct cannot be instantiated, because the very first instance of it requires an instance of it to already exist.

```rust
enum A<'a> {
    ZeroSized,
    SelfRef (&'a A<'a>),
}
```
The moment an enum has more than a single value, it cannot be Zero-Sized. Otherwise it isn't different than a struct.

Therefore, I propose to assume that whenever you find a self reference (or multiple types referencing in a loop),
decide the reference is not Zero-Sized, since there most likely WILL be other data somewhere in the chain.
This is most relevant to unions, which could have self-references and be instantiated at the same time.

### `*const` and `*mut` pointers
[pointers]: #pointers

It is possible to convert a reference to a pointer. Currently, a reference to ZST points to an arbitrary location,
and when converting to a pointer the pointer recieves that arbitrary location.

After this change, the reference will not hold any data. I propose that whenever a ZST reference is converted to a pointer,
a warning/error be issued ("Warning: taking the address of a Zero-Sized Type is meaningless") and the pointer will recieve an address
with the same algorithm that assigned an address to the reference in our current implementation.

The purpose is to not break current code that might do this. We probably don't want to assign Null since for pointers it has
a meaning that the value doesn't exist, which is different than "exist but no data".

Converting in the other direction, the value of the pointer will be silently dropped - that value never had a meaning in the first place.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

For most rust users, this change will be invisible. Their code will just become a tiny bit smaller.

Users of unsafe rust might encounter this case. Therefore there should probably be a note in the nomicon that references might be optimized away for ZST.

# Drawbacks
[drawbacks]: #drawbacks

### Breaking code

Any place that assumed a reference holds a pointer might introduce bugs.

FFI might behave differently than now, breaking code (see Unresolved questions).

# Alternatives
[alternatives]: #alternatives

The system that is now works well, and does not have to be changed.

# Unresolved questions
[unresolved]: #unresolved-questions

### Definition of `&`

Is the definition of `&` state that we guarantee it's a pointer? Or do we only promise you can use it to access the data?

### FFI + Escape mechanism

Do we need a way to remove the optimization? For example, if we have
```rust
struct ExternalStruct;

let x: &mut ExternalStruct = ffi_function();
other_ffi_function(x);
```
We might want to represent a pointer we got from FFI and checked it's not null as a reference to an object.
Since that object is not accessable directly we represent it as an empty struct.
However then the references to it are optimized away and we lose the pointer and then can't call the second function.

Note: This break will not be silent. The safe wrapper for other_ffi_function will convert the reference back to pointer before
using it, raising the warning/error of converting references to pointers.

For that case, we might want to mark `ExternalStruct` as Non-Zero-Sized. Possibly
```rust
impl !Sized for ExternalStruct;
```
and maybe it should require `unsafe`.

### Errors of conversion

Do we want to give an error or a warning for converting references to pointers?

The code above is an example where we break working code, so an error might be needed to show the significance.

However, some conversions might be meaningless and wouldn't affect the execution, so the programmer might allow the conversion.

### Mitigating breakage

Safe rust code should be affected positively by this change. However unsafe code might break.

FFI is especially vulnurable to this change, as shown above.
Are there better ways to deal with these errors without user involvement?

(Unsafe code might break - are there examples of VALID code that breaks? Or does only invalid uses of references break?
And what is our stance on breaking invalid code, assuming there is a large amount of it?)

### Specific examples - Pro

The RFC isn't well-justified until it has at least one detailed use case where it helps.
Please share specific examples of code where Zero-Sized references are useful.

### Specific examples against

If you have specific examples where this change is detrimental, please share them.
