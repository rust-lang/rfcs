- Feature Name: `proc_macro_attribute_recursion`
- Start Date: 2019-01-24
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Expand `proc_macro_attributes` recursively.

# Motivation
[motivation]: #motivation

Currently, procedural macros have no way to expand macros at all. [RFC #2320](https://github.com/rust-lang/rfcs/pull/2320) aims to rectify this, but despite being reworked a lot still suffers from some complexity.

This proc_macro author wants something workable now instead of waiting for that RFC while leaving the doors open for an eventual implementation. Also making this small part available allows us to collect experience with macro expansion in proc_macros at very modest cost.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`proc_macro_attributes` can add other macro invocations (in the form of bang-macros or attributes) in their output. The expander expands all macros in the proc_macro output recursively in order of appearance.

Here's an example from [flamer](https://crates.io/crates/flamer):

```rust
use flamer::flame;

macro_rules! macro_fun {
    () => {
        fn this_is_fun(x: u64) -> u64 {
            x + 1
        }
    }
}

#[flame]
mod fun {
    macro_fun!();
}
```

Flamer adds a clone of its own attribute to all `Macro` nodes it finds. In our example, it adds the attribute to `macro_fun!();` so we get `#[flame] macro_fun!();`.

The expander checks the output and, because it is from the original code, first expands `macro_fun!()` yielding

```rust
mod fun {
    #[flame]
    fn this_is_fun(x: u64) -> u64 {
        x + 1
    }
}
```

Because of the `#[flame]` attribute added during the first expansion of the outer `#[flame]`, this is fed back to flamer, which modifies the function resulting from the macro. Note that as in this example, the attribute needs not be placed at the same AST node (and in fact, flamer would place it only on macro invocation nodes).

This way, `proc_macro_attribute`s can be deemed *recursive* like macros-by-example. Note that the macro recursion limit must also be observed by the `proc_macro_attribute` implementations.

`proc_macro` writers can implement their macros in terms of `proc_macro_attributes` (which is a very roundabout way to deal with macros, but at least it would work at all) to gain the same benefits. The expansion logic could even be put into its own "expand" crate.

For example, a `strlen!` proc macro to calculate the string length could expand `strlen!(concat!("foo", "bar"))` into:

```rust
#[expand_bang(strlen)]
(concat!("foo", "bar"), );
```

Using the trick outlined above, this would then be expanded into:

```rust
#[expand_bang(strlen)]
("foobar", )
```

Afterwards, the proc_macro_attribute can reconstruct the original macro call:

```rust
strlen!("foobar")
```

Which will then be able to calculate the desired `6`. This RFC leaves the implementation of the `expand_bang` proc_macro_attribute as an exercise for the reader.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The expander is extended to search the expansion output of `proc_macro` and `proc_macro_attributes` for other macro invocations. Those are then expanded until there are no more attributes or macro invocations left or the macro expansion limit is reached, whichever comes first.

Implementors will have to make sure to order the expansions within expanded output by their origin: macros which are in the `proc_macro_attribute`s' input need to be expanded before expanding macros that have been added by the `proc_macro_attribute`s expansion themselves. This can easily be done by examining the `Span`s of the expansion and ordering them by `SyntaxContext` number.

This is necessary to avoid infinite loops, where a `proc_macro_attribute` calls itself without ever getting the expansion of its argument macro invocations.

# Drawbacks
[drawbacks]: #drawbacks

This is in theory a breaking change. However, the author deems it very unlikely that other `proc_macro_attribute` authors would introduce attrs into their expansions, except in the hope of triggering the expansion this RFC suggests, as those currently have zero functionality. In any event, a crater run shouldn't hurt.

# Rationale and alternatives
[alternatives]: #alternatives

* leave things as they are, but this leaves proc_macro authors in the cold if they want to deal with macros in invocations
* [RFC #2320](https://github.com/rust-lang/rfcs/pull/2320) has a more general solution but tackles more complexity. Note that this RFC is a part of #2320 broken out, so we can still implement the rest of it afterwards

# Unresolved questions
[unresolved]: #unresolved-questions

None
