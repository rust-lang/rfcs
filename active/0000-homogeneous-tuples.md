- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Add methods to allow homogeneous tuples (e.g., `(T, T, T)`) to be used
as slices (`&[T]`).

# Motivation

Homogeneous tuples and fixed-length arrays have the same memory
layout, but provide distinct capabilities. Tuples are particularly
useful because they can be split apart into their componenet parts:

    let (a, b, c) = tuple;
    
On the other hand, fixed-length arrays support indexing, which is
great for writing generic code:

    for i in range(0, fixed_length.len()) { ... fixed_length[i] ... }
    
I have found that in many cases I would prefer a tuple type, because I
am managing a tuple of distinct cases that I would like to be able to
destructor and reassign, but I would also sometimes like to use
indexing to avoid code duplication.

In this RFC, therefore, I augment tuple types with the ability to
support indexing and in general act as slices. This means that tuples
are a better choice than fixed-length arrays for those cases where one
intends to pull apart the tuple at some point.
    
# Detailed design

Add the following traits for tuples of sizes 2 to 16 whose component
type is `$T`:

```
pub trait $Tuple<$T> {
    /// Number of elements in the tuple.
    fn len(&self) -> uint;

    /// A slice pointing onto the tuple.
    fn as_slice<'a>(&'a self) -> &'a [$T];
    
    /// A mutable slice pointing onto the tuple.
    fn as_mut_slice<'a>(&'a mut self) -> &'a mut [$T];

    /// Iterate through the elements of the tuple.
    fn iter<'a>(&'a self) -> Items<'a, $T>;
    
    /// Iterate through the elements of the tuple.
    fn mut_iter<'a>(&'a mut self) -> MutItems<'a, $T>;
    
    /// Index into the tuple.
    fn get<'a>(&'a self, index: uint) -> &'a $T;
    
    /// Index mutably into the tuple.
    fn get_mut<'a>(&'a mut self, index: uint) -> &'a mut $T;
}
```

The implementation of `as_slice` and `as_mut_slice` is done by simply
creating a slice pair and transmuting. All other methods can be
derived by invoking the appropriate method on the slice type.

# Drawbacks

None of which I am aware.

# Alternatives

1. Make fixed-length arrays a supertype of homogeneous tuples.  More
   precisely, `(T_1, ..., T_n)` would be a subtype of `[U, ..n]` if
   `forall i. T_i <: U`. This is elegant but would be a deeper change
   for something that rarely comes up in practice. I am not sure of
   the full repercussions.
  
2. Make fixed-length patterns more-expressive so that they can easily
   support moves. In other words, people might write:
   
       let [a, b, c] = fixed_length;
       
   instead of
   
       let (a, b, c) = tuple;
       
   This may be a good idea, and we do need to put some effort post-DST
   into rationalizing vector patterns, but is basically independent
   from this RFC. That is, doing this RFC doesn't preclude us from
   improving vector patterns, nor does it make it any harder.

# Unresolved questions

None.
