- Feature Name: introduce_debug_attribute
- Start Date: 2017-02-04
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Make `Debug trait` be usable without a lot of boilerplate

# Motivation
[motivation]: #motivation

We can use `#[derive(Debug)]` to make the compiler implement `Debug trait`.

While developing a project, we strive to make it as much efficient as we can.

This is why a lot of types don't have `#[derive(Debug)]` above them.

This is why when looking for a bug we often add `#[derive(Debug)]` to some types, then to their fields' types.

I propose to add a new attribute `#[Debug]`.

The compiler should implement `Debug trait` if someone uses an item (struct, enum) as `Debug trait`.

The compiler should implement `Debug trait` only if all fields or variants either implement Debug or has `#[Debug]`.

The compiler shouldn't implement `Debug trait` if any field or variant doesn't implement `Debug trait`.

It makes the code as efficient as we want when we don't need it.

# Detailed design
[design]: #detailed-design

I can't describe how should it be implemented. I also can't determine whether it can be implemented at all.

## Examples:

Let's assume we have a `struct Foo`:

```rust
#[Debug]
struct Foo {
    value: i32,
}
```

The first case:

```rust
fn main() {
    let foo = Foo { value: 5 };
    println!("{}", foo.value);
}
```

In the first case we don't use the `struct Foo` as the `Debug trait`
so the compiler shouldn't generate `impl Debug`.

The second case:

```rust
fn main() {
    let foo = Foo { value: 5 };
    println!("{}", foo);
}
```

In the second case we use the `struct Foo` as the `Debug trait`
so the compiler should generate `impl Debug`.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

We should describe the change in all books.
All existing Rust users understand the feature easily.

# Drawbacks
[drawbacks]: #drawbacks

It adds more complexity to the compiler.

# Alternatives
[alternatives]: #alternatives

Don't add this and we will continue to add `#[derive(Debug)]` ourselves.

It's not a problem, but it takes a lot of time.

# Unresolved questions
[unresolved]: #unresolved-questions

Probably there are other drawbacks.
