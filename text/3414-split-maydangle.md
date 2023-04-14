- Feature Name: `split_maydangle`
- Start Date: 2023-02-13
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add `#[needs_drop]`, ignore `PhantomData` for outlives requirements.

# Motivation
[motivation]: #motivation

This fails to compile:

```rust
use core::marker::PhantomData;

struct PrintOnDrop<'s>(&'s str);
impl<'s> Drop for PrintOnDrop<'s> {
    fn drop(&mut self) {
        println!("{}", self.0);
    }
}

fn to_pd<T>(_: T) -> PhantomData<T> {
    PhantomData
}

pub fn foo() {
    let mut x;
    {
        let s = String::from("temporary");
        let p = PrintOnDrop(&s);
        x = (to_pd(p), String::new());
    }
}
```

And yet, this compiles:

```rust
use core::marker::PhantomData;

struct PrintOnDrop<'s>(&'s str);
impl<'s> Drop for PrintOnDrop<'s> {
    fn drop(&mut self) {
        println!("{}", self.0);
    }
}

fn to_pd<T>(_: T) -> PhantomData<T> {
    PhantomData
}

pub fn foo() {
    let mut x;
    {
        let s = String::from("temporary");
        let p = PrintOnDrop(&s);
        x = (to_pd(p), ());
    }
}
```

Since the values in the tuple are unrelated, they should not affect each other.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

A type marked `#[needs_drop]` gets checked for liveness at drop. This is
necessary for `Vec`:

```rust
struct Vec<T> {
  ...
}

unsafe impl<#[needs_drop] #[may_dangle] T> Drop for Vec<T> {
  fn drop(&mut self) {
    ...
  }
}
```

So that this compiles:

```rust
fn main() {
  let mut v = vec![];
  {
    v.push(&String::from("temporary"));
  }
}
```

But this cannot compile, as it would be unsound:

```rust
struct PrintOnDrop<'s>(&'s str);
impl<'s> Drop for PrintOnDrop<'s> {
    fn drop(&mut self) {
        println!("{}", self.0);
    }
}

fn main() {
  let mut v = vec![];
  {
    v.push(PrintOnDrop(&*String::from("temporary")));
  }
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This RFC removes the dropck/outlives constraints from `PhantomData` and moves
them into the relevant `Drop` impls instead.

# Drawbacks
[drawbacks]: #drawbacks

Requires mild churn to update things to the new way. Failing to update wouldn't
break existing code, but would allow unsound code to compile.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

A type which doesn't need drop should never have dropck/outlives contraints,
but due to the rushed way in which `may_dangle` was implemented, `PhantomData`
ended up having this unfortunate behaviour. This RFC removes this behaviour and
allows strictly more code to compile.

# Prior art
[prior-art]: #prior-art

- Compiler MCP 563: It is the exact same thing as this RFC, but a full RFC
    seemed appropriate due to observable changes on stable, even if they are
    fairly obscure.
- Unsound dropck elaboration for `BTreeMap`: <https://github.com/rust-lang/rust/pull/99413>
- `may_dangle`: RFC 1238, RFC 1327
- This is effectively split from RFC PR 3390 and is not intended for
    stabilization.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

N/A

# Future possibilities
[future-possibilities]: #future-possibilities

The full RFC 3390, and stabilization.
