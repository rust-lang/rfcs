- Feature Name: (none)
- Start Date: 2015-02-12
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Running `rustdoc` or `cargo doc` should pass a special `-–cfg rustdoc` flag to `rustc`.

# Motivation

## Document platform-specific APIs

Example:

```rust
/// Open the web page in Internet Explorer.
#[cfg(target_os="win32")]
pub fn open_in_internet_explorer(url: &str) { ... }

/// Open the web page in Safari
#[cfg(any(target_os="macos", target_os="win32"))]
pub fn open_in_safari(url: &str) { ... }
```

In the current architecture, some of the above API will be missing depending on the OS calling `rustdoc`/`cargo doc`. However, we could ensure both will appear in the documentation if the tool provide a specific config:

```rust
/// Open the web page in Internet Explorer.
#[cfg(any(rustdoc, target_os="win32"))]
pub fn open_in_internet_explorer(url: &str) { ... }

/// Open the web page in Safari
#[cfg(any(rustdoc, target_os="macos", target_os="win32"))]
pub fn open_in_safari(url: &str) { ... }
```

## Document plugins

It may be convenient if we could produce a macro example like how `std::env!` is represented.

```rust
/// Performs some meta-programming magic.
#[cfg(rustdoc)]
#[macro_export]
macro_rules! my_plugin {
    ($($x:ident),*) => { /* plugin */ }
}
```

## Needed if the std doc is built with cargo

Rustc defines `--cfg dox` to document `env!`, `format_args!`, etc. ([rust-lang/rust#13255](https://github.com/rust-lang/rust/pull/13255)), by specifying this flag in the Makefile. If we want to use cargo to build the documentation ([rust-lang/rust#19240](https://github.com/rust-lang/rust/issues/19240)), then this RFC is likely needed.

# Detailed design

When `rustdoc` or `cargo doc` invokes `rustc`, it should add `--cfg rustdoc` as an additional flag.

Users can add documentation-only declarations with the `#[cfg(rustdoc)]` attribute.

# Drawbacks

Possibly abused to produce documentation not matching the actual API.

# Alternatives

* The identifier `rustdoc` can be changed to something else.

* Add a `cfg = [...]` option to the [profile sections](http://doc.crates.io/manifest.html#the-%5Bprofile.*%5D-sections) in Cargo.toml.

    ```toml
    [profile.doc]
    opt-level = 0
    debug = true
    rpath = false
    lto = false
    cfg = ["rustdoc"]    # <-- new
    ```

* With `cargo` it can be worked around using "features":

    ```toml
    # Cargo.toml

    [features]
    documentation = []
    ```

    ```rust
    // lib.rs
    #[cfg(any(feature="documentation", unix)]
    pub fn unix_specific_api() { ... }
    ```

    ```sh
    ## command line
    $ cargo doc --features documentation
    ```

    But the invocation is very ugly, and exposes a useless feature to Cargo.toml.

# Unresolved questions

None yet.

