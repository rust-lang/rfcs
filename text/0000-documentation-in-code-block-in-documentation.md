- Feature Name: documentation-in-code-block-in-documentation
- Start Date: 2020-04-01
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

The goal is to improve the experience of displaying complex examples in the documentation.

The currently proposed solution is to make triple forward slashes in code block in documentation displayed as normal documentation.

Example:

```
/// This is some documentation
///
/// ```
/// // A comment inside a code block
/// let some_code = 0;
///
/// /// A line rendered as *regular documentation* <--- This line
///
/// let other_code = some_code;
/// ```
```

… would be rendered as:

`------------------------------`

This is some documentation

``` rust
// A comment inside a code block
let some_code = 0;
```

A line rendered as *regular documentation* <--- This line

``` rust
let other_code = some_code;
```

`------------------------------`

# Motivation
[motivation]: #motivation

Currently, `cargo doc` generates a really nice documentation. However, if you have a complicated setup to explain, it can be quite difficult to express examples in a concise and maintainable way.

For example if you are working on a graph library, even a small example requires to create a graph. If you are documenting a function that have multiple use-cases it can become quickly an issue. Let's take a concrete example.

``` rust
fn dijkstra(
    graph: &Graph,
    start: Graph::Node,
    exit_condition: &dyn Fn(Graph::Node, Cost) -> bool
    edge_cost: &dyn Fn(Graph::Node) -> Cost
) -> Hashmap<Graph::Node, Cost>;
```

The user can change the behavior of the function in many ways. As a library writer, we would like to give examples of the major use-cases. Each of those examples will need to instantiate a graph. Since the dijkstra function doesn't modify the graph, and since if the content isn't related to the examples themselves, we may want to share the set-up between all the example.

The rendered documentation we may want to create could look like this:

`------------------------------`

Function

```
fn dijkstra(
    graph: &Graph,
    start: Graph::Node,
    exit_condition: &dyn Fn(Graph::Node, Cost) -> bool
    edge_cost: &dyn Fn(Graph::Node) -> Cost
) -> Hashmap<Graph::Node, Cost>;
```

---

# Examples

## Set-up

``` rust
use Graph;
use dijkstra;
use std::collections::HashMap;

let mut graph = Graph::new();
let a = graph.add_node();
let b = graph.add_node();
let c = graph.add_node();

// z will be in another connected component
let z = graph.add_node();

graph.extend_with_edges(&[
    (a, b),
    (b, c),
    (c, d),
    (d, a),
]);

// a ----> b           z (not connected)
// ^       |
// |       v
// d <---- c
```

## Basic usage

Compute the distances to all nodes in the graph from `a`.

``` rust
let distances = dijkstra(
    graph,
    a,
    &|_node, _total_distance| -> false,
    &|_edge| -> 1,
);
```

## Early stopping

Stops the algorithm if a given number of nodes have been reached.

``` rust
let distances = dijkstra(
    graph,
    a,
    &|_node, total_distance| -> total_distance > 3,
    &|_edge| -> 1,
);
```

… (more examples)


`------------------------------`

As you can see, and even if the setup is quite trivial, it takes quite a lot of lines to write. And even if is trivial, it is required for each examples to compile.

So how can we currently create such kind of documentation?

- we can duplicate the set-up for each code-block, and add a `#` before each line
- use a macro (prefixed by `#`) at the beginning of each code-block to remove the duplication
- use a single code-block, and use comments inside the block

Both of those options are far from optimal.

- Duplicating the set-up code means more maintenance.
- Creating a macro for such a trivial task seems strange (and the content of the macro would still need to be copied in the set-up part of the documentation).
- By having a single code-block, the documentation (especially the titles) will not be rendered nicely (as a comment instead of markdown).

Having a way to way to render documentation inside code-block inside documentation solves this dilemma.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When documenting a complex code-block, you can add text that will be rendered as regular documentation by repeating `///` a second time.

As shown in the summary, this example demonstrate a use of this feature.

```
/// This is some documentation
///
/// ```
/// // A comment inside a code block
/// let some_code = 0;
///
/// /// A line rendered as *regular documentation* <--- This line
///
/// let other_code = some_code;
/// ```
```

You can use documentation in code blocks in documentation (yes, it's a funny thing to say) to add some high level explanations in the middle of an code block. If you are familiar with [jupyter notebook](https://jupyter.org/) ([link to the rust kernel](https://github.com/google/evcxr/tree/master/evcxr_jupyter)), it is similar to use markdown block in between your executable code. They provide an easy way to have a nicely rendered multi-part explanation.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

When running `cargo test` or equivalent, the code is processed as we do currently (no changes).

---

When running `cargo doc` or equivalent, lines in documentation, inside a code-bloc should be displayed using the normal markdown engine, as if the code-block was closed above and re-opened bellow.

```
/// This is some documentation
///
/// ```
/// // A comment inside a code block
/// let some_code = 0;
///
/// /// A line rendered as *regular documentation* <--- This line
///
/// let other_code = some_code;
/// ```
```

… would be rendered as-if the following code was written.

```
/// This is some documentation
///
/// ```
/// // A comment inside a code block
/// let some_code = 0;
/// ```
/// A line rendered as *regular documentation* <--- This line
/// ```
///
/// let other_code = some_code;
/// ```
```

---

In case of multiples lines, they would be grouped together.

```
/// ```
/// let some_code = 0;
/// /// A first line of documentation
/// /// An a second
/// let other_code = some_code;
/// ```
```

… would be rendered as-if the following code was written.

```
/// ```
/// let some_code = 0;
/// ```
/// A first line of documentation
/// An a second
/// ```
/// let other_code = some_code;
/// ```
```

---

Code-blocks in documentation in code-blocks in documentation should probably create an error. Example:

``` rust
/// ```
/// /// ``` // <- this shouldn't be allowed
/// /// assert(foo == bar); // <- if it was allowed, does this line need to be valid rust?
/// /// ```
/// ```
```

Disabling code-block in documentation in code-block in documentation prevents many corner cases, like tests in code-block in documentation in code-block in documentation that would need to be runnable. It prevents some really creative usage, but I think it is safe to ignore them for the moment. If generating an error isn't a possibility, we could display a warning and display them as comments.

---

If the documentation engine provides a `run` button, each snipped should run as-if it contained the code from the begging of the code-block to the current part of the code-block.

``` rust
/// ```
/// let some_code = 0;
/// /// A first line of documentation…
/// let other_code = some_code;
/// /// … and a second
/// let third_part = other_code;
/// ```
```

… would be parsed as-if this was written

``` rust
/// ```
/// let some_code = 0;
/// ```
/// A first line of documentation…
/// ```
/// let other_code = some_code;
/// ```
/// … and a second
/// ```
/// let third_part = other_code;
/// ```
```

Witch would generated the following documentation:

`------------------------------`

```
let some_code = 0;
```

A first line of documentation…

```
let other_code = some_code;
```

…and a second

```
let third_part = other_code;
```

`------------------------------`

Witch contains 3 code snippets. If the documentation engine generate a `run` button, the code ran by the first snippet would be.

```
let some_code = 0;
```

The second.

```
let some_code = 0;
let other_code = some_code;
```

An the third

```
let some_code = 0;
let other_code = some_code;
let third_part = other_code;
```

Not having the code of the next one allow progressive discover of what the code is doing.

---

Since `cargo test` will run on the code-block, then the list snippet is guaranteed to be valid rust. However, inserting documentation in the middle of an expression will make intermediate snippet invalid, and is the responsibility of the author.

``` rust
/// ```
/// foo(3,
/// /// This documenation splits the function call in two
///     4);
/// ```
```

The rendered documentation would be fine.

``` rust
foo(3,
```
This documenation splits the function call in two
``` rust
    4);
```

But only the last snippet is guaranted to be valid rust.

``` rust
// first snippet, does not compile
foo(3,
```

``` rust
// last snippet, guaranted to compile
foo(3,
    4);
```

Since the last snippet will always compile, the reader of a broken documentation will always be able to access to a valid snippet. As such, I don't think we should do anything (at least for a first implementation) against that, given how complex I expect this corner case can be detected. The burden should be put on the documentation writter.

# Drawbacks
[drawbacks]: #drawbacks

It make things more complicated to parse, as explained in the section above.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Here is a list of possible alternative:

- Add an attribute (like "``` continue_previous") to merge two block of code.

``` rust
/// ```
/// let some_code = 0;
/// /// This proposition   // <--
/// let other_code = some_code;
/// ```
```

``` rust
/// ```
/// let some_code = 0;
/// ```                    // <--
/// The other proposition  // <--
/// ``` continue_previous  // <--
/// let other_code = some_code;
/// ```
```

I don't have a strong preference. The current proposition have the advantage to not have to change anything when running `cargo test`, and both possibilities share the same drawbacks.

- Display unit tests in the documentation (possibly with an attribute to be able to opt-in).

I think both approach can be implemented, and they compliment each other.

# Prior art
[prior-art]: #prior-art

This proposition allow to document your code a bit like what you would do with a [jupyter notebook](https://jupyter.org/) ([link to the rust kernel](https://github.com/google/evcxr/tree/master/evcxr_jupyter)). I personally think that jupyter are a nice way to illustrate how your code should be used.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Not for the moment. Alternative solutions are welcomes.

# Future possibilities
[future-possibilities]: #future-possibilities

As explained above, and in addition/instead of the current proposition, I think it should be possible to render unit-tests (probably behind a `expand` button) in the documentation.

As a possible extension of the current proposition, it should be possible to remove the limitation of using code-blocks inside documentation inside code-blocks inside documentation (for example to display a snippet in another programming language in the middle of a multi-part set-up). I don't think it is *that* important, witch explains why I think it is out of scope for this first version.
