- Start Date: 2014-12-06
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

This is a request to change RFC 0445 to disallow blanket implementations of `FooExt` traits.

# Motivation

RFC0445 describes a convention to split traits into two, one named `Foo`, another `FooExt`. One
of the reasons is to allow splitting object-safe and unsafe parts into two. It suggest to also
provide a blanket implementation `impl FooExt<A> for Foo<A>`.

This introduces a problem: if `FooExt` contains default implementations for methods, all of them
become fixed. As there is no mechanism to provide a more specific implementation of the `FooExt`
trait taking precedence, there is no way for a user to provide better implementations for those
methods.

Disallowing blanket implementations would mitigate that.

# Detailed design

The `Iterator` and `IteratorExt` implementations are good examples. Consider an Iterator over a
finite collection with random access. A good implementation for `last()` in that case is:

```rust
impl IteratorExt<A> for RaCollection<A> {
    fn last(mut self) -> Option<A> {
        self[self.length - 1]
    }
}
```

Currently, this is not possible to implement, as `IteratorExt` is already implemented as, following
RFC 0445:

```rust
public trait IteratorExt<A> {
  //....
  fn last(mut self) -> Option<A> {
      let mut last = None;
      for x in self { last = Some(x); }
      last
  }
}

impl<A, I> IteratorExt<A> for I where I: Iterator<A> {}
```

This leads to competing implementations.

Implementors wishing to override just `last()` cannot use the IteratorExt trait, needing to implement their
own, introducing the following problems:

* Code opting into `Ext` instead of the trait will use unintended, possibly less performant, implementations
* In the case of wrapping other libraries (e.g. C libraries), some might already have implementations that need
  to be used.
* Mixing between their own Iterators and others in one file becomes inconvenient, as importing `IteratorExt`
  will clash with their implementations
* Sidestepping the issue by using a different vocabulary (e.g. `fast_last()`) can introduce usage errors.

Instead, I propose to disallow blanket implementations and implement the `Ext` type specifically for each type:

```rust
impl Iterator<A> for RaCollection<A> {
  //...
}

impl IteratorExt<A> for RaCollection<A> {
  //...
}
```

This has the same advantages of splitting described in RFC 0445 while still allowing users to work with
those traits as with all others.

# Drawbacks

The main drawback is that all `Foo` implementations must remember opting into `FooExt` as well.

# Alternatives

If RFC 0445 stays as it is, it might lead to blocking of vocabulary on a large scale (the mentioned
`IteratorExt` takes a lot of standard vocabulary).

It also might lead to user frustration if they have to sidestep these implementations.

Another alternative would be to allow more specialized implementations for traits.

# Unresolved questions

Have I forgotten any other solutions to the stated problem?

Can the drawback be solved using a macro?
