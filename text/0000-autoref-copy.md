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
    by_ref(s); // This works after this RFC.
    // by_mut_ref(s); // ERROR -- expected `&mut S`, found `S`
}
```

# Motivation
[motivation]: #motivation

When working with `Copy` data, the distinction between borrowed and owned data
is often unimportant. However, generic code often results in code which will
only accept a particular variant of a `Copy` type (`T`, `&T`, `&&T`, ...).
This is a frustration in many places:

```rust
// Comparisons:
let v = vec![0, 1, 2, 3];
v.iter().filter(|x| x > 1); // ERROR: expected &&_, found integral variable
// These work:
0 > 1;
&0 > &1;
// But these don't:
&0 > 1;
0 > &1;

// Trait instantiations:
let mut map: HashMap::new();
map.insert(0, "Hello");
map.insert(1, "world!");
map[0]; // ERROR: expected &{integer}, found integral variable
// or
map.get(1); // ERROR: expected `&{integer}`, found integral variable

// Numeric operators:
// These work:
&0 + &1;
0 + &1;
&0 + 1;
// But these don't:
&&0 + 1;
&&0 + &1;
&&0 + &&1;
```

These interactions confuse both new and experienced users without providing
any significant value. It's clear what's intended by `map.get(1)` or
`vec.iter().filter(|x| x > 1)`. When users encounter these errors in practice,
the only reasonable thing they can do is add `&` and `*` as necessary to make
their code compile.

This RFC seeks to address one particular variant of this problem: passing
owned `Copy` data where a reference was expected.

Example:

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

`Copy` types autoreference: at
[coercion sites](https://github.com/rust-lang/rfcs/blob/master/text/0401-coercions.md#coercions),
when a reference is expected, but an owned
`Copy` type is found, an `&` will be automatically inserted.

Some types have interior mutability through `UnsafeCell`, so passing them
via a hidden reference could allow surprising mutations to occur.
To prevent this, the autoreferencing coercion is limited to types which do not
directly contain an internal `UnsafeCell`.
Types which reference an `UnsafeCell` through an indirection can still be
coerced (e.g. `&MyCopyCell<u8>` to `&&MyCopyCell<u8>`).

This coercion will not occur if the lifetime of the resulting reference would
last longer than the call site. For example:

```rust
// `foo`'s argument can be coerced to from `u8` because it only lasts for the
// lifetime of the function call:
fn foo(x: &u8) { ... }

// `bar`'s argument cannot be coerced to from `u8` because the lifetime of the
// required reference outlives the scope of `bar`.
fn bar<'a>(x: &'a u8) -> &'a u8 { ... }

fn main() {
    foo(1); // OK

    let x = bar(5); // ERROR: expected `&u8`, found `u8`.
}
```

# Drawbacks
[drawbacks]: #drawbacks

- This increases the special behavior of `Copy` types, making it potentially
confusing to new users. Some simple, non-generic code becomes more confusing
because `fn foo(x: &i32) { ... }` can now be called like `foo(5)`;
- The existing coercion mechanism doesn't allow for coercion of generic
arguments, nor does it use coercions in trait lookup. Because of this, users
will still need to manually add `&` when trying to, for example, index into
a `HashMap<usize, T>`
(users will still need to write `x[&5]` instead of `x[5]`).

# Rationale and Alternatives
[alternatives]: #alternatives
One alternative would be to do nothing. However, this issue is frequently annoying for new users, and the solution is relatively simple.

We could also allow borrows which outlive function call sites, but these could
produce surprising errors (for example, if the user attempted to mutate a
variable while it was implicitly borrowed).

We could make the autoreferencing coercion more general and allow `T -> &T`
for all types which don't contain an `UnsafeCell`. However, this could harm
users' ability to reason about when variables are `move`d.

Another alternative would be to also add the following conversions for
`T: Copy`:
- `T` to `&mut T`: This conversion has the potential to introduce hidden
mutations. With this change, passing a variable to a function could allow
the function to change the value of the variable, even if no `&mut` is present.
- `&mut T` and `&T` to `T`: This conversion would cause extra copies to occur
which could be difficult to identify when attempting to optimize code.

# Unresolved questions
[unresolved]: #unresolved-questions
- Can we make it easier to use copy values where references are expected when
working with traits and generic code? In particular, it would be nice to make
operators such as `Index` or `Add` auto-reference and auto-dereference.
It would also be great if we could find some way to trigger existing deref
coercions in generic cases such as passing `&Rc<MyT>` to
`fn foo<T: SomeTrait>(t: &T)`.
