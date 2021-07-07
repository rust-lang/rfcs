- Feature Name: Scope Prints In Diagnostics
- Start Date: -
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

The diagnostics messages emitted by the Rust compiler make a very good effort
for quoting relevent pieces of code when emitting about warnings and errors.
Attached to these are annotated fragments of code, with filenames and line
numbers.

However, the context in which the fragments reside is not mentioned, i.e.
whether a function or another item. This proposal calls for adding that as
well.


# Motivation
[motivation]: #motivation

In large source files where the number of lines goes to the thousands, it is
easy to lose track. If the code is slightly repetitive, the annotations may not
be enough to provide proper context.

Some developers use development environments where they look at warnings or
errors emitted in console and only then go to the source. Having the context
appear with the diagonstics may be beneficial to them.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The changes proposed are two-fold:

### Item path next to line number

For each source position on which we already mention file name, line number and
column, an item path is printed along with the type of the item. Two spaces
separate it from the line number so that the line number remains apparent.

For example, the following will be printed:

```
  --> main.rs:14:13  in fn main
```

Instead of:

```
  --> main.rs:14:13`
```

### Context lines

To the source annotations, lines mentioning the context are added.

```
warning: unused variable: `y`
  --> test.rs:13:13  in fn foo::bar
   |
7  | mod foo {
...
12 |     pub fn bar() {
13 |         let y = 1;
   |             ^ help: consider prefixing with an underscore: `_y`
   |
   = note: `#[warn(unused_variables)]` on by default
```

To prevent overloading the diagnostics with repeated information regarding
contexts having multiple errors in them, each context line will be mentioned
only once for that purpose.  For example:

```
warning: unused variable: `y`
  --> test.rs:13:13  in fn foo::bar
   |
7  | mod foo {
...
12 |     pub fn bar() {
13 |         let y = 1;
   |             ^ help: consider prefixing with an underscore: `_y`
   |
   = note: `#[warn(unused_variables)]` on by default

warning: unused variable: `x`
  --> test.rs:14:13  in fn foo::bar
   |
14 |         let x = 1;
   |             ^ help: consider prefixing with an underscore: `_x`
```


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The information for the line is gathered by a mechanism to resolve a `Span` to
the corresponding named scope, by using a Trait dynamic call from
`librustc_error` back to `libsyntax`, in addition to what is done with
`CodeMap`. The call does a quick DFS into the AST for the `Span` and collects
the relevant `Ident` path, so it should handle nested scopes properly.

# Drawbacks
[drawbacks]: #drawbacks

We may not want to do this because to prevent unnecessary noise in
diagnositics. AFAIK, there were no loud complaints about this information
missing thus far.

# Rationale and alternatives
[alternatives]: #alternatives

Alternatively, we can choose not to this. However we would rely on IDEs
being smart enough to assist developers in navigating through warnings and
errors (i.e the common jump-to-source functionality).

# Prior art
[prior-art]: #prior-art

Other compilers do this. For example, `gcc` emits `In function 'foo':`.  Git
diff adds context to unidiff hunk header information to increase readability.
Clang does not do this, neither does Python.  Java (OpenJDK) does so to some
degree (`error: variable y is already defined in method x()`).

# Unresolved questions
[unresolved]: #unresolved-questions

- Should we avoid adding the extra context lines altogether?
- Should we avoid adding the item path next to the line number?
- Should we handle more scoped items such as loop labels, and closures? What
  would be their path strings?
