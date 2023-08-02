- Feature Name: `markup_macro`
- Start Date: 2023-08-01
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

The `markup!` and `markup_type!` macros allow constructing arbitrary node types, where each node contains arbitrary attributes and zero or more child nodes.

# Motivation
[motivation]: #motivation

Applications may use node hierarchies as a fundamental way of describing graphics. Without the `markup!` macro, the developer must manually construct a node by chaining methods that are usually prefixed by `set_` and call a chaining `append_children` method.

A procedural macro is limited for cases where the node type provides common `set_` prefixed methods for all node kinds, but where the node type does not provide methods that are very specific to a node kind.

For example, consider a `Node` type and a kind `Button`:

- `Arc<Button>` has a method `set_warning`
- `Node` has no method `set_warning`
- `Button::new(|btn| btn)` returns `Node`
- `Node` does have a `to::<K>()` conversion method

This cannot be expressed with a procedural macro unless the identifier in the markup macro constructed via a procedural macro performs a string comparison from the identifier to the name of the node kind, which puts the following limitation:

- The markup will ignore node types from the lexical scope. For example, the `Button` tag always constructs a speficic type, not the type at the lexical scope.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `markup!` macro takes elements of the form:

```plain
<Node/>
<Node></Node>
{node}
{iterable}
```

The angle brackets forms are similiar to XML, accepting attributes with interpolations, where `Node` must be a type defined by the macro `markup_type!`.

The `markup_type!` macro is used to define a node type can be constructed by the `markup!` macro. Besides that, this macro describes how children are added, which attributes are valid, which is the type of each attribute and to which expression the attribute assignment translates.

The following program defines a `Node` type that can be constructed via `markup!`:

```rust
markup_type! {
    pub struct Node;

    // constructs the node in an initial state.
    fn new() -> Node {
        Node
    }

    // the `append_child` function describes
    // how the contained nodes are added into the collection.
    fn append_child(parent: Node, child: Node) {
        // append child here.
    }

    // define the attribute `some_attr`
    fn some_attr(node: Node, value: u64) {
        // set attribute here
    }
}

let node = markup!(
    <Node>
        <Node some_attr={15}/>
    </Node>
);
```

### Return

The `markup!` macro returns either a single node or a `Vec` of nodes if it contains more than one node at the top.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `markup!` macro takes a sequence of items: XML-based tags and interpolations:

- An interpolation accepts a node or an iterable. If it evaluates to an iterable, then that iterable contributes all of its items to the enclosing tag or to the top `markup!`.
- A XML-based tag consists of a type, optional attributes (supporting interpolation) and optional children. It is an error if the type is not defined by `markup_type!`. For each attribute, based in `markup_type!`, validate if it exists and evaluate its assignment. For each child, evaluate it and call `append_child` from `markup_type!`.

The `markup_type!` macro puts no retriction about which type is returned by `new`; that is, `new` may return a different type from the enclosing `markup_type!`'s type. For example, `Button` may be construct via `new` resulting into a `Node`.

# Drawbacks
[drawbacks]: #drawbacks

This adds two macros to the standard library which must be implemented at the language-level due to how attributes are translated into expressions.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- The `markup_type!` macro allows describing attributes of arbitrary name whose type errors are caught at compile-time. A trait could not be used instead.

# Prior art
[prior-art]: #prior-art

N/A

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Generic nodes.

# Future possibilities
[future-possibilities]: #future-possibilities

Nothing yet.
