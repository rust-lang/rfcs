- Start Date: 2014-07-16
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Allow rustc plugins that perform code generation step.

# Motivation

Rust is dropping quite a few features now, while it's maturing. Still, it's extremely flexible, as rustc plugins allow custom AST generators. This RFC suggests taking one more step and allowing custom code generation.

This will allow plugin authors to implement support for a number of RFCs that got rejected ([linker-specific things][1], [more of them][2], [custom exhaustive `match` generation][3]) to be used for their specific requirements.

# Detailed design

I see a few ways to do this. First, it might be possible to pick a different codegen paths based on item metadata, e.g.
```rust
static i: int = 10;
```

and 

```rust
#[awesome_int]
static i: int = 10;
```

would generate two different things, thanks to `awesome_macro` that will tag this item for custom codegen.

Second option — a custom AST type, that will be processed by external plugins. This way the "usual" macro can convert incoming AST, say `ItemFn` into `ItemCustom`, which will be one again materialised with a dedicated code.

While this interface **is** extremely fragile by design, so is all the macro support which needs access to most of the compiler interals. Providing ways to hook into more internals should be safe, given that all the responsibility is still on the plugin authors.

# Drawbacks

This might be quite complex to implement. Also, such plugins would depent on bouth unstable rust internals API and unstable llvm API.

Badly written plugins can cause havoc. That applies to current procedural macros as well, though.

# Alternatives

Continue to use external pre- and post-processing of source and binaries. While this is still an option, it requires writing lots of boilerplate code and use other programming languages (C and assembly) for things, that could be actually done in rust.

# Unresolved questions

How flexible can such plugin be?

Can we take over the whole chunks of AST, like generating a custom implementation for an `if { ... } else { ... } block`, or we're limited to much smaller scale?

[1]: https://github.com/farcaller/rfcs/blob/4933072f08ae1767e76d7faa03f5d9aabb136b44/active/0000-better-low-level-handling.md
[2]: https://github.com/farcaller/rfcs/blob/650d47dd0827940ee18a69541d284e0584f22b63/active/0000-linker-placement.md
[3]: https://github.com/farcaller/rfcs/blob/967b166bd87b208f79cf1b03fb6782dad6059774/active/0000-bit-fields-and-matching.md
