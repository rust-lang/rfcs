- Start Date: 2014-05-19
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Allow the `unsafe` keyword on individual struct fields, and force such fields
to be accessed only within `unsafe` blocks.

# Motivation

Instead of utilizing privacy, allow library authors to leave a field public
while communicating that accessing certain struct fields may violate assumed
  invariants that could lead to unsafe behavior.

# Drawbacks

May be the teensiest bit disingenuous, as it is the *effect* of the field
access that is potentially unsafe, not the field access itself. May also be
redundant with field privacy.

# Detailed design

Consider the following program:
```rust
struct Foo { data: Vec<int>, idx: uint }

impl Foo {
    fn new() -> Foo { Foo { data: vec![], idx: 0 } }

    fn set(&mut self, newdata: Vec<int>) { self.data = newdata; }

    fn get(&self) -> int {
        unsafe {
            *(self.data.as_slice().unsafe_ref(self.idx))
        }
    }
}

fn main() {
    let mut foo = Foo::new();
    foo.set(vec![7,8,9]);
    foo.idx = 5; // uh-oh!!
    println!("{}", foo.get());
}
```
Surely the author of `Foo` would love
to disallow code from randomly mutating `idx`, but privacy only extends beyond
module boundaries. Furthermore, merely using privacy here isn't
self-documenting: there are plenty of structs with fields that probably
*shouldn't* be mutated, but only a few of those fields will actually be
*unsafe* to mutate.

This proposal would make the following struct definition legal:
```rust
struct Foo { unsafe x: int }
```
...and would make the following code illegal:
```rust
let foo = Foo { x: 1 }; foo.x;  //  error: access of unsafe field requires unsafe function or block
```
All accesses of unsafe fields would require the user to explicitly enter unsafe code.

Unfortunately the compiler has no way to verify that all fields that are unsafe
to mutate will actually be marked `unsafe`, so it's possible that, if `unsafe`
is allowed on struct fields, its accidental omission could lead to a false
sense of security. However, that's exactly the same boat that we're in today,
where functions can freely do unsafe things in `unsafe` blocks without
themselves being marked as unsafe to use.

# Alternatives

This RFC proposes that all accesses of `unsafe` fields require an `unsafe`
block. As an alternative, we could allow reading the field to be possible in
safe code, while only requiring `unsafe` blocks for actions that could
potentially mutate the field, such as assignment and taking `&mut` references
to the field.

# Unresolved questions

If privacy is a more appropriate mechanism here, does allowing unsafe access to
struct fields at all open us up to accidentally making Rust code more unsafe
rather than less?
