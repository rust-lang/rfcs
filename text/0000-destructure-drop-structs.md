- Feature Name: Destructure structs implementing Drop
- Start Date: 2017-07-08
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allow destructuring of structs that implement Drop.

# Motivation
[motivation]: #motivation

Destructuring allows to reverse the process of creating a struct from individual fields.
```
struct Compound<A, B> { a: A, b: B }
let a: A = ...
let b: B = ...
let c = Compound { a: a, b: b } // wraps and into the struct Compound
let Compound { a: a, b: b } = c; // destructures c into its fields.
// a and b are now the same they where before c was created
```

However this is currently not possible if the struct implements `Drop`.
The current way to destructure these cases anyway, requires to read non-copy fields via 
`ptr::read(&struct.field)` and then `mem::forget(struct)` to avoid running drop.

This leaves more room for error, than there would have to be: 
1.  forgetting to read all fields, possibly creating a memory leak,
2.  acidentally working with `&struct` instead of `struct`, leading to a double free.
     (The fields are copied, but mem::forget only forgets the reference.)

Allowing to destructure these types would ensure that:
1.  unused fields are dropped,
2.  only owned structs can be destructured.

It is not allowed to implicitly move fields out of structs implementing Drop.
This would make it too easy to accidentally trigger destructuring by accessing a field,
which then means that drop does not run and possibly cause a memory leak.

# Detailed design
[design]: #detailed-design

As 
previously shown destructuring of structs with destructures may create unsoundness
[[1]](https://github.com/rust-lang/rust/issues/3147)
[[2]](https://github.com/rust-lang/rust/issues/26940).

One possible solution would be to make this an `unsafe` operation and only allow it inside an unsafe `block`.
Another possiblity would be to restrict it to modules that are allowed to impement the type in question.

Either restriction would prevent the issue of [[2]](https://github.com/rust-lang/rust/issues/26940).

Say there is the following struct [(taken from here)](https://github.com/rust-lang/rust/issues/26940#issuecomment-120502565):
```rust
pub struct MaybeCrashOnDrop { c: bool }
impl Drop for MaybeCrashOnDrop {
    fn drop(&mut self) {
        if self.c {
            unsafe { *(1 as *mut u8) = 0 }
        }
    }
}
pub struct InteriorUnsafe { pub m: MaybeCrashOnDrop }
impl InteriorUnsafe {
    pub fn new() -> InteriorUnsafe {
        InteriorUnsafe { m: MaybeCrashOnDrop{ c: true } }
    }
}
impl Drop for InteriorUnsafe {
    fn drop(&mut self) {
        self.m.c = false;
    }
}
```

## Approach 1:
```rust
let i = InteriorUnsafe::new();
unsafe {
     let InteriorUnsafe { m: does_crash } = i;
}
```
Here full responsibility is given to the user, ensuring that the destructuring is sound.

Pros: More flexibility without resorting to ptr::read & mem::forget.

Cons: It is still a special case of destructuring and requires an `unsafe` block.

## Approach 2:
```rust
// somewhere in the same module where InteriorUnsafe was declared
let i = InteriorUnsafe::new();
let InteriorUnsafe { m: does_crash } = i;
```
Destructuring does not require an `unsafe` block, but is limited to the module where the type to be destructured was defined.
This ensures no third party may create unsound behaviour by incorrectly destructuring a foreign type.

Pros: No unsafe block required

Cons: If a struct implementing Drop needs to be destructured outside the module of definiton, the ptr::read & mem::forget are still required. However this will only affect structs with public fields.

## Approach 1 & 2:
Inside the module of declaration, no `unsafe` block is required. Otherwhise an `unsafe` block is required.

Pros: does not require an `unsafe` block in most cases.
Cons: more complexity than required.

## Approach 3:
Introduce an `unsafe` auto-trait `IndependentDrop`.
It is automatically derived when all fields implement `IndependentDrop`.

Destructuring of structs is possible if they do not implement `Drop`, or when they implement `IndependentDrop`.

Pros: No `unsafe` block required and destructuring is not limited to the same module.
Cons: Requires an additional trait that authors have to be aware of.

## Suggested approach: 
This RFC proposes solution Approch 2.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

Probably with a small section in the Nomicon, that explains why destructuring might be dangerous and under what circumstances it is allowed.

# Drawbacks
[drawbacks]: #drawbacks

Why should we *not* do this?

# Alternatives
[alternatives]: #alternatives

The trivial alternative is to keep it as is, and keep using `ptr::read` & `mem::forget`.
This could possibly be automated with a macro.

# Unresolved questions
[unresolved]: #unresolved-questions

Which restrictions are required to make this sound?
