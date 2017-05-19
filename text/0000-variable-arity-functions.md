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
Variable Arity Functions provide enhanced ergonomics around library design. They allow sensible, although potentially complex, default values for function arguments. In the case that default values are also implemented, they allow for default values that rely on a runtime variable, or a computation too complex for a function signature, or something that should be kept private to the implementation/library.

### Detailed design
#### Notation
The notation `function_name/[number]` is used to indicate the function and arity (ex. `foo/3` is the function `foo` that takes 3 arguments). Additionally, the notation `function_name*` will be used to refer to the group of all variants of a function with a particular name (ex. foo* refers to [foo/1, foo/2]).

#### Invariants
[1] A combination of name+arity is used to determine which version of a function is called.
[2] This should be completed entirely at compile time with no runtime cost.
[3] The group of functions with the same name and varying arity must form a set (Mutually exclusive name+arity). This differs from RFC Issue#153, which also allows varying types of the same arity.
[4] The signatures for foo* must have matching types for the portion of their signature that overlaps.

This design does not permit any ambiguity regarding which function is dispatched. Furthermore, it avoids the complexity of using argument types to determine which function is dispatched, an approach that can subsume variable arity functions, but has it's own merits and drawbacks. The current design is a vast improvement compared to Java where type coercion can make it difficult to determine which function is dispatched.



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

#### Invariant #4, Identical Argument Types
To prevent this feature from inadvertently allowing a "differing arity" type based function overloading, the signatures of foo* must have the same types for arguments that are common between definitions. This is intended to abide by the current decisions of the rust language. Additionally, it acts as a bikeshedding tool to ensure that functions that share the same name, behave similarly.

ex.) Compiles fine
```
// foo/1
fn foo(a: u32) {}

// foo/2
fn foo(a: u32, b: u64) {}
```

ex.) Error: Common argument types do not match
```
// foo/1
fn foo(a: u32) {}

// foo/2
fn foo(a: u64, b: u32) {}
```

ex.) Error: Common argument types do not match
```
// foo/1
fn foo(a: u32) -> u32 {}

// foo/2
fn foo(a: u32, b: u32) -> Graph {}
```

#### Interaction with Default Arguments
Default arguments are the most basic form variable arity, as perceived by a user.  They are also strict in that the return type and all other arguments' types are identical. Maintaining Invariant #3, interaction is prevented by enforcing foo* only contains at most one definition of a function for a particular arity.

ex.) Compiles fine
```
// foo/1 & foo/2
fn foo(a: u32, b: u32 = 3) {}

// foo/3
fn foo(a: u32, b: u32, c: u32)
```

ex.) Error: two definitions of foo/2
```
// foo/1 & foo/2
fn foo(a: u32, b: u32 = 3) {}

// foo/2
fn foo(a: u32, b: u32)
```

ex.) Error: two definitions of foo/2
```
// foo/1 & foo/2
fn foo(a: u32, b: u32 = 3) {}

// foo/2 & foo/3
fn foo(a: u32, b: u32, c: u32 = 3) {}
```


#### Interaction with Variadic Functions
In keeping with Invariant #3, a variable arity function defined with an arity in the range of a variadic function results in a compile time error. In the following example `u32...` denotes a variadic argument.

ex.) Compiles fine
```
// foo/2
fn foo(a: u32, b: u32) {}

// foo/3...
fn foo(a: u32, b: u32, c: u32...)
```

ex.) Error: foo/2 overlaps with foo/2...
```
// foo/2
fn foo(a: u32, b: u32) {}

// foo/2...
fn foo(a: u32, b: u32...)
```

#### Function Pointers
Ideally, type inference handles determining which arity of a function is used (Invariant#1/2). In cases where type inference doesn't work, explicit casting can be done in the same manner as [unique types per fn Item rust/pull/19891](https://github.com/rust-lang/rust/pull/19891).

ex.)
```
struct EventHandler {
    pub callback: fn(u32) -> (),
}
fn foo(a: u32) { unimplemented!() }
fn foo(a: u32, b: u64)  { unimplemented!() }

fn something() {
    // Use type system (implicitly foo/1 chosen)
    let handler = EventHandler{
        callback: foo,
    }

    // Explicit cast
    let handler = EventHandler{
        callback: foo as fn(u32) -> (),
    }

    // Posible language syntax extension; Mirrors Elixir/Erlang
    // Undesirable if this can be accomplished with existing features (cast)
    let handler = EventHandler{
        callback: foo/1,
    }

    // foo/1 used
    let func = foo;
    let handler = EventHandler{
        callback: func,
    }

    // foo/1 used
    let func: fn(u32) -> () = foo;

    // foo/2 used
    let func: fn(u32, u64) -> () = foo;

    // Error: Ambiguous
    let func = foo;
}
```

### Drawbacks
It's usage/benefits may not be worth the implementation cost and maintenance.
The design & implementation must consider the effects on other potential future features like variadic functions, default arguments, keyword arguments, currying.

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
Does the compiler have enough information already to easily add this feature? (Perhaps from Fn Item Types - Rust PR#19891)
Does variable arity complicate virtual (runtime) dispatch with trait objects, fat pointers, etc?