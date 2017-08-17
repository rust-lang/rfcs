- Feature Name: rustdocs-fully-qualified-names
- Start Date: 2017-08-16
- RFC PR: 
- Rust Issue: 

# Summary
[summary]: #summary

Currently, RustDocs use unqualified type names. This is annoying when a library uses common idiomatic names, such as `Error` or `Result`. Instead, this RFC proposes that RustDocs should have an option to show fully-qualified names for identifiers declared outside of the current crate or wherever types are not unique.

# Motivation
[motivation]: #motivation

Currently, RustDocs use unqualified type names. This is annoying especially when a library uses common idiomatic names, such as `Error` or `Result`. A motivating example comes from [Tokio](https://docs.rs/tokio-proto/0.1.1/tokio_proto/pipeline/trait.ServerProto.html).

```rust
pub trait ServerProto<T: 'static>: 'static {
    type Request: 'static;
    type Response: 'static;
    type Transport: 'static + Stream<Item=Self::Request, Error=Error> + Sink<SinkItem=Self::Response, SinkError=Error>;
    type BindTransport: IntoFuture<Item=Self::Transport, Error=Error>;
    fn bind_transport(&self, io: T) -> Self::BindTransport;
}
```

Notice the `<Error=Error>` parts? Currently, it's not clear at a glance what `Error` type we are talking about (Is it `tokio::io::Error` or a stdlib type?). Likewise, the same problem would come up if you are using multiple `Result` types (e.g. `std::io::Result` and `std::result::Result`).

While it is possible in current RustDocs to see the fully-qualified name by hovering over the type with your cursor, it ought to be possible to figure out what is going on at a glance. Moreover, hovering hinders keyboard-only users.

The goal of this proposal is

- To make names unambiguous at a glance without the need for hovering
- To avoid cluttering a doc with lots of really long names
- To allow people who already know the types to procede without hinderance

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

If you are visiting a RustDoc for a crate you are not familiar with, you can select a "Show unambiguous names" option from a settings panel on the RustDoc. This has the effect of switching all type names on the entire page to fully-qualified names if either of the following is true

- the name is ambiguous otherwise (e.g. `std::result::Result` and `std::io::Result`)
- the name is from another crate (e.g. the stdlib)

This allows you to see exactly where each type comes from at a glance without the need to use the mouse.

Users who are already familiar with all of the types can simply disable the option to see RustDocs as they currently are.

Using the previous example, under this proposal, rustdocs would use the fully-qualified name: `<Error=std::io::error::Error>` or `std::result::Result`:

```rust
pub trait ServerProto<T: 'static>: 'static {
    type Request: 'static;
    type Response: 'static;
    type Transport: 'static + Stream<Item=Self::Request, Error=std::io::error::Error> + Sink<SinkItem=Self::Response, SinkError=std::io::error::Error>;
    type BindTransport: IntoFuture<Item=Self::Transport, Error=std::io::error::Error>;
    fn bind_transport(&self, io: T) -> Self::BindTransport;
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

A checkbox or panel would be added to the RustDocs page, which provides the functionality described above. The feature would be opt-in (off by default). Most likely, implementation would rely on simple javascript. The RustDoc tool would identify types for which expansion would be toggled when the docs are generated.

# Drawbacks
[drawbacks]: #drawbacks

- It increases dependency on javascript. I vaguely recall some discussions about wanting to limit dependence on javascript, but we already use it heavily and this would add very minimally to that dependence.
- Using fully exapanded names would cause extra clutter. However, since we only expand ambinguous or external names this is minimal. Additionally, the feature is opt-in, so if you find it clutter-inducing, you can just leave it turned off.

# Rationale and Alternatives
[alternatives]: #alternatives

These all come from the discussion thread [here](https://github.com/rust-lang/rfcs/issues/2004).

- Do nothing. We would just accept the problems noted in the "Motivation" section.

- Instead of having a toggling setting, the user would simply hold down the `F` key on the keyboard. Fully-qualified names would appear for the duration of the key being held down. This is probably a viable alternative, but one would expect that if you need to use the feature you want to turn it on and keep it on. Thus, this would be less convenient.

- Use the `use` declarations in the module, either by showing them on the RustDoc or by using the paths in these declarations instead of just the type names. However, it's not clear how to make this work well since the `use`s in my code might not be the same `use`s a typical consumer would need.

- Show fully-qualified names only for expanded items on the doc. Collapsed items would show in the current format. However, this does not serve the case where you know the types and just want to read about the item.

- This proposal except that we would show fully-qualified names only for types that are ambiguous (or as others have put it "go up levels until all ambiguities get resolved"). Unfortunately, a name may be unambiguous without it being clear to a reader that it is unambiguous. For example, in the example above, it turns out that `io::Error` would have been unambiguous because `tokio::io` does not contain an `Error` type; however, this is far from obvious to a reader who is not familiar to `tokio::io`.

# Unresolved questions
[unresolved]: #unresolved-questions

- Do we want to have a keyboard shortcut for toggling fully-qualified names?
