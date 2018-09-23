- Feature Name: cargo-editorconfig
- Start Date: 2018-09-23
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Create editorconfig file as part of cargo projects.

# Motivation
[motivation]: #motivation

[Editorconfig](https://editorconfig.org/) is a way to solve the old `tabs vs spaces` debate, and more. It allows to express and enforce a minimal style guide across the majority of development environments. This becomes especially important as soon as collaborative project work enters the picture.

Let's look at an example editorconfig configuration file:

```
# https://editorconfig.org
root = true

[*]
charset = utf-8
end_of_line = lf
indent_size = 4
indent_style = space
insert_final_newline = true
trim_trailing_whitespace = true
```

It defines that for all files matching the pattern `*` the following rules should apply. It defines the charset, line ending character, indentation size, indentation style, whether or not a and only one new line should be inserted at the of the file and whether or not trailing whitespaces at the end of lines should be trimmed. With the exception of the indent size and style these rules are hardly controversial. Concerning indent size and style, the rust book talks about it at the very beginning [here](https://doc.rust-lang.org/stable/book/second-edition/ch01-02-hello-world.html#anatomy-of-a-rust-program), suggesting indent size 4 and spaces.

Generally editorconfig is flexible, all rules can be specified and overwritten for patterns, using it privately and professionally for several years, I have yet to run into problems with it. The only issue is if the individuals development environment requires a plugin and the person doesn't know about it and or didn't care to install it. Yet if the file is completely ignored the situation is like before and for those that know about it or have their development environment pick it up automatically it helps avoid unnecessary issues.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Projects created with cargo, like `cargo new --bin` or `cargo new --lib`, create among other things, a file in the root of the project with the name `.editorconfig`, this file helps enforce a minimal style guide. For more information about editorconfig and how to use it visit the project homepage https://editorconfig.org.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The cargo tool creates a file in the project root of new projects named `.editorconfig` and fills it with the default conforming to the `rustfmt` style as seen in the above example.

Add cli option to cargo, to avoid creation of this file. Similar to how the `--vcs` option can be set to `none`.

# Drawbacks
[drawbacks]: #drawbacks

In the [rust internals discussion](https://internals.rust-lang.org/t/create-editorconfig-file-as-part-of-cargo-project/8411) 3 drawbacks were formulated.

- We already have `rustfmt` it already fills this role and does more
- Editors should follow `rustfmt` rules when editing `.rs` files
- More generated files can increase the knowledge burden

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### Why is this design the best in the space of possible designs?

Solid support across the ecosystem today.

### What other designs have been considered and what is the rationale for not choosing them?

Editorconfig can describe a minimal agreed upon subset of `rustfmt`. It should not be seen as a replacement for `rustfmt`, more as a foundation.

Looking at this [survey](https://insights.stackoverflow.com/survey/2017#technology-most-popular-developer-environments-by-occupation), out of the 20 most used development environments, 18 support editorconfig today, several out of the box without plugins. In contrast `rustfmt` currently only lists support for 5 of those editors, and comes with 0 out the box.

Another suggestion was that we should rely on editors doing the 'right' thing, when they edit `.rs` files. However even defining what the 'right' thing is tricky. Ideally it is a subset of `rustfmt`. Which subset? Further most editors allow customizing a default for things like `indent_size`, `insert_final_newline` and `trim_trailing_whitespace`. Expecting editors to ignore these user specified defaults is uncommon, and it is unlikely that editor project maintainers will do so any time soon. Even without user customization and only looking at the 3 aforementioned style options, significant inconsistencies occurred when trying this with 3 of the top 20 editors.

Ideally all editors would do the 'right' thing whenever they encounter rust files. That is not the case today. Should this ever become a reality it should be trivial to remove this file from future or even existing projects.

### What is the impact of not doing this?

Needless friction when doing collaborative work on rust projects.

# Prior art
[prior-art]: #prior-art

Here is an incomplete list of notable projects using editorconfig: https://github.com/editorconfig/editorconfig/wiki/Projects-Using-EditorConfig.

It seems there is no prior successful attempt to do this for a language ecosystem, however rust has the unique advantage that `indent_size` and `indent_style` are agreed upon, and recommended by the documentation.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

One question would be, if it makes sense to specify the os native `end_of_line`, or stick with one for all platforms. In my opinion the consistency out ways the correctness here.
