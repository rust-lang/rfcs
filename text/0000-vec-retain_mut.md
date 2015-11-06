- Feature Name: retain_mut
- Start Date: 2015-11-6
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add a `retain_mut` method to `Vec` and `VecDeque` which allows elements to be
mutated before deciding whether to retain them.

# Motivation
[motivation]: #motivation

A common pattern with vectors is to look at each element, modify it and
remove it from the vector if a certain condition is met. The `retain` method
allows elements to be removed from a vector in-place but does not allow existing
elements to be modified.

The lack of a `retain_mut` method means that users have to do 2 passes, one with
`iter_mut` and one with `retain`. The resulting code is harder to read and
slower. Another, more efficient, way of doing this would be to work with vector
indices directly and swap elements into place while mutating them, similar to
what `retain` already does.

# Detailed design
[design]: #detailed-design

The only difference between `retain_mut` and `retain` is that `retain_mut` takes
a `FnMut(&mut T)` parameter instead of a `FnMut(&T)`. This allows elements to
be mutated before deciding whether to retain them.

The new `retain_mut` method will be added to both `Vec` and `VecDeque`.

Here is an example which will decrement each element of a vector and filter out
elements that have reached a value of zero.

    let mut vec = vec![7, 1, 3, 10];
    vec.retain_mut(|x| {
        *x -= 1;
        *x != 0
    });
    assert_eq!(vec, [6, 2, 9]);

# Drawbacks
[drawbacks]: #drawbacks

The `retain` method really should have had a `FnMut(&mut T)` parameter from the
start, but it is too late to change that. Adding `retain_mut` will result in
two methods that have almost identical implementations.

# Alternatives
[alternatives]: #alternatives

Changing the existing `retain` method to take a `FnMut(&mut T)` was considered
in rust-lang/rust#25477 but this is likely to break a lot of existing code which
passes a closure defined using `|&x| {...}`.

Another alternative is to not do anything. Users can implement their own version
of `retain_mut` or they can restructure their code into an `iter_mut` pass on
the vector followed by a `retain` pass.

# Unresolved questions
[unresolved]: #unresolved-questions

None
