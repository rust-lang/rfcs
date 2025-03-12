# Shorter temporary lifetimes in tail expressions

- Feature Name: `shorter_tail_lifetimes`
- Start Date: 2023-05-04
- RFC PR: [rust-lang/rfcs#3606](https://github.com/rust-lang/rfcs/pull/3606)
- Tracking Issue: [rust-lang/rust#123739](https://github.com/rust-lang/rust/issues/123739)

# Summary

In the next edition, drop temporaries in tail expressions *before* dropping locals, rather than after.

![A diagram showing a function with one let statement "let x = g();" and a tail expression "temp().h()"
and a visualisation of how long x and temp live before and after this change.
Before: x is created first, then temp is created, then x is dropped, then temp is dropped.
After: x is created first, then temp is created, then temp is dropped, then x is dropped.
](3606-temporary-lifetimes-in-tail-expressions/diagram.svg)

# Motivation

Temporaries in the tail expression in a block live longer than the block itself,
so that e.g. `{expr;}` and `{expr}` can behave very differently.

For example, this fails to compile:

```rust
// This fails to compile!
fn f() -> usize {
    let c = RefCell::new("..");
    c.borrow().len() // ERROR!!!
}
```

The temporary `std::cell::Ref` created in the tail expression will be dropped
after the local `RefCell` is dropped, resulting in a lifetime error.

This leads to having to add seemingly unnecessary extra `let` statements
or having to add seemingly unnecessary semicolons:

```rust
fn main() {
    let c = std::cell::RefCell::new(123);

    if let Ok(mut b) = c.try_borrow_mut() {
        *b = 321;
    }; // <-- Error if you remove the semicolon!
}
```

Both of these examples will compile fine after the proposed change.

# Guide-level explanation

Temporaries are normally dropped at the end of the statement.

The tail expression of a block
(such as a function body, if/else body, match arm, block expression, etc.)
is not a statement, so has its own rule:

- Starting in Rust 2024,
  temporaries in tail expressions are dropped after evaluating the tail expression,
  but before dropping any local variables of the block.

For example:

```rust
fn f() -> usize {
    let c = RefCell::new("..");
    c.borrow().len() // Ok in Rust 2024
}
```

The `.borrow()` method returns a (temporary) `Ref` object that borrows `c`.
Starting in Rust 2024, this will compile fine,
because the temporary `Ref` is dropped before dropping local variable `c`.

# Reference-level explanation

For blocks/bodies/arms whose `{}` tokens come from Rust 2024 code,
temporaries in the tail expression will be dropped *before* the locals of the block are dropped.

# Breakage

It is tricky to come up with examples that will stop compiling.

For tail expressions of a function body, such code will involve a tail
expression that injects a borrow to a temporary
into an already existing local variable that borrows it on drop.

For example:

```rust
fn why_would_you_do_this() -> bool {
    let mut x = None;
    // Make a temporary `RefCell` and put a `Ref` that borrows it in `x`.
    x.replace(RefCell::new(123).borrow()).is_some()
}
```

We expect such patterns to be very rare in real world code.

For tail expressions of block expressions (and if/else bodies and match arms),
the block could be a subexpression of a larger expression.
In that case, dropping the (not lifetime extended) temporaries at the end of
the block (rather than at the end of the statement) can cause subtle breakage.
For example:

```rust
    let zero = { String::new().as_str() }.len();
```

This example compiles if the temporary `String` is kept alive until the end of
the statement, which is what happens today without the proposed changes.
However, it will no longer compile with the proposed changes in the next edition,
since the temporary `String` will be dropped at the end of the block expression,
before `.len()` is executed on the `&str` that borrows the `String`.

(In this specific case, possible fixes are: removing the `{}`,
using `()` instead of `{}`, moving the `.len()` call inside the block, or removing `.as_str()`.)

Such situations are less rare than the first breakage example, but likely still uncommon.

The other kind of breakage to consider is code that will still compile, but behave differently.
However, we also expect code for which it the current drop order is critical is very rare,
as it will involve a Drop implementation with side effects.

For example:

```rust
fn f(m: &Mutex<i32>) -> i32 {
    let _x = PanicOnDrop;
    *m.lock().unwrap()
}
```

This function will always panic, but will today poison the `Mutex`.
After the proposed change, this code will still panic, but leave the mutex unpoisoned.
(Because the mutex is unlocked *before* dropping the `PanicOnDrop`,
which probably better matches expectations.)

# Edition migration

Since this is a breaking change, this should be an edition change,
even though we expect the impact to be minimal.

We need to investigate any real world cases where this change results in an observable difference.
Depending on this investigation, we can either:

- Not have any migration lint at all, or
- Have a migration lint that warns but does not suggest new code, or
- Have a migration lint that suggests new code for the most basic common cases (e.g. replacing `{}` by `()`), or
- Have a migration lint that suggests new code for all cases (e.g. using explicit `let` and `drop()` statements).

We highly doubt the last option is necessary.
If it turns out to be necessary, that might be a reason to not continue with this change.

# Drawbacks

- It introduces another subtle difference between editions.
  (That's kind of the point of editions, though.)

- There's a very small chance this breaks existing code in a very subtle way. However, we can detect these cases and issue warnings.

# Prior art

- There has been an earlier attempt at changing temporary lifetimes with [RFC 66](https://rust.tf/rfc66).
  However, it turned out to be too complicated to resolve types prematurely and
  it introduced inconsistency when generics are involved.

# Unresolved questions

- How uncommon are the situations where this change could affect existing code?
- How advanced should the edition lint and migration be?
- Can we make sure a lint catches the cases with unsafe code that could result in undefined behaviour?

# Future possibilities

- Not really "future" but more "recent past":
  Making temporary lifetime extension consistent between block expressions and
  if/else blocks and match arms. This has already been implemented and approved:
  https://github.com/rust-lang/rust/pull/121346

- Dropping temporaries in a match scrutinee *before* the arms are evaluated,
  rather than after, to prevent deadlocks.
  This has been explored in depth as part of the
  [temporary lifetimes effort](https://rust-lang.zulipchat.com/#narrow/stream/403629-t-lang.2Ftemporary-lifetimes-2024),
  but our initial approaches didn't work out.
  This requires more research and design.

- An explicit way to make use of temporary lifetime extension. (`super let`)
  This does not require an edition change and will be part of a separate RFC.
