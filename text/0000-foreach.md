- Feature Name: foreach
- Start Date: 16 April 2015
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add a `foreach()` method to `std::iter::Iterator`, which eagerly calls a function on
each item the Iterator contains and returns `()`.

# Motivation

By design, iterators are lazy, and will not be evaluated until they are consumed
in some way. The design of the `std::iter` library, and the language in general,
strongly encourages chaining Iterator method calls one after another. This idiom
is a very positive attribute of Rust: things that are hard to do by chaining
iterators are usually things that make code less comprehensible (e.g. it is
usually clearer code to `filter()` than to `continue` and to `take()` than to
`break`).

The `foreach()` method proposed by this RFC is fundamentally sugar; it does not
enable programmers to write any instruction not already possible with existing
language features. The most common recommendation is to write a `for` loop, but
it is also possible to chain one of several consuming iterator methods after a
map, to write a fold which returns `()`, or to add a dependency to the itertools
crate and use the foreach method defined there. A
[prior RFC](https://github.com/rust-lang/rfcs/pull/582) to the same effect was
closed without merging for these reasons.

However, if the inclusion of `foreach()` is syntactic sugar, then the exclusion of
`foreach()` is syntactic salt. Several consumers are currently 'blessed' by std as
special methods of Iterator, but no generic consumer method exists. For this
reason, anyone who needs to consume an iterator by some mean not blessed by std
needs to either depend on a crate whose tools are otherwise _more specific_,
rather than _more general_, or do something that feels unidiomatic and wrong.
Compare these three examples (the first being semantically different from the
others, and the last being enabled by this RFC):

```rust
let vec = my_collection.iter()
                       .filter_map(|x| { ... })
                       .take_while(|x| { ... })
                       .collect::<Vec<_>>()
```
```rust
let tmp = my_collection.iter()
                       .filter_map(|x| { ... })
                       .take_while(|x| { ... });
for x in tmp {
    tx.send(x).ok();
}
```
```rust
my_collection.iter()
             .filter_map(|x| { ... })
             .take_while(|x| { ... })
             .foreach(|x| { tx.send(x).ok(); });
```

Surely it is not intended that consuming an iterator by sending each element
across a thread boundary (or by any other means not blessed by std) seem
unidiomatic, but that is the effect of the current design. This is not about
making std 'batteries included', it is about defining the limits of idiomatic
Rust.

# Detailed design

Implementing this RFC is quite simple. Since this is, in implementation, just
sugar, it involves adding a very small predefined method to the
`std::iter::Iterator` trait, which would look something like this:

```
fn foreach<F>(&mut self, mut f: F) where F: FnMut(Self::Item) {
    for item in self { f(item); }
}
```

# Drawbacks

The main reason not to add this method is that it is sugar.

It could be argued that the Itertools crate implements `foreach()`, and that the
Itertools crate is not very popular, making this uncommonly used sugar. To this
there are several responses:
  * The common way to get around this salt is probably not to add a dependency,
    but to do the salty thing and write a `for` loop or add a `.all(|_| true)` and
    so on. Thus, the popularity of this method cannot be found by analyzing the
    popularity of the Itertools crate.
  * Even if this method were not popularly used (because the blessed consumers
    satisfy the majority of cases), that does not mean that a flexible method
    for consuming an iterator should not be made available by std; even if its
    use is a corner case, the blessed consumers are all degenerate
    implementations of foreach and the only reason to make available degenerate
    cases without the general case is if the general case is intended to be
    unidiomatic.

# Alternatives

### Do nothing

The main alternative is to do nothing.

### Implement `each()` or some other method with similar but distinct semantics.

Alternatively, Rust could have a similar method with different semantics, such
as a method like Ruby's `each()`, which is essentially an eager version of Rust's
`inspect`. I think maintaining the consistency of iterators return iterators
being lazy is valuable, and that this method should return `()`.

# Unresolved questions

Should this method be named `foreach()` or `for_each()`? I think of __foreach__ as
an existing PL keyword (used mainly by languages with non-iterative __for__
loops), so my preference is foreach(), but it doesn't much matter to me. It also
could be called `each()`, like Ruby's method, though its semantics are different.
