- Start Date: 2014-05-22
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Macros defined with `macro_rules!` should have the ability to count how many repetitions a `$()`
sequence will expand to. There should also be a nonterminal that expands to the current iteration
number inside of a `$()` sequence.

# Motivation

The primary motivation for counting the number of repetitions is so `vec![]`, and other similar
macros, can preallocate enough space for the number of arguments it has. Which is to say,
`vec![1,2,3,4,5]` currently has to allocate twice (once with a capacity of 4, and then again with a
capacity of 8) even though the number of elements is known at compile-time.

The motivation for being able to get the current iteration number is to support counting macros,
such as `declare_special_idents_and_keywords!()` in `syntax::parse::token`.

# Detailed design

To support repetition counts, the `macro_rules!` parser will learn a new nonterminal syntax
`$#(..)`, which expands to an unsuffixed integer literal equal to the count of the repetitions of
the equivalent `$(..)*` sequence. This syntax is only used in the body and is illegal in the pattern
section of the macro. This nonterminal may contain the same tokens that `$(..)*` does, although
there is no practical benefit to using it with anything other than another nonterminal. For example,
`$#(foo: $arg)` is legal, but equivalent to `$#($arg)`.

To support iteration numbers, the nonterminal syntax `$(#)` expands to the iteration number of the
current sequence (as an unsuffixed integer literal starting at 0). This looks like the existing
nonterminal sequence syntax (minus the optional separator and trailing `*`/`+`), but a nonterminal
sequence must contain at least one syntax variable, and so using `$(#)` does not conflict with any
legal nonterminal sequence. Note that there is no trailing separator or `*`/`+` on this syntax.
Disambiguating this syntax from a normal nonterminal sequence can be done by looking ahead 2 tokens
after `$(`.

With this implemented, `vec![]` can change from its current definition to the following:

```rust
macro_rules! vec(
    ($(e:expr),*) => ({
        let mut _temp = ::std::vec::Vec::with_capacity($#($e));
        $(_temp.push($e);)*
        _temp
    });
    ($($e:expr),+,) => (vec!($($e),+))
)
```

# Drawbacks

This adds yet more complexity to the rather fragile `macro_rules!` API.

# Alternatives

The alternatives for counting repetitions are:

1. Create a public `count!()` macro that counts its arguments. The downside is this would only be
   able to accept expression arguments.
2. Fix macro hygiene and then create a private `count!()` macro for `vec![]`, and require that
   user-created macros that need this functionality reinvent the `count!()` macro. Fixing macro
   hygiene to allow this is rather problematic.
3. Come up with a specialized argument pattern for `vec![]` that isn't ambiguous with the expression
   arguments that allows for baking counting into the macro. Again, user-created macros that need
   this functionality would have to reinvent this pattern.

There is no general alternative for counting the iteration number. If expanding to a sequence of
expressions, you can probably get away with using a mutable variable that you increment each time,
but e.g. this won't work for creating statics. Instead you can provide the value to the macro by
hand, the way `declare_special_idents_and_keywords!()` does.

# Unresolved questions

Do we need to provide any way to get the current iteration number of a parent sequence? By that I
mean, if you have `$(foo: $($bar),*),*`, do we need to provide a way to get the outer sequence
number from within the inner `$($bar),*` sequence?
