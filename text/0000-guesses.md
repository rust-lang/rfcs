- Feature Name: guess_diagnostic
- Start Date: 2017-02-23
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add a new kind of compiler diagnostic ("guess"), to distinguish "suggestions"
from code snippets that are found by a heuristic or have multiple possible
solutions. Change all current "suggestions" which are not guaranteed to be correct
to "guesses".

# Motivation
[motivation]: #motivation

TLDR: `rustfix` should be the semantic companion to `rustfmt` (which is purely syntactical) and automatically change code to be more idiomatic.

Clippy (and some compiler builtin lints) produce "suggestions", which are code
snippets that can be copy pasted over a part of the linted code to replace,
enhance or remove the detected suboptimal piece of code. In some cases there is
a clearly correct solution, like removing one ampersand on the rhs of the following
let binding (due to it being automatically dereferenced by the compiler anyway):

```rust
let x: &i32 = &&5;
```

This would be a situation where a "suggestion" would be used, since there is no
possible ambiguity or disadvantage to having a tool (e.g. `rustfix`) automatically
remove the ampersand.

When there is no clear solution (e.g. when the user references a type `Iter`,
but there is no such type in the scope) heuristics can supply "guesses" (e.g.
right now, the compiler supplies an unstructured list of types called `Iter`).

# Detailed design
[design]: #detailed-design

## Diagnostics frontend API changes

The compiler diagnostics API is extended with the following methods:

```rust
pub fn span_guesses<I>(&mut self, sp: Span, msg: &str, guesses: I) -> &mut Self where I: IntoIterator<Item = String>;
pub fn span_guess(&mut self, sp: Span, msg: &str, guess: String) -> &mut Self {
    self.span_guesses(sp, msg, ::std::iter::once(guess))
}
```

`span_guess` is just a convenience method which forwards to `span_guesses`.

`span_guesses` takes an iterator of guesses to be presented to the user.
The span `sp` is the region of code which will be replaced by one of the code snippets
given by the `guesses` iterator. The span can be zero bytes long, if the text is supposed
to be inserted (e.g. adding a new `use xyz::abc;` at the top of the file) instead of
replacing existing code.

If multispan replacements (replacing multiple separate spans at once, e.g. when modifying the
pattern in a for loop together with the object being iterated over) are desired,
one should modify the `Diagnostic` object manually. This might be inconvenient, but
multispan replacements are very rare (occurring exactly
[twice in clippy](https://github.com/Manishearth/rust-clippy/search?utf8=%E2%9C%93&q=multispan_sugg))
and therefor the API should be figured out once/if they become more common.

## Changes in diagnostic API usage

### Replace `suggestions` with `guesses`

All current calls to `span_suggestion` are replaced with `span_guess` if 
their replacement is obtained by a heuristic. A new type of tests is added: "suggestion". Tests in the "suggestion" directory must be failing
with an error (or denied lint) and contain a suggestion (not required to
be emitted by the error or lint failing the test). Then the suggestion is
applied, and the test must now compile and run successfully.

### Change `help` messages containing code snippets to `guesses`

Whenever a `span_help` is passed a snippet or hand built expression,
it is changed into a `span_guess` or `span_guesses` if multiple helps
are generated in some form of a loop. If `span_guesses` is used, all
arbitrary limits on the number of displayed items is removed. This limit
is instead enforced by the command line diagnostic backend.

## Json diagnostics backend API changes

The json object gets a new field `guesses` which is an array of
`Guess` objects. Every `Guess` object contains an array of span +
snippet pairs stating which part of the code should be replaced
with what replacment. There is no need for all guesses to replace
the same piece of code or even require the same number of replacements.

## Command line diagnostics backend API changes

Currently suggestions are inserted directly into the error structure as a "help" sub-diagnostic
of the main error, with a special `Option` field set for the replacement text.
This means that backends like the existing command line backend and json backend will need to
process the main error's sub-diagnostics to figure out how to treat the suggestion and extract
that information back out of the sub-diagnostic.

The backend API is changed to add a `code_hints` field to the main diagnostic, which contains
an enum with a `Suggestion` and a `Guesses` variant. The actual backend will then destructure
these variants and apply them as they see fit best. An implementation of this backend change
exists at https://github.com/rust-lang/rust/pull/39973 (does not implement guesses yet).

The command line backend will print guesses as a sequence of "help" messages, just like in the following example

```
error[E0412]: type name `Iter` is undefined or not in scope
 --> <anon>:4:12
  |
4 |     let t: Iter;
  |            ^^^^ undefined or not in scope
  |
  = help: you can import several candidates into scope (`use ...;`):
  = help:   `std::collections::binary_heap::Iter`
  = help:   `std::collections::btree_map::Iter`
  = help:   `std::collections::btree_set::Iter`
  = help:   `std::collections::hash_map::Iter`
  = help:   and 8 other candidates
```

With the only change being that the `use ...;` is directly inserted into the help messages as shown below.
This has precedent in the   r

```
error[E0412]: type name `Iter` is undefined or not in scope
 --> <anon>:4:12
  |
4 |     let t: Iter;
  |            ^^^^ undefined or not in scope
  |
  = help: you can import several candidates into scope:
  = help:   `use std::collections::binary_heap::Iter;`
  = help:   `use std::collections::btree_map::Iter;`
  = help:   `use std::collections::btree_set::Iter;`
  = help:   `use std::collections::hash_map::Iter;`
  = help:   and 8 other candidates
```

The actual span where the guess should be inserted is not shown in the command line diagnostics and has
no influence on the display of the guess.

### Special case: Only one guess

In case there is only a single guess, there are multiple variants on what can happen.

* replacement contains `\n`
    * display in an extra `help`
* No `note` attached to the main diagnostic
    * span of the replacement is the same as the main diagnostic
        * attach the replacement string to the guess message
        ```
        42 |    abc
           |    ^^^ did you mean `abcd`
        ```
    * span of the replacement is part of the main diagnostic's span
        * attach the replacement string to the guess message, even if it's technically wrong
        ```
        42 |    Opition<u8>
           |    ^^^^^^^^^^^ did you mean `Option`
    * span of the replacement contains the main diagnostic's span
        * expand the displayed span to the entire span which should be replaced
        ```
        42 |    a.I
           |    ^^^ did you mean `a::I`
        ```
* `note` already attched to the main diagnostic
    * Attach the guess below the note according tho the no-note rules (as if the note were the main diagnostic)
    ```
    5 |     ignore(|z| if false { &y } else { z });
      |            ^^^             - `y` is borrowed here
      |            |
      |            may outlive borrowed value `y`
      |            did you mean `move |z|`
    ```

# How We Teach This
[how-we-teach-this]: #how-we-teach-this



What names and terminology work best for these concepts and why?
How is this idea best presented—as a continuation of existing Rust patterns, or as a wholly new one?

Would the acceptance of this proposal change how Rust is taught to new users at any level?
How should this feature be introduced and taught to existing Rust users?

What additions or changes to the Rust Reference, _The Rust Programming Language_, and/or _Rust by Example_ does it entail?

# Drawbacks
[drawbacks]: #drawbacks

## The new "guess" diagnostic category does not add any benefit

Citing @nrc in https://github.com/rust-lang/rust/pull/39458#issuecomment-277898885

> My work flow for 'automatic application' is that the user opts in to doing this
> in an IDE or rustfix tool, possibly with some further input.
> I don't think there is a need or expectation that after such an application
> the resulting code is guaranteed to be logically correct or even to compile.
> So, I think a suggestion with placeholders would be fine as would a suggestion with multiple choices.
> In other words, I'd rather make suggestions more flexible than add another category of error.

## JSON and plain text error messages might differ

Plain text will output many simple guesses as notes on the relevant span (e.g. "did you mean `abcd`")
instead of showing them as full suggestions like:

```
42 |    abc
   |    ^^^ unknown identifier
help: did you mean `abcd`?
   |
42 |    abcd
```

It would be confusing for users to get semantically differend kinds of messages depending on how they view the errors.


# Alternatives
[alternatives]: #alternatives

## Don't add a new category, just make everything a suggestion

This will make it impossible for automatic semantic code rewriting.

## Don't add a new category, but allow adding multiple suggestions 

This allows changing "help" messages like "did you mean ..." to suggestions, if there are multiple things that apply.

## Also apply all single-guess rules to suggestions to reduce error verbosity

This can be done as a later step to improve suggestions

# Unresolved questions
[unresolved]: #unresolved-questions

1. Test whether all compiler suggestions are correct by applying them to the run-pass tests (or a new test folder) and checking whether the code still works.
