- Feature Name: some! macro
- Start Date: 2016-01-15
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

The `some!` macro intends to mimick the `try!` macro to be used for `Option<T>` types.
It offers safety while still preserving the brevity and readability of the `unwrap`
method that it intends to replace in most instances.

# Motivation
[motivation]: #motivation

As a rust newbie, something I see a lot of in documentation and tutorials is an excessive
use of the `unwrap` function. Many tutorials proclaim that this is to enable "brevity",
and indeed it does help with that. However, in many cases you could have equal brevity
and more demonstration of best practices with a macro to unwrap `Option<T>` 
values, while still preserving safe coding practices. 

Code should be written to be modular, but it also needs to remain safe. Rust 
should discourage the use of `unwrap` in all but the most trivial or targeted 
of cases by providing macros that achieve similar brevity with all the safety 
benefits of rust's error handling without `panic!`.

# Detailed design
[design]: #detailed-design

The design is simply to add the `some!` macro to extract a value from `Option<T>` types
if `Some<T>` is returned, or `return None` if `None` is returned. The exact 
implementation is very similar to the `try!` macro:

```
macro_rules! some {
    ($expr:expr) => (match $expr {
        Some(val) => val,
        None => return None,
    })
}
```

This code block would be put directly below the `try!` macro in `src/libcore/macros.rs`

## Other names

There are other possible names that I have thought of, but I have arrived at `some!`
because:
 - it makes linguistic sense. `let a = some!(operation(x, y, z));` means that
     you will be assigning `a` "some value" (i.e. not None)
 - it is short, and easy to remember because it is the same word as the `Some` struct
     with which it operates on.

However, it's similarity with the `Some` struct could cause some confusion, as you are
not getting a `Some<T>`, but are getting `T` (i.e. compare with `vec!` which gives you
a `Vec`). For this reason there are other possible names for the macro.

 - `value!`: this takes the negatives of `some!` and flip them on their head. It is
      the most linguistically correct of the options (for reasons highlighted above), 
      but "value" is such a common (and overrused) word in programming that I 
      would be reluctant to use it. However, one possible benefit of using it 
      here would be that it would discourage it's use for generic values, which 
      might actually end up making rust more readable!
 - `get!`: I've seen this one mentioned before, but I don't like it for the same
      reason that I don't like value -- it's meaning is so generic as to be
      meaningless, especially for someone first comming to the langauge.
 - `choose!`: I like this one because you can "choose an option", however I dislike
      it because it is too easy to confuse it with `match` or `select`. It's
      similarity to `select` especially makes it a no-go for me.
 
In the end, although `some!` isn't the most perfect linguistic option, it is the
most clear, and the most likely to not overlap with other common functions and
macros. This, combined with the fact that it is intuitave and easy to remember
make it the best option in my opinion.

# Drawbacks
[drawbacks]: #drawbacks

Adding new functions/macros to the stdlib of a lanuage, particularly for symbols that are
used automatically without import should also be done slowly and cautiously.

# Alternatives
[alternatives]: #alternatives

I am not aware of other design possibilities. Perhaps much of this will be resolved
with the `?` syntax? Whether `?` should automatically return `None` is a question
for that RFC, but some of those issues could be hashed out here. If `?` *does* work
with `Option` types, then I would think we would want a symetric macro to `try!`
as well.

# Unresolved questions
[unresolved]: #unresolved-questions

Nothing as far as I know -- the `try!` api is very good and works very well as far as I know.
This would bring the same functionality to Option types
