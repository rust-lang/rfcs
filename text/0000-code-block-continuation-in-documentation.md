- Feature Name: code-block-continuation-in-documentation
- Start Date: 2020-04-01
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

The goal is to improve the experience of displaying complex examples in the documentation.

The currently proposed solution is to add `merge_next` and `merge_previous` attributes to code-block.

Example:

```rust
/// This is some documentation
///
/// ```merge_next
/// // A comment inside a code block
/// let some_code = 0;
/// ```
///
/// A line rendered as *regular documentation*
///
/// ```merge_previous
/// /// We can use variable declared in the first code-block
/// let other_code = some_code;
/// ```
```

… would be rendered as:

`<<<<<<<<<<<<<<<<<<<<<<<<<<<<<`

This is some documentation

```rust
// A comment inside a code block
let some_code = 0;
```

A line rendered as *regular documentation*

```rust
/// We can use variable declared in the first code-block
let other_code = some_code;
```

`>>>>>>>>>>>>>>>>>>>>>>>>>>>>>`

When running `cargo test`, and if the documentation generates a link to the playground, or a `run me` button, two snippets would be generated.

The first contains only the code of the first one.

```rust
// A comment inside a code block
let some_code = 0;
```

The second will contains the aggregate of both.

```rust
// A comment inside a code block
let some_code = 0;
/// We can use variable declared in the first code-block
let other_code = some_code;
```

# Motivation
[motivation]: #motivation

Currently, `cargo doc` generates a really nice documentation. However, if you have a complicated setup to explain, it can be quite difficult to express examples in a concise and maintainable way.

For example if you are working on a graph library, even a small example requires to create a graph. If you are documenting a function that have multiple use-cases it can become quickly an issue. Let's take a concrete example.

```rust
fn dijkstra(
    graph: &Graph,
    start: Graph::Node,
    exit_condition: &dyn Fn(Graph::Node, Cost) -> bool
    edge_cost: &dyn Fn(Graph::Node) -> Cost
) -> Hashmap<Graph::Node, Cost>;
```

The user can change the behavior of the function in many ways. As a library writer, we would like to give examples of the major use-cases. Each of those examples will need to instantiate a graph. Since the dijkstra function doesn't modify the graph, and since if the content isn't related to the examples themselves, we may want to share the set-up between all the example.

The rendered documentation we may want to create could look like this:

`<<<<<<<<<<<<<<<<<<<<<<<<<<<<<`

Function

```rust
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

```rust
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

```rust
let distances = dijkstra(
    graph,
    a,
    &|_node, _total_distance| -> false,
    &|_edge| -> 1,
);
```

## Early stopping

Stops the algorithm if a given number of nodes have been reached.

```rust
let distances = dijkstra(
    graph,
    a,
    &|_node, total_distance| -> total_distance > 3,
    &|_edge| -> 1,
);
```

… (more examples)


`>>>>>>>>>>>>>>>>>>>>>>>>>>>>>`

As you can see, and even if the setup is quite trivial, it takes quite a lot of lines to write. And even if is trivial, it is required for each examples to compile.

So how can we currently create such kind of documentation?

- we can duplicate the set-up for each code-block, and add a `#` before each line
- use a macro (declared in #[cfg(test)] mod tests { } modules in the sources, and prefixed by `#` in the documentation) at the beginning of each code-block to remove the duplication
- use a single code-block, and use comments inside the block

All of those options are far from optimal.

- Duplicating the set-up code means more maintenance.
- Creating a macro for such a trivial task seems strange (and the content of the macro would still need to be duplicated in the set-up part of the documentation).
- By having a single code-block, the documentation (especially the titles) will not be rendered nicely (as a comment instead of markdown).

Having a way to way to render documentation inside code-block inside documentation solves this dilemma.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When documenting a complex code-block, you can merge multiple block by adding `merge_next` to the first parts, and `merge_previous` to the next one. It can be done multiple time (to split a code-block in 3 or more parts). In between those code-block, any regular items can be used, including other code-blocks.

As shown in the summary, this example demonstrate how this feature can be used.

```rust
/// This is some documentation
///
/// ```merge_next
/// // A comment inside a code block
/// let some_code = 0;
/// ```
///
/// A line rendered as *regular documentation*
///
/// ```merge_previous
/// /// We can use variable declared in the first code-block
/// let other_code = some_code;
/// ```
```

You can use this feature to add some high level explanations in the middle of an code block. If you are familiar with [jupyter notebook](https://jupyter.org/) ([link to the rust kernel](https://github.com/google/evcxr/tree/master/evcxr_jupyter)), it is similar to use markdown block in between your executable code. They provide an easy way to have a nicely rendered multi-part explanation.

---

Both `merge_next` and `merge_previous` implies `rust`. As such they are imcompatible with tags like `text`. The tag `rust` can be used explicitely but isn't required.

---

Each code-block with a `merge_next` tag must have another `merge_previous` block before the end of the current documentation block, and vice versa.

You can't write …

```rust
/// ```merge_next
/// let some_statement = 0;
/// ```
/// No `merge_next` block will follow
fn foo();
```

… nor …

```rust
/// No `merge_previous` block before this one
/// ```merge_previous
/// let some_statement = 0;
/// ```
fn foo();
```

---

All code-blocks are independents, and therefore can have different tags (like `compile_fail` or `ignore`). This also means that a tag may need to be repeated multiple times (like `edition`).

```rust
/// ```merge_next,compile_fail
/// match "some_string" {
///     "some_string" => "true",
/// ```
///
/// The statement is split between two code-blocks. The first one will fail to
/// compile (but shouldn't be considered an error), so a `compile_fail` tag is
/// needed.
///
/// The expected use-case for this construction is gives some explanations in
/// the middle of a complex statement.
///
/// ```merge_previous
///     _ => "impossible",
/// ```
/// The second code-block doesn't need `compile_fail` since it will contains the
/// aggregate of both parts, and thus will generates a valid snippet.
```

---

Code block can be split in more than two parts.

```rust
/// ```merge_next
/// let part_0 = 0;
/// ```
/// A first line of documentation…
/// ```merge_previous,merge_next
/// let part_1 = part_0;
/// ```merge_previous
/// … and a second
/// let part_2a = part_0;
/// let part_2b = part_1;
/// ```
```

---

It is possible to have complex use case, like displaying other code-blocks (even
rust code even it shouldn't be recommended).

```rust
/// The following code-block is the first of a 3 part snippet.
///
/// ```merge_next
/// // A comment inside a code block
/// let from_first_block = 0;
/// ```
///
/// You can use any kind of documentation, like another code-block.
///
/// ```text
/// Some verbatim text
/// ```
///
/// The next code-block is the second part of the snippet. It need both to
/// explicitely be merged to the previous and the next one.
///
/// ```merge_previous,merge_next
/// /// We can use variable declared in the first code-block
/// let from_second_block = from_first_block;
/// ```
///
/// It is even possible to use regular rust code-block in between a multi-parts
/// snippet.
///
/// ```rust
/// let some_unrelated_rust_code = 1;
/// // You can't use `from_first_block` or `from_second_block` here.
/// ```
///
/// And finally the last part.
///
/// ```merge_previous
/// /// We can use variable declared in the first or second code-block
/// let in_third_block_a = from_first_block;
/// let in_third_block_b = from_second_block;
/// ```
///
/// The following multi-parts snippet isn't merged with the previous
///
/// ```merge_next
/// let in_part_4 = 0
/// // You can't use any of `from_first_block`, `from_second_block`,
/// // `in_third_block_a` and `in_third_block_b` here.
/// ```
///
/// And finally a last snippet
///
/// ```merge_previous
/// let in_part_5 = in_part_5;
/// ```
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The currently proposed solution is to add `merge_next` and `merge_previous` attributes to code-block.

A explained above, the following parts will have to be modified.

- When running `cargo test`, and if the documentation engine. generates a link to the playground, or a `run me` button, one snippet will be generated by code-block (like usual), but each subsequent snippet will also contains the previous ones.
- When running `cargo doc` (or similar tools), the generated html should be displayed using the normal markdown engine, as if the code-blocks had the `rust` attributes.

# Drawbacks
[drawbacks]: #drawbacks

It make things more complicated to parse, as explained in the section above.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Here is a list of possible alternative:

- Have a closing tag, instead of and opening tags to merge multiple code-blocks

```rust
/// ```
/// let some_code = 0;
/// ```pause
/// Some documentation
/// ```continue
/// let other_code = some_code;
/// ```
```

It is more natural, but may need more modifications in the parser itself. It would also make it non standard (see example 15 of the commonmark [documentation](https://spec.commonmark.org/0.28/#fenced-code-blocks)).

- Display unit tests in the documentation (possibly with an attribute to be able to opt-in).

I think both approach can be implemented, and they complement each other.

- Dupplicate the code (prefixed by `#`) in each subsequent code-block.

The amount of dupplication is obviously way too high.

- For each subsequent code-block, create a macro in `#[cfg(test)] mod tests { }` modules inside the regular source and prefix it by `#` in the code-blocks.

This isn't really user-friendly, hard to teach, and require duplication (the content of the macro must be duplicated in the documentation for the first code-block).

# Prior art
[prior-art]: #prior-art

This proposition allow to document your code a bit like what you would do with a [jupyter notebook](https://jupyter.org/) ([link to the rust kernel](https://github.com/google/evcxr/tree/master/evcxr_jupyter)). I personally think that jupyter are a nice way to illustrate how your code should be used. As far as I understand it enables [literate programming](https://en.wikipedia.org/wiki/Literate_programming).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None for the moment.

# Future possibilities
[future-possibilities]: #future-possibilities

- As explained above, and in addition/instead of the current proposition, I think it should be possible to render unit-tests (probably behind a `expand` button) in the documentation.

- If we ever support testing more languages than just `rust` (like a snippet in `C`), then `merge_next`/`merge_previous` should become compatible with more than just `rust`, and extend to any supported languages.
