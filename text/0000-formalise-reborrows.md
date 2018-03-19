- Feature Name: Formalise Reborrows
- Start Date: 2018-03-19
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Formalise the re-borrowing logic used on `&mut T`.

# Motivation
[motivation]: #motivation

Solve https://github.com/rust-lang/rfcs/issues/1403:
*Some way to simulate `&mut` reborrows in user code*.

Currently user types including `&mut T` fields are not as powerful as real
`&mut` types: the latter can be *reborrowed*, whereas the former can't.
Example uses:

-   allow copying of `Option<&mut T>`
-   applying the compiler's aliasing analysis to other uses where only a single
    active alias must exist, without requiring an actual reference
-   implicit reborrowing with `fn foo<X: T>(x: X)` functions where the trait
    `T` is implemented for `&'a X` (or `&'a mut X`) types

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

What is a reborrow? Say we have a function `f` taking an argument `x` of type
`T`. What could that look like?

|   | `f(x: &T)` | `f(x: T)` where `T: Copy` |
| --- | --- | --- |
| `f(y), y: T` | error | move or copy |
| `f(y), y: &T` | reborrow | deref and copy |

A *reborrow* is essentially a *copy* of a pointer with extra lifetime analysis:
in the case of immutable references, the reborrow `x: &T` must have shorter
lifetime than the parent `y: &T`; in the case of mutable references, there is
an additional restriction: the parent `y: &mut T` cannot be used until after
the child `x: &mut T` expires.

## Reborrowing derived types

If, instead of reborrowing a `&T` or `&mut T` type, you want to reborrow a
derived type, currently, you're out of luck. Well, not quite:

```rust
struct MyRef<'a, T: 'a + ?Sized>(&'a T);

fn a(x: MyRef<str>) {
    println!("a has: {}", x);
}

fn b(x: MyRef<str>) {
    a(MyRef(x.0));  // manual reborrow: reconstruct reference
    println!("b called: a({})", x);
}
```

It would be really nice if in `b`, we could just write `a(x);`, but if we try
that we get:

```
error[E0382]: use of moved value: `x`
  --> src/main.rs:17:33
   |
16 |     a(x);
   |       - value moved here
17 |     println!("b called: a({})", x);
   |                                 ^ value used here after move
   |
   = note: move occurs because `x` has type `MyRef<'_, str>`, which does not implement the `Copy` trait
```

The compiler is telling us that `a(x)` is interpreted as *copy `x` into `a` if
`x` supports `Copy`, otherwise move `x` into `a`*. Except, as we know, if `x`
had type `&str` instead of `MyRef<str>` for both the argument and parameter
type, the compiler would have no problem interpreting this as a *reborrow*.
A reborrow is not quite a copy due to the requirement for shorter lifetimes.

Full example: https://play.rust-lang.org/?gist=d343ac07c9faf21607a5ad92bbaf5f45&version=stable

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Reborrowing trait

We introduce the following trait, as member of `std::marker`:

```rust
trait Reborrow {
    fn reborrow(&self) -> ???;
}
```

Unfortunately the return type of `fn reborrow` isn't representable in general;
the return type must be `Self` but with different lifetime(s) (and note that
there will be multiple lifetimes if the type has multiple fields with
lifetimes). It is not possible to use an associated type `type Result: ???`
because the return type includes a lifetime (or multiple) bound by the caller.

Fortunately it *is* possible to write specific implementations in Rust today,
for example the above `MyRef` type can be [given a reborrow implementation](https://play.rust-lang.org/?gist=67e4600b48d123fea45dd2b9bdcbf035&version=stable):

```rust
impl<'a, T: 'a + ?Sized> Reborrow for MyRef<'a, T> {
    fn reborrow<'b>(&self) -> MyRef<'b, T> where 'a: 'b {
        MyRef(self.0)
    }
}
```

### Implementation

The `&T` and `&mut T` types already support reborrowing semantics; this
formalisation requires that both support the `Reborrow` trait.

In theory, it should be possible to manually implement `Reborrow` with code
similar to the above; I do not know if this is possible due to the improper
definition of the return type.

Rust should have built-in support for deriving the `Reborrow` trait. The
derived implementation should copy any fields supporting `Copy` and reborrow
any fields supporting `Reborrow`; if neither is possible for any field then the
trait cannot be derived.

Anonymous types should have this trait automatically derived as is the case for
`Copy` today; e.g. `(u32, &i32)` should support `Reborrow`.

## Usage of reborrow

Today, reborrowing happens automatically for references, as in the `f(y)`
example from the table above. We formalise this:

When a parameter `x` is passed into a function `f` (e.g. `f(x)`),

-   if `x` supports `Copy` [and `x` is later reused], then a copy of `x` will
    be passed into `f`
-   if `x` supports `Reborrow` [and `x` is later reused], then a reborrow of `x`
    will be passed into `f`
-   otherwise, `x` will be moved into `f` [or usage is an error]

### Explicit reborrow

Implicit usage does not imply that explicit usage is impossible. It is proposed
that calling `reborrow` directly is possible, so long as the trait is in scope:

```rust
use std::marker::reborrow;

let x = 1;
let y = &mut x;
let z = y.reborrow();
```


# Drawbacks
[drawbacks]: #drawbacks

This RFC introduces a trait which cannot be defined in stable Rust today. This
is a challenge which would require significant syntax extensions to fix, and
may never be resolved.

# Rationale and alternatives
[alternatives]: #alternatives

Arguably the `Reborrow` trait should be called `Reborrowable`, but that
name is unwieldy, and besides, `Copy` and `Clone` have verb rather than
adjective names.

Possibly `Reborrow` should have its own sub-module of `std`, as `Clone` does.

Alternatively, the concept of *reborrowing* could remain entirely implicit, as
it is today but with automatic implementation for derived types as above.

Given that we have significant motivation for solving this problem and that
the compiler already has reborrowing logic, re-using that logic as much as
possible is desirable. This RFC merely seeks to formalise the existing concepts,
apply them to a slightly broader probem, and document this.

I will also say that formal documentation of the reborrowing logic is a strong
secondary goal of this RFC; without documentation or notation to describe the
reborrow logic I have found it a confusing concept; with notation and
documentation it is a much more approachable concept.

# Prior art
[prior-art]: #prior-art

See https://github.com/rust-lang/rfcs/issues/1403. No known prior to this RFC
exists.

# Unresolved questions
[unresolved]: #unresolved-questions

How to define the return type of `Reborrow::reborrow`.
