- Start Date: 2014-07-03
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

This RFC proposes to add optional parameters to Rust functions.

# Motivation

Currently in Rust there are a lot of functions that do the same thing, but take a different number of parameters.
The current design forces those functions to have different names.
This causes the standard library to have badly-named functions.

# Detailed design

Optional arguments are an implicit form of overloading.
Java has a very complicated overloading design that includes overloading by static types.
Overloading on types mixed with type inference might be very confusing.
However, overloading based on arity is very simple and clear.
Java also forces the writer of the function to write out a new function signature for every possible variant.
A nice syntax for optional parameters is preferable to the Java approach.

So `to_str_radix(&self, radix: uint) -> String` can be now written as `to_str(&self, ?(radix: uint))`
Since this is sugar for full overloading, a function declared with an optional parameter like this satisfies both 
```rust
pub trait ToStr {
    fn to_str_(&self) -> String;
}
```

and

```rust
pub trait ToStrRadix {
    fn to_str(&self, radix: uint) -> String;
}
```

traits.

Inside the body of the function, the arguments will be pattern matched something like this:

```rust
let rad = match args {
    [_, radix] => radix,
    _ => 10
};
```

This allows for default arguments or actually doing completely different in each case.

This will let Rust get rid of the sheer multitude of functions that only differ by a few parameters like
`concat` and `connect`; `split` and `splitn`. You can even go further and have a boolean indicating whether to
use terminator semantics and put that into `split`, eliminating `split_terminator`.
You could also have another boolean indicating whether it is reversed, eliminating `rsplitn`.

In this way, the library design is better, allowing auto-completion on `split`.
There are less names in the std library. Some naming problems go away.

The easiest design is to allow all trailing parameters to be optional.
That means that no mandatory parameter can come after an optional one.
This design still allows to simplify naming in the standard library before 1.0 ships and the names are set in stone.
Further refinements like trailing varargs, keyword arguments are possible in a backwards-compatible way.

# Drawbacks

Currently, you are able to write a function that differs by name to achieve the same effect.
This works for `slice_from` and `slice_to` and `slice`.
They are aptly named for what they do and can be easily autocompleted.
However, just one `slice` with two optional args (defaulting to 0 and the length of the string)
is much more elegant and doesn't clutter up the standard library with extra functions.

Of course, as I mentioned earlier, this also interacts with traits since now you satisfy two traits with one function.
This probably interacts with closures and/or lifetimes in some way as well.
So the correct standard library design must be weighed against adding yet another feature to Rust.

# Alternatives

Another proposal is to somehow represent optional arguments as some kind of an Option type.
The drawback of this proposal is that Option is a library type. It would have to be baked into the language instead.

Another alternative is to keep the full overloading syntax. This eliminates having to destructure the args array.
While this makes it a pain to rewrite all the possible variants, it's extremely explicit and clear.
If you want either one or three arguments only, it won't accept two.
For any function with k mandatory parameters and n total parameters the current proposal accepts all arities between k and n.

An alternative to trailing optional arguments is keyword arguments. This allows optional arguments in any place as long as
the following required arguments or following optional arguments are called by their keywords to resolve ambiguities.
This proposal has the downside of a more complicated argument resolution system (allowing some arguments to be called by position 
and some by keyword). It can also be implemented in a backwards-compatible way post 1.0 so it's not a 1.0 priority.

# Unresolved questions

If destructuring the arguments array is necessary, should there be some kind of a keyword for them.
Is there any way to implement it without using a keyword, other than Java-style overloading?

If varargs are included in the proposal, then the `main` function could be written as

```rust
fn main(...arguments: String)
```

which could be nice. But then again, using std::os::args() is not a huge problem.
