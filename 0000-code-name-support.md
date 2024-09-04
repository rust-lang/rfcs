- Feature Name: code-name-support
- Start Date: 2024-09-04
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add optional code_name parameter to Cargo.toml [package] section.

# Motivation
[motivation]: #motivation

Many software projects use code names for their releases. It allows you the possibility
to refer to a program not only by its version number but also by code name or marking name.

For example instead of Windows 4.0.950 C people usually say Windows 95.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Let's say that you are working on the "my beloved doggy friend" program and for each
major or minor release you would like to use a new code name. You want to use dog names
as code names.

For the 0.1.0 release, you want to use the code name "Ace".

```toml
[package]
name = "mbdf"
version = "0.1.0"
edition = "2021"
code_name = "Ace"
```

For the next release, you would like to have a different code name.

```toml
[package]
name = "mbdf"
version = "0.2.0"
edition = "2021"
code_name = "Bailey"
```

This code name is visible everywhere where the version number is displayed.

```sh
 cargo build
   Compiling mbdf v0.1.0 "Ace" (/home/michal/projects/mbdf)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.72s
```

```sh
 cargo test
   Compiling mbdf v0.1.0 "Ace" (/home/michal/projects/mbdf)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.23s
     Running unittests src/main.rs (target/debug/deps/mbdf-902f78683f3a6e64)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Code name can be used inside the program.

```rust
    let code_name = env!("CARGO_PKG_CODE_NAME").to_string();
```

This is an optional feature. If it's not used there should be neither impact on displayed
versions nor CARGO_PKG_CODE_NAME env should not be created.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

I didn't do any research on how to implement it yet. I would like to get some feedback first.

If you don't consider this feature useful let me know. No hard feelings :)

# Drawbacks
[drawbacks]: #drawbacks

This is a feature that may be "nice to have". It's something that can be cool for some people.

It's definitely not an essential feature. If someone wants to have a code name displayed
somewhere in the app it's possible to just use some constant or normal variable.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This approach would provide a standardized way for having code names in the project.

Alternatives are:
- using constant
- using variable

# Prior art
[prior-art]: #prior-art

Marketing names for proprietary software like:
- Microsoft Office 95 instead of referring to version number 7.0
- Visual Studio 2005 instead of referring to version number 8.0

Code names in Linux:
- Baby Opossum Posse instead of referring to version number 6.11.0

# Unresolved questions
[unresolved-questions]: #unresolved-questions

TODO.

# Future possibilities
[future-possibilities]: #future-possibilities

None.
