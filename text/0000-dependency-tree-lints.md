- Feature Name: `multiple_crate_versions_lint` 
- Start Date: 2022-03-27
- RFC PR: N/A (yet)
- Rust Issue: N/A (yet)

# Summary
[summary]: #summary

Give maintainers a mechanism to declare that their library is unlikely to work (e.g. fail at runtime) if there are multiple versions of it in the dependency tree. When the constraint is violated, a compiler warning is emitted to inform users of the issues they are likely to encounter.

# Motivation
[motivation]: #motivation

## Implicit arguments
[implicit-arguments]: #implicit-arguments

In a simpler world, a function would declare all its required dependencies explicitly, as function arguments. In reality, this is often inconvenient, annoying or impossible—e.g. almost every function would have to require a logger as argument.  
Various patterns have emerged to provide support for **implicit arguments**: 

- Thread-local storage (e.g. [retrieving the current runtime in `tokio`](https://github.com/tokio-rs/tokio/blob/a8b75dbdf4360c9fd7fb874022169f0c00d38c4a/tokio/src/runtime/context.rs#L7) or [the current OpenTelemetry context in `opentelemetry`](https://github.com/open-telemetry/opentelemetry-rust/blob/dacd75af209550283d98be9f6f93e91588493032/opentelemetry-api/src/context.rs#L9));
- Task-local storage (e.g. [retrieving incoming flash messages in `actix-web-flash-messages`](https://github.com/LukeMathWalker/actix-web-flash-messages/blob/a7673e7db14f07cbc3b406581cf47353bfed70a5/actix-web-flash-messages/src/middleware.rs#L13));
- Request-local storage (e.g. the extensions type map in pretty much every single Rust web framework);
- Process state (e.g. [the global dispatcher in `tracing`](https://github.com/tokio-rs/tracing/blob/001eefbb423f85ba146c4097bfc4e080bd7b5a77/tracing-core/src/dispatch.rs#L197))

There is a clear pattern: the implicit arguments are global values scoped to a context (a thread, a task, a process, an incoming request, etc.).  

## Runtime failures
[runtime-failures]: #runtime-failures

All these patterns for implicit propagation break down **at runtime** as soon as the types do not line up, as it happens when two different versions of the crate are being used in different parts of the program.  
The runtime failures can be either visible or silent.

`tokio` is an example of a visible runtime failure.  
If your `main` function creates a runtime using `tokio:0.3.x` and somewhere in your program a future is spawned using `tokio:1.x.y`, you will get a panic with the following error message: `there is no reactor running, must be called from the context of a Tokio 1.x runtime`.

`opentelemetry`, instead, is an example of a silent runtime failure.  
The OpenTelemetry context won't be propagated if the OpenTelemetry context on the incoming HTTP request was extracted using `opentelemetry:0.15.x` but the propagation code used by the HTTP client relies on `opentelemetry:0.16.x`. Everything compiles, there is no runtime error, but nothing works as expected.

Both failure modes are undesirable. Catching these issues at compile-time would be preferable.  
Silent runtime failures, in particular, are tricky to debug if you do not have a solid understanding of Rust's type systems and the mechanisms used by these crates for implicit propagation. Beginners, in particular, are left puzzled and can waste a significant amount of time trying to troubleshoot these issues.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`cargo` makes it possible for your project to depend on multiple versions of the same crate.  
This behaviour can sometimes be undesirable: you can encounter runtime errors when depending on multiple versions of certain crates.  

Library authors can opt into emitting a compiler warning when this is the case:

```toml
[package]
name = "mylib"
version = "1.0.0"

[lib]
multiple_crate_versions = "warn"
```

Compiling a project that depends on both `mylib:0.3.0` and `mylib:1.0.0` will lead to this warning:

```text
warning: there are multiple versions of `mylib` in your dependency tree.

mylib v0.3.0

mylib v1.0.0
└── httpclient v1.3.5
    └── httpserver v0.1.6
```

As a library author, you can go one step further. You can specify a custom warning message to explain to the users of your crate what issues might arise by depending on multiple versions of it at the same time:

```toml
[package]
name = "mylib"
version = "1.0.0"

[lib]
multiple_crate_versions = { level = "warn", message = "`MyType::build` will panic if called within a context managed by a different version of `mylib`." }
```

```text
warning: there are multiple versions of `mylib` in your dependency tree.
         `MyType::build` will panic if called within a context managed by a 
         different version of `mylib`.

mylib v0.3.0

mylib v1.0.0
└── httpclient v1.3.5
    └── httpserver v0.1.6
```

The warning shows the different versions of `mylib` in your dependency tree and it highlights how they came to be there. In our example, the binary depends on `mylib:0.3.0` directly, while `mylib:1.0.0` is brought in as a transitive dependency of `httpserver:0.1.6`.  
You can either try to downgrade `httpserver` to a previous version or upgrade your direct `mylib` dependency to `1.0.0`.

As a user, there might be cases when you want to ignore this warning. You can do so by adding the following attribute in the entrypoint of your binary:

```rust
//! src/main.rs
#![allow(cargo::multiple_crate_versions(mylib))]
// [...]
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The parsing logic for `Cargo.toml` would have to be augmented to detect the new entry in the `lib` section.   
The lint will be evaluated after dependency resolution: it does not act as a constraint on `cargo`'s resolver.

# Drawbacks
[drawbacks]: #drawbacks

This RFC broadens the feature set of `cargo`, which might be deemed undesirable.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Implement the lint outside of `cargo`

There is enough machinery already available in the ecosystem (e.g. [`guppy`](https://docs.rs/guppy/latest/guppy/)) to write a third-party tool, outside of `cargo`, to enforce the lint described in this RFC. Some community-maintained tools provide, today, a very similar functionality (e.g. [`cargo-deny`](https://github.com/EmbarkStudios/cargo-deny/issues/350)).  

A third-party solution has various disadvantages: 

- It is unlikely to be used or discovered by beginners, the cohort that is most impacted by the type of runtime failures that this RFC seeks to prevent;
- It would most likely be consumer-driven instead of author-driven[^adoption], requiring a significant amount of due diligence by crate consumers. Crate authors are best-positioned to provide recommendations given their intimate knowledge of the inner workings of the libraries they maintain.
 
## Abuse existing features

Instead of adding a new feature to `cargo`, we could nudge library authors to achieve the same objective via existing features.  
Both mechanisms detailed below have the same drawbacks: 

- They result in a compiler error. This prevents the consumer from choosing to use multiple versions of the same crate, a necessity in certain scenarios (e.g. when upgrading the library version in a large application, piece by piece). Furthermore, making this a hard error would prevent existing crates in the ecosystem from adopting this feature, at the very least until their next breaking release;
- The error message is confusing due to the fact that we are hijacking features that are designed for different usecases.

### Link

The [`links` section](https://doc.rust-lang.org/cargo/reference/build-scripts.html#the-links-manifest-key) of the manifest is conceived as a mechanism to make `cargo` aware that a crate provides binding for a native library.  
`cargo` will return an error if you try building a binary that depends on the two crates with the same `links` section.  

Library authors could populate the `links` section using a non-existing yet sufficiently-unique name even if they do not link to a native library. This would prevent `cargo` from building a project that depends on two different versions of the library at the same time.

### `no_mangle`

Library authors can declare a public symbol annotated with [`no_mangle`](https://doc.rust-lang.org/reference/abi.html#the-no_mangle-attribute) in all versions of their library. 

```rust
#[no_mangle]
pub extern "C" fn there_can_be_only_one_version_of_mylib_at_once() {}
```

This causes a compiler error when multiple versions of the symbol are in scope (i.e. multiple versions of the library are present in the dependency tree.)

# Prior art
[prior-art]: #prior-art

## `clippy`

`clippy` defines a [`multiple_crate_versions` lint](https://rust-lang.github.io/rust-clippy/master/#multiple_crate_versions), in the `cargo` lint group.  
`clippy`'s lint [scans the dependency tree](https://github.com/rust-lang/rust-clippy/blob/6206086dd5a83477131094b1f0ef61b10e7ced42/clippy_lints/src/cargo/multiple_crate_versions.rs#L13) and gets triggered as soon as there is at least one crate that appears in the dependency tree with more than one version.

`clippy`'s lint can be useful when working with deployment targets where bloat is a major issue (e.g. embedded). It is impractical for projects with a large dependency tree: there will almost always be at least one crate that violates the constraint and `clippy`'s lint does not provide a mechanism to selectively silence the check (e.g. do not warn me about the `cookie` crate); you must instead `allow` the lint, disabling the check altogether.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## Custom warning messages

If multiple versions of the same crate define a custom warning message, what should `cargo` show to the user?  
The warning message from the latest version? All warning messages?

## `allow`

I am not familiar enough with `cargo`'s and `rustc`'s internals to understand how `cargo`, where I imagine this lint would live, would become aware of the `allow` statements relevant to this lint.

## Syntax

Is the `lib` section of the manifest the most appropriate location for configuring this lint? Should it be a top-level field or would we prefer to have it nested for future extensibility (e.g. inside `[[lib.lints]]`)?

Is it even desirable to have the lint "definition" in the `Cargo.toml` file? Is there an alternative syntax we could use, without being ambiguous, to have it at the top of the `lib.rs` file?

# Future possibilities
[future-possibilities]: #future-possibilities

Compatibility across versions is not always clear-cut due to the [semver trick](https://github.com/dtolnay/semver-trick).  
This lint could be extended to allow specifying which versions are incompatible:

```toml
[package]
name = "mylib"
version = "1.0.0"

[lib]
multiple_crate_versions = { level = "warn", conflicts_with = [">=0.2.0,<0.3.0"] }
```

[^adoption]: An author-driven system, like the one detailed in this RFC, could only be achieved via a third-party tool if there was critical mass (in terms of adoption) behind a single third-party linter. At that point, we would probably be talking of upstreaming this into `cargo` itself anyway.