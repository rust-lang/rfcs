- Feature Name: rename_attr
- Start Date: 2015-06-03
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Extend `use` to work in trait definitions and impl blocks for re-exporting associated
items under different names. This provides a clean way to rename items harmlessly, and
provides a more ergonomic way to provide the same functionality under different names
when necessary to satisfy an API.

This enables

```rust
trait Iterator {
    fn len_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }

    #[deprecated("renamed to len_hint")]
    use Self::len_hint as size_hint;
}
```



# Motivation

Naming things is hard, and APIs churned enough near the release of 1.0 that we stabilized on
some suboptimal names. We'll probably do it again in the future. Therefore it would be
desirable to deprecate these in favour of a rename. It's currently possible to do this for
top level items like structs, traits, and functions as follows:

```rust
#[deprecated(reason = "renamed to new struct")]
pub use NewStruct as OldStruct;

// Note: this was originally called OldStruct
struct NewStruct {
    ...
}
```

However it is not possible to do this for scoped items like methods (inherent or trait).
For inherent implementations this isn't a particularly big deal, just slightly less
ergonomic. However for trait methods this can be particularly nasty. For instance,
consider size_hint on Iterator:

```rust
trait Iterator {
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }
}
```

If we want to rename size_hint to len_hint, we could do the following:

```rust
trait Iterator {
    #[deprecated(reason = "renamed to len_hint")]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }

    #[allow(deprecated)]
    fn len_hint(&self) -> (usize, Option<usize>) {
        self.size_hint()
    }
}
```

However this is *backwards*. In particular, everyone in the world *wants* to
just implement len_hint since size_hint is deprecated, but the "most effecient"
thing to do is implement size_hint. If you *do* just implement len_hint, then
you have the unfortunate situation that *size_hint is still implemented, but
with a different implementation!

If we instead do the reverse:

```rust
trait Iterator {
    #[deprecated(reason = "renamed to len_hint")]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.len_hint()
    }

    fn len_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }
}
```

Then legacy implementors of size_hint will fail to provide len_hint, which is what
everyone should be using!

What we would like to do is explicitly state that these two methods are *one and the
same*, and that implementing one is implementing the other. Implementing *both* in
particular should be illegal. Like with structs, we would like to do the following:

```rust
trait Iterator {
    fn len_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }

    #[deprecated(reason = "renamed to len_hint")]
    use Self::len_hint as size_hint;
}
```

All legacy implementors of size_hint would be correctly forwarded to len_hint, and
all conformant implementors of len_hint would be silently forwarded to legacy consumers
of size_hint. In addition, any attempt to implement both would be met with a duplicate
implementation error (as implementing *any* method twice would).



# Detailed design

Allow `use` to be placed in trait definition and impl blocks to re-export arbitrary
associated items under a different name. In traits declarations, this would enable
a trait to be implemented by providing an item under *either* name, while specifically
marking one name as deprecated. In inherent impl blocks, this would enable providing
the same method under different names.

This RFC does not currently propose allowing traits to be implemented by using
a method with the appropriate name. It also does not propose allowing names to be
obtained from other impl blocks. This is to minimize complexity, although the
author's knowledge of the ever-cryptic *resolve* subsystem is sufficiently limited
that it may be *simpler* to allow these things. See the unresolved questions
section for details.


# Drawbacks

This could further complicate the already over-burdened *resolve* system. It may
also be confusing/surprising to see `use` used in this manner.



# Alternatives


Add an attribute for functions `#[renamed(from="old_name", since="...")]`. This should have the following behaviour:

* Any reference to the current or old_name should resolve to the current
* The old name can be silently shadowed, to avoid namespace pollution
* The function can only be implemented under one name (deny duplicates)
* Users of the old name should receive a `warn(renamed)`
* Multiple renames should be able to co-exist (though this would be really unfortunate)

In addition, the mythical rustfix could identify these attributes and automatically fix the
source of anything using the old names. The len_hint example would then be resolved as
follows.

```rust
    /// Returns a lower and upper bound on the remaining length of the iterator.
    ///
    /// An upper bound of `None` means either there is no known upper bound, or
    /// the upper bound does not fit within a `usize`.
    #[inline]
    #[stable(feature = "rust1", since = "1.0.0")]
    #[renamed(from = "size_hint", since = "1.2.0")]
    fn len_hint(&self) -> (usize, Option<usize>) { (0, None) }
```

Now everyone who implements or consumes `size_hint` receives a `warn(renamed)`.
A trivial grep party (or ideally rustfix), updates everything to use `len_hint`,
and the warnings go away.

This design was originally proposed by this RFC, but was generally frowned upon
as "too specific".



# Unresolved questions

Should it be possible to import items from different blocks? That is, should it be
possible to do:

```rust
impl<T> Default for Vec<T> {
    fn default() -> Self { .. }
}

impl<T> Vec<T> {
    pub use Self::default as new;
}
```

or


```rust
impl Foo {
    fn foo(&self) { .. }
}

impl Foo {
    pub use Self::foo as bar;
}
```


-----

Should it be possible to implement a trait by just using items from elsewhere that
have the right signature? e.g.

```
impl<T> Default for Vec<T> {
    pub use Self::new as default;
}
```