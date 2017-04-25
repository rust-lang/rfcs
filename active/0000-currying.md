- Start Date: 2014-08-05
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Add automatic currying to all Rust functions/closures. This means that a closure like
`|a: A, b: B| -> C` would be transformed to something like `|a: A| -> |b: B| -> C`.
This should be optimized out in all cases where we just call the function/closure.
The only performance cost should be when we return a closure.

# Motivation

This should be a win in ergonomics when doing functional style of programming.
You get something like partial application when all functions are curried.
Thus, you can write `fn add(a: int, b: int) -> int { a + b }` and call it with `add(1)`
and get `|b: int| -> (b + 1)` back. This means you can get closures easily without
having heavy closure return types. This pattern allows you to code without having all 
of the data at the same time, but not having to write closures all over.

# Detailed design

A function called with no arguments remaining in its signature is called immediately. 
A function called with some arguments remaining is turned into a closure.
`a -> b -> c` is right-associative, equivalent to `a -> (b -> c)`

# Drawbacks

This effectively precludes default arguments if done in the familiar order.
If currying is done backwards (from the last argument to the first), then they are compatible.
But this betrays the expectations of functional programmers.
Currying also complicates overloading.

However, neither of those things are in Rust or necessary. The way to avoid both of these 
issues is to support optional keyword arguments that are specified before positional arguments.
That way you can overload `fn concat(&self, separator => sep: &str)` and `fn concat(&self)`
knowing that when you call `foo.concat()` you're already passing `foo` and it will execute. 
When you write `str::StrVector::concat(separator => ";")` you know you're getting a closure.
Similarly, `foo.concat(separator => ";")` also executes because you're passing a foo in.

So while this restricts some of the possibilities, it is still possible to add some overloading.
Most calls in Rust are to methods, which means you're passing a positional `self` argument.
Keyword-based overloading only fails to curry things like `Vec::new` which is a static method.

But the question is whether currying should be made in a way that's convenient for currying
or whether it should be adapted in some backwards way to be convenient to optional args.

# Alternatives

The alternative is to curry only some functions selectively. But this makes currying much more
of a pain to use and doesn't have the nice property that all functions take 1 or 0 arguments.

# Unresolved questions

By committing to currying and choosing which order the arguments are passed in, default arguments
could be pre-emptively blocked from appearing in Rust. Should this feature be implemented first 
or should Rust decide whether it wants keyword args or default args?

Also, can currying be implemented in such a way that there is no performance overhead in most 
simple cases?

Can there be possible type errors of the kind `do_stuff();` where `do_stuff` actually expects an 
argument? Of course, this can be linted easily (this closure doesn't get assigned to anything).
