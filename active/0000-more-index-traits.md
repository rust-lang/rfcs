- Start Date: 2014-07-02
- RFC PR #:
- Rust Issue #:

# Summary

[RFC #34](https://github.com/rust-lang/rfcs/pull/111) added two traits for
index-operator overloading: `Index` and `IndexMut`. However, some critical
functionality remains missing. Specifically, the ability to assign to an index
into a structure, the ability to move out of an index into a structure and the
ability to return by value from an index into a structure.

# Motivation

There are two distinct cases to handle: collections which can return references
to elements and those that cannot. For example, the `Vec` type can return
references to its elements, but `BitvSet` cannot as the boolean elements are
stored in a non-addressable compressed form. A cache is another example of a
non-addressable but indexable collection. References cannot be handed out
since cached values will be purged at arbitrary times, so it would probably
need to return a smart pointer such as `Rc` instead.

# Detailed design

RFC #34 defines two traits:
```rust
pub trait Index<E, R> {
    fn index<'a>(&'a self, element: &E) -> &'a R;
}

pub trait IndexMut<E, R> {
    fn index_mut<'a>(&'a mut self, element: &E) -> &'a mut R;
}
```

Two additional traits will be added: `IndexGet` and `IndexSet`. In addition,
the type of the second argument to the `index` and `index_mut` methods will
be changed from `&E` to `E`. This is not strictly necessary, but will make the
traits more flexible than they were before (E can always be set to `&'a Foo` to
match the old functionality). In addition, `index_set` *must* take its second
argument by value if it is to work with types like `HashMap`.
Here are the four `Index` traits:
```rust
pub trait Index<E, R> {
    fn index<'a>(&'a self, element: E) -> &'a R;
}

pub trait IndexMut<E, R>: Index<E, R> {
    fn index_mut<'a>(&'a mut self, element: E) -> &'a mut R;
}

pub trait IndexGet<E, R> {
    fn index_get(self, element: E) -> R;
}

pub trait IndexSet<E, R> {
    fn index_set(&mut self, element: E, value: R);
}
```

Note that `IndexGet` handles both the move and by-value use cases, as one can
implementing it for either `T` or `&T`. It's theoretically possible to remove
`Index` and `IndexMut` in favor of `IndexGet` but I feel that it
overcomplicates the implementation (especially as to how the compiler's
supposed to handle `&foo[idx]` and `&mut foo[idx]`.

# Drawbacks

This adds several new lang items and adds complexity to the language as a
whole.

# Alternatives

There was some discussion in the previous RFC about how index-set should be
implemented. One alternative mentioned would be to add `index_set` as a method
in `IndexMut` with a default implementation:
```rust
pub trait IndexMut<E, R> {
    fn index_mut<'a>(&mut self, element: &E) -> &'a mut V;

    fn index_set(&mut self, element: E, value: V) {
        *self.index_mut(&element) = value;
    }
}
```

However, this does not handle the by-value case. We can restructure and rename
the traits a bit and end up with this:
```rust
pub trait IndexRef<E, R> {
    fn index<'a>(&'a self, element: &E) -> &'a V;
}

pub trait IndexRefMut<E, R>: IndexRef<E, R> {
    fn index_mut<'a>(&mut self, element: &E) -> &'a mut V;

    fn index_set(&mut self, element: E, value: V) {
        *self.index_mut(&element) = value;
    }
}

pub trait IndexValue<E, R> {
    fn index(&self, element: &E) -> R;
}

pub trait IndexValueMut<E, R>: IndexValue<E, R> {
    fn index_set(&mut self, element: E, value: V);
}

pub trait IndexMove<E, R> {
    fn index_move(self, element: &E) -> R;
}
```

The compiler would forbid the implementation of both `IndexRef` and
`IndexValue` for the same type.

This is a bit more straightforward - there is a clear separation between the
by-ref collections and the by-value collections. It's also immediately clear
how one would implement a by-value set - the pattern of implementing a trait
with a method that takes `self` by value on `&T` has no parallel in the
standard library that I'm aware of at this time. On the other hand, it is a
larger set of traits. On the other hand, it is significantly less flexible.

# Unresolved questions

How should compound assignment be handled? For types that implement `IndexMut`,
we can easily expand `a[b] += c` to `*a.index_mut(b) += c`. The implementation
for types which only implement `IndexGet` and `IndexSet` is a bit more
complicated. One option is to simply expand to `a.index_set(b, a.index_get(b) +
c)`, but I'd imagine there are situations in which that is needlessly
inefficient. It's similar to the old desugaring of `a += b` to `a = a + b`
which was removed because of the inefficiency. It may be best to punt on this
question until the situation with compound assignment in general is worked out.
