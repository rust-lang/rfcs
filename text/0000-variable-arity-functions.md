Feature Name: Variable Arity Functions
Start Date: 2016-04-19
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)
- RFC Issue: #1586

#### Related
RFC Issue #323 Is related, but covers Keyword Arguments, Default Arguments, and allegedly covers variable arity functions, but then also conflates it with variadic functions.

### Summary
Support [Variable Arity Functions](https://en.wikipedia.org/wiki/Arity#Variable_arity). Which are multiple functions that have the same name with a different, but explicitly defined and finite, number `n` of arguments. This not to be confused with [Variadic Functions](https://en.wikipedia.org/wiki/Variadic_function) (varargs), which typically support a range of `0..n` arguments, much like an array.

Elixir/Erlang is a good reference.

### Motivation
Variable Arity Functions provide enhanced ergonomics around library design. They allow sensible, although potentially complex, default values for function arguments. In the case that default values are also implemented, they allow for default values that rely on a runtime variable, or a computation too complex for a function signature.

### Detailed design
- A combination of both name and arity is used to determine which version of a function is called.
- This should be completed entirely at compile time with no runtime cost.
- The group of functions with the same name and varying arity must form a set (Mutually exclusive name+arity). This differs from RFC Issue#153, which also allows varying types.

This design does not permit any ambiguity regarding which function is dispatched. Furthermore, it avoids the complexity of using argument types to determine which function is dispatched, an approach that can subsume variable arity functions, but has it's own merits and drawbacks. The current design is a vast improvement compared to Java where type coercion can make it difficult to determine which function is dispatched.

For the remainder of this RFC, the notation `function_name/[number]` is used to indicate the function and arity (ex. `foo/3` is the function `foo` that takes 3 arguments).

#### Example Pseudo-Code
```
pub struct SceneGraph {
  tree: RoseTree<Node>,
  root: &Node,
}

impl SceneGraph {
    /// Adds a node to the scenegraph
    pub fn add_node(&mut self, node: Node) -> NodeIndex {
        self.tree.add_node(node)
    }
    pub fn add_node(&mut self) -> NodeIndex {
        self.add_node(Node::new())
    }

    /// Renders the scenegraph
    pub fn render(&mut self) {
        self.render(self.root)
    }
    pub fn render(&mut self, node: &Node) {
        self.tree.render(node)
    }
}
```

### Drawbacks
It's usage may not be worth the implementation cost and maintenance.
This could complicate virtual (runtime) dispatch with trait objects and their fat pointers.
It could increase the complexity of some future variadic function implementation.

### Alternatives
1. Use macros
2. Use multiple names for the different arity of functions
3. Use `Option` to create optional arguments

#### Alternative Drawbacks
1. Macros are more opaque to novice end users, and according to the book `These drawbacks make macros something of a "feature of last resort".`. Potentially complex implementations are moved from easier to test/comprehend code into macros. Finally, if macro's respect scoping rules, then the struct would need to leak "private information" to make the implementation possible, if it doesn't respect scoping rules, then one cannot rely on some of rust's powerful features to ensure code is implemented properly.

2. While this is the easiest alternative, it means that more API calls must remembered by the library user. It can also lead to alternative names that accidentally expose fragile implementation details.
ex.) `render/1,2` -> `render_entire_tree/1` & `render_node_and_children/2`
But if some form of dirty checking is implemented, then `render_entire_tree/1` would need to be re-factored to `render_dirty_nodes/1`, despite no-changes as perceived by the user.

3. Using `Option` carries a runtime penalty to unwrap and test the value (conceivably LLVM could optimize this under certain circumstances, although I don't believe it does). Aside from visual clutter, they can also make it more opaque as to which version/branch of a function is expected to be taken. Granted, there are times where it is useful/preferable to variable arity, although that decision should be available to the library author.

### Unresolved questions
Should all functions of the same name have the same return type? (Theoretically not necessary, but it could complicate implementation)