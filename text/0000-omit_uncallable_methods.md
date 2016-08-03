- Start Date: (2016-08-03)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Allow uncallable methods implementations to be omitted from trait
implementations.

# Motivation

These methods impls are superfluous and clutter code. Having to write out
"what" their implementation is seems weird when they don't really have an
implementation at all.

For example, consider the trait impls in libcore/libstd for `!`:

```rust
impl PartialEq for ! {
    fn eq(&self, _: &!) -> bool {
        *self
    }
}

impl PartialOrd for ! {
    fn partial_cmp(&self, _: &!) -> Option<Ordering> {
        *self
    }
}

impl Error for ! {
    fn description(&self) -> &str {
        *self
    }
}

impl Debug for ! {
    fn fmt(&self, _: &mut Formatter) -> Result {
        *self
    }
}

impl Display for ! {
    fn fmt(&self, _: &mut Formatter) -> Result {
        *self
    }
}
```

As `&Self` is uninhabited in all these methods, with this RFC these could be
reduced to:

```rust
impl PartialEq for ! {}

impl PartialOrd for ! {}

impl Error for ! {}

impl Debug for ! {}

impl Display for ! {}
```

Another way to think about this is that we already allow infallible
pattern-matching in function argument lists. This RFC would, in some sense, be
an extension of this feature to unreachable patterns.

# Detailed design

Allow method impls to be omitted from a trait impl if any of the argument types
are uninhabited. Note that this doesn't just include impls on `!`.  Consider
this simplified version of the `Handler` trait from `mio`.

```rust
trait Handler {
    type Timeout;

    fn timeout(&self, timeout: Self::Timeout);

    // Some other items omitted.
}
```

`mio` users can register a `Timeout` with their `Handler` and `mio` will call
the `timeout` method with the `Timeout` they gave it. If someone doesn't want
to use this feature of `Handler`s, the most sensible type to set `Timeout` to
is `!`. In this case, why should they be forced to define the `timeout` method
at all? The method doesn't actually exist, so giving it a method body is
misleading. Rather, by setting `Timeout = !` they've indicated that the method
should be, in some sense, deleted (or at least rendered unusable) similar to
how `!` can be used to "delete" unwanted enum variants.

As such, the impl should be able to be written as

```rust
impl Handler for Foo {
    type Timeout = !;

    // Some other items omitted.
}
```

Note however that they could still write *dead* code which calls the uncallable
method. eg. `foo.timeout(panic!())`.

# Drawbacks

This might confuse people who don't know about this feature when they notice
that code compiles despite some method impls being apparently missing.

# Alternatives

Not do this.

# Unresolved questions

None.

