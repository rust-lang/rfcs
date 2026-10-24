- Feature Name: `diagnostic_blocking`
- Start Date: 2024-05-17
- RFC PR: [rust-lang/rfcs#3639](https://github.com/rust-lang/rfcs/pull/3639)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

`#[diagnostic::blocking]` is a marker attribute for functions that are considered (by their author) to be a blocking operation, and as such shouldn't be invoked from an `async` function. `rustc`, `clippy` and other tools can use this signal to lint against calling these functions in `async` contexts.

# Motivation
[motivation]: #motivation

Calling blocking operations in `async` functions is a common, well publicized foot-gun, but we don't provide nearly enough automated checks against it currently. Adding this annotation will allow `rustc` and `clippy` to detect the "trivial" cases (direct call of blocking operation within an `async fn`), while leaving the door open for future exhaustive analysis by these or other tools (perform DFS exploration of the crate's call graph looking for *transitive* calls to blocking operations invoked from `async` functions). Making this an annotation that users can add to their own APIs greatly adds to the utility of this linting, expanding coverage significantly.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`#[diagnostic::blocking]` is an attribute that can annotate `fn` items. It signals to the compiler and other tools that the annotated function can perform blocking operations, making it unsuitable to be called from `async` contexts.

The Rust compiler/clippy will do a best effort analysis to lint against uses of blocking operations from `async` contexts, relying on this annotation, but it is not assured it will.

```
warning: async function `foo` can block
  --> $DIR/blocking-calls-in-async.rs:28:1
   |
28 | async fn foo() {
   | --------------
29 |     interesting();
   |     ^^^^^^^^^^^^^`foo` is determined to block because it calls into `interesting`
   |
note: `interesting` is considered to be blocking because it was explicitly marked as such
  --> $DIR/blocking-calls-in-async.rs:5:1
   |
5  | #[diagnostic::blocking]
   | ^^^^^^^^^^^^^^^^^^^^^^^
6  | fn interesting() {}
   | ----------------
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The barest version of this feature is the addition of the `blocking` attribute to the `diagnostic` namespace, *and nothing else*, with no behavioral change. This would allow clippy and other tools to use the mark for their own analysis.

The next step up is for `rustc` to incorporate a lint for direct calls. This would be trivial to implement.

The final version would be to implement a call-graph analyisis lint so that transitive calls to blockign operations also produce a warning, but this is explicitly out of scope of this RFC.

# Drawbacks
[drawbacks]: #drawbacks

There is a risk crate authors will *over*annotate their APIs with `#[diagnostic::blocking]` to get around temporary lacks in the analysis. Crate authors might *also* overannotate their APIs as a way of discouraging calls from `async` scopes, even if they are not blocking.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The use of the `#[diagnostic]` namespace has a few benefits:

 - Any library with an MSRV >= 1.78 can safely add the annotation to their APIs without concerns about their users seeing warnings or errors 
 - We don't pollute the top-level namespace with yet another marker 
 - The namespace is explicitly allowing for `rustc` to do *nothing* with this information. These `blocking` markers are useful even with a lack of tools using them, as they act as machine parseable documentation.
 - `clippy` (and `rustc`) can maintain a master list of paths to check for, but such lists are usually incomplete and always unwhieldy to maintain.
 - `clippy` could add their own attribute, but this is such a fundamental signal that I believe *should* live in the language and not in external tools.
 - We could add a `#[rustc_blocking]` attribute, but then third party crates would not be able to use these.

# Prior art
[prior-art]: #prior-art

`rustc` currently has a number of `#[rustc_*]` attributes used to influence errors and lints. The language also has attributes like `#[must_use]` which are similar in spirit to the proposed attribute. `clippy` also has explicit checks for type paths of both `std` and third party crates types in order to lint against specific invalid uses.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What parts of the design do you expect to resolve through the RFC process before this gets merged?
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?

# Future possibilities
[future-possibilities]: #future-possibilities

If `rustc` ever gains the ability to perform call-graph analysis, an exhaustive check of all `async` functions for transitive reachability of blocking operations would provide perfect visibility into issues.