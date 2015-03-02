- Feature Name: named_and_destructable_self
- Start Date: 2015-03-02
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Allow destructing assignment for `self` argument.

# Motivation

It will allow consistency between "normal" arguments and implicit `self`

# Detailed design

For now there is no way to destruct `self` other than using additional line:

```rust
impl Foo {
    fn bar(self) {
        let Foo(a, b) = self;
        …
    }
}
```

Which is inconsistent with "normal" arguments, where we can:


```rust
impl Foo {
    fn bar(self, Foo(a, b): Self) {
        …
    }
}
```

Proposition is to use `self` and `&self` as syntactic sugar for:

```rust
impl Foo {
    fn bar(self: Self) {} // the same as `fn bar(self) {}`
    fn baz(self: &Self) {} // the same as `fn baz(&self) {}`
}
```

It will also allow destructive assignments:

```rust
impl Foo {
    fn bar(Foo(a, b): Self) {}
}
```

# Drawbacks

In some cases it can be misleading to see which one is instance method and which
is struct method, but it shouldn't be big issue. Bigger issue would be documentation
which can have problems with infering that `fn bar(Foo(a, b): Self)` should be
presented as `fn bar(self)`.

# Alternatives

- `fn bar(Foo(a, b): self)` but this is stupid idea as it can confuse people that
  at one moment there should be used `Self` and at other one `self`.

# Unresolved questions

- Would it lead to use different names to `self` (i.e. `this`)? I think not,
  but question still exist.
- Is it really that big issue that we need another line to destruct `self`?
- Is there reason for introducing it at this stage?
