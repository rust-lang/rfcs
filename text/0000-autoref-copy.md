- Feature Name: autoref_copy
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC allows autoreferencing of `T` where `T: Copy`.

Example:

```rust
[derive(Copy, Clone)]
struct S(u8);

fn by_val(_: S) {}
fn by_ref(_: &S) {}
fn by_mut_ref(_: &mut S) {}

fn main() {
    let s = S(5);
    by_val(s);
    by_ref(s);
    // by_mut_ref(s); // ERROR -- expected `&mut S`, found `S`
}
```

# Motivation
[motivation]: #motivation

When working with `Copy` data, the distinction between borrowed and owned data
is often unimportant. However, generic code often results in function which
expect a reference rather than an owned value. In these cases, users have to
add manually create a reference by adding `&` at the call site.

```rust
use std::collections::HashMap;
fn main() {
    let mut map = HashMap::new();
    map.insert(1, "hello!");

    println!("{:?}", map.get(1)); // ERROR: expected `&{integer}`

    // Instead, users have to write this:
    println!("{:?}", map.get(&1));
}
```

This is an unnecessary frustration. This RFC proposes to prevent this issue
by auto-referencing `Copy` types.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When calling a function which expects a reference to a `Copy` type, you can
pass an owned `Copy` type and a reference will be created automatically:

```rust
fn print_num(x: &usize) { println!("{:?}", x); }

fn main() {
  let x = 5;
  print_num(x);
}
```

This is particularly convenient when working with some generic types whose
methods take references as arguments:

```rust
use std::collections::HashMap;
fn main() {
    let mut map = HashMap::new();
    map.insert(1, "hello!");

    // We can write this:
    println!("{:?}", map.get(1));

    // Instead of having to write this:
    println!("{:?}", map.get(&1));
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

`Copy` types will autoreference: at
[coercion sites](https://github.com/rust-lang/rfcs/blob/master/text/0401-coercions.md#coercions),
when a reference is expected, but an owned
`Copy` type is found, an `&` will be automatically inserted.

If the `Copy` value is a temporary, its lifetime will be promoted to meet the
required lifetime of the reference (where possible).
This behavior is specified in detail in
[RFC 66](https://github.com/rust-lang/rust/issues/15023) and @nikomatsakis's
[amendment](https://github.com/nikomatsakis/rfcs/blob/rfc66-amendment/text/0066-better-temporary-lifetimes.md).

When it is not possible to promote the `Copy` value's lifetime to the lifetime
required by the reference, a customized error will be issued:

```rust
struct u8Ref<'a>(&'a u8);

fn new_static(x: u8) -> u8Ref<'static> {
    u8Ref(x) // ERROR: borrowed value does not live long enough
    //    ^autoreference occurs here, but `x` does not live long enough
}
```

If the lifetime of the reference would overlap with a mutable reference or a
mutation of the referenced value, a custom error will be issued:

```rust
struct u8Ref<'a>(&'a u8);

fn main() {
    let mut x = 5;
    let y = u8Ref(x);
    //            ^ autoreference of `x` occurs here
    x = 7; // ERROR: cannot assign to `x` because it is borrowed
}
```

# Drawbacks
[drawbacks]: #drawbacks

- This increases the special behavior of `Copy` types, making it potentially
confusing to new users.
- The existing coercion mechanism doesn't allow for coercion of generic
arguments, nor does it use coercions in trait lookup. Because of this, users
will still need to manually add `&` when trying to, for example, index into
a `HashMap<usize, T>`
(users will still need to write `x[&5]` instead of `x[5]`).
- Autoreferencing may produce surprising errors when attempting to mutate data.

# Rationale and Alternatives
[alternatives]: #alternatives

One alternative would be to do nothing. However, this issue is frequently
annoying for new users, and the solution is relatively simple.

Another alternative would be to also add the following conversions for
`T: Copy`:
-`T` to `&mut T`: This conversion has the potential to introduce hidden
mutations. With this change, passing a variable to a function could allow
the function to change the value of the variable, even if no `&mut` is present.
-`&mut T` and `&T` to `T`: This conversion would cause extra copies to occur
which could be difficult to identify when attempting to optimize code.

# Unresolved questions
[unresolved]: #unresolved-questions
- Can we make it easier to use copy values where references are expected when
working with traits and generic code? In particular, it would be nice to make
operators such as `Index` auto-reference. One possible way of doing this would
be to introduce default implementations such as
`Index<&Idx>` for `T: Index<Idx>`.
