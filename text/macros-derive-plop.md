- Feature Name: macros_derive_plop
- Start Date: 2018-03-05
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This feature would allow for macros to interact at a block level with surrounding code. However, not all blocks would be able to be interacted with. More specifically, during certain types of controle flows some blocks are only valid when attached to other blocks, these being `else if` and `else` specifically. The goal of this RFC is to allow for macros to "attach" themselves to these other blocks to complete the controle flow syntax.

# Motivation
[motivation]: #motivation

The motivation to do this comes from the desire for macros that look more like other blocks of code and for macros that can be optionally extended by external code.

The first is mostly subjective but if an optional part of the macro was instead defined in an following else block instead of some internal representation then it would be easier for the user or maintainers in the future to understand what was going on with the code.

The second is the most useful reason to implement this. The current model of macros requires that recursive macros must produce by themselves a fully qualified code block irrespective of where it was called, also the compiler also requires that the surounding code must be also syntactically complete as well.
If there were a few cases where these restrictions were able to be relaxed then it would be possible for macros to have very nice error handling or optional case handling.

Examples:

```rust
let handler = dbOpen!{
    ip = address,
    username = user,
    password = pass
} else {
    // handle the case where the connection fails to open
}
```

```rust
while! (expr {
    // do something
}) else {
    // if expr was never true
}
```

I have seen somethings like the above wanted in the language but with this RFC such things could be implemented by users instead of going through the RFC process

# Detailed design
[design]: #detailed-design

The design of this RFC is meant to work well with either the current `macro_rules!` system or the new Macros2.0 system. But would most likely work the best and look the most like rust if only applied to Macros2.0.

Proposing two new compiler traits, these being `PlopAhead` and `PlopBehind` where they mean the same sort of thing but on which side of the macro such connection is permitted.

To add them to a macro it would just `#derive(...)` them.

Example:

```rust
#derive(PlopAhead, PlopBehind)
macro foo {
    ...
}
```

To simplify the description the rest of the design witll be talked about in terms of `Plop` which is generic over either of the actual traits except for the following concerns:
* `PlopAhead` only allows attachment when the macro is ahead of other syntax. Or in other words only attaching to the syntax that follows the macro.
* `PlopBehind` only allows attachment when the macro is behind of other syntax. Or in other words only attaching to the syntax that preceeds the macro.
* When used in combination with one another attachment is allowed on both sides of the macro.

Even with these traits the requirement that macros must produce fully correct syntax is still present. Macros cannot create dangling blocks or other such things. It does not permit the use of Plop-ing while expanding the macro.

So what does Ploping allow, it allows for the syntax around a macro to not necessarily be fully legal if the macro was not present. The easiest example would be an `else` statement following a macro. Generally (ie, currently), this is not allowed since an else block is meaningless without a corresponding `if` block. However, if a macro derives `PlopAhead` and after expanding ends with an `if` block then a following `else` would be able to become the `else` for that produced `if`.

The reason for having two is so that macro creators can more finely control how a macro is used, and so that they don't have to worry about the case which they don't explicitly opt-into. 

For `PlopBehind` such a macro would be able to attach to even a previous keyword. The main example of this would be starting the production with an `if` statement and this would be allowed to then attach to a daginling `else` to form an `else if`.

This would only be allowed to connect to `if`, `else if`, or `else` statments as they are currently the only blocks that this sort of connection would make sense.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

The reason that I chose `plop` is that the code is connection to other parts of the code and it is a fun word that describes to a reasonable degree of correctness what is happening.

This is a continuation of existing rust patterns of macros because it allows for more finely controlled use of meta programming. In a sense we are meta programming the meta programming because the use can be extended beyond the macro definition.

I don't believe that this proposal should change how Rust is taught to new users because the creation of advanced macros is not really a beginner topic. However, when teaching macros it would makes sense to eventually teach it as it is would be a useful tool for some cases of macro creation.

To teach this feature to existing Rust users the book should be updated to explain the feature. I believe that this would be the best way since the book is a main goto reference of rust programing paradignms. It would also make sense for rust-by-example to get a few examples of how to use it.

In the sections to do with macros2.0 it would make sense to show that these traits exist and what they do. A couple of examples of how the macros using the traits can interact with other parts of the code.

An example to show off this feature could be:

```rust
while! {(expr) {
    // do something
}} else {
    // if expr was never true
}
```

# Drawbacks
[drawbacks]: #drawbacks

This should not be done because it might expose macros to misuse. It also could make parsing very much harder because syntax could only be rejected after parsing a macro that has these traits to make sure that the macro doesn't make the code correct.

# Alternatives
[alternatives]: #alternatives

1. Nothing could be done, the current macro system does work

# Unresolved questions
[unresolved]: #unresolved-questions

What parts of the design are still TBD?
