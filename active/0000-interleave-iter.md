- Start Date: 2014-07-23
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Adding an interleaving iterator to the set of core iterator adaptors. 
That is ```a_iter.interleave(b_iter)``` yields ```a1,b1,a2,b2,...``` until one terminates.

# Motivation

There's no easy way to accomplish this with the current set of iterator adaptors, 
and there are enough subtle corner cases that I wouldn't expect someone to
trivially implement it correctly when they need it.

Desirable for when something expects a single iterator, but you want to 
interleave the contents of multiple iterators. For instance, if you wanted
to create a stream of loosely unsorted data, you could do something like 
```sorted.interleave(sorted.rev())```

# Detailed design

I've actually fully implemented this [in this pull request](https://github.com/rust-lang/rust/pull/15886),
this is a retroactive RFC to see if this is desirable to have in libcore at all.

# Drawbacks

More code, more api. Otherwise only affects itself. Fully backcompat. 

# Alternatives

No real alternatives, it simply is or isn't available.

# Unresolved questions

It's possible to have interleave *not* terminate when any one of the source iterators terminates, 
but rather just output the rest of the remaining iterator. This would increase complexity, and is not
clearly desirable. In this vein, Interleave could potentially be augmented to provide all kinds of combinations of
behaviour, such as offsetting the splicing of the iterators ```(a1, a2, a3, b1, a4, b2, a5, b3, ...)``` 
or unbalanced usage of the sources ```(a1, a2, b1, a3, a4, b2...)```.

Would we want any of these?
