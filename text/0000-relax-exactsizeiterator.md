- Feature Name: relaxed_exact_size_iterator
- Start Date: 2015-03-02
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Replace the following requirement from `ExactSizeIterator`:

> `Iterator::size_hint` must return the exact size of the iterator.

With 

> `ExactSizeIterator::len` must return the exact size of the iterator. The default
> implementation of `ExactSizeIterator::len` assumes that `Iterator::size_hint`
> returns the exact size of the iterator. If this is not the case, you must
> provide your own `ExactSizeIterator::len` implementation.

# Motivation

This requirement is redundant and unnecessary because `ExactSizeIterator::len`
must already return the exact size of the iterator.  If you want the exact size
of an `ExactSizeIterator`, you should call `ExactSizeIterator::len()`

# Drawbacks

1. This requires (slightly) changing a stable API.

# Alternatives

We could also remove the default implementation of `len()` but that would break
quite a few libraries and force us to add many trivial `len()` implementations.

# Unresolved questions

None.
