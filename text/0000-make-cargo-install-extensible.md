# RFC: Make cargo install extensible

- Feature Name: extensible_cargo_install
- Start Date: 2018-03-27
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

## Summary

The goal of this RFC is to introduce the ability for an end user to be able to extend the `cargo install` command arbitrarily to include instructions that should be executed occur after `cargo install <project>`. 

### A note about `cargo` as a distribution tool

**We are in no way suggesting that** `cargo install` **should be the definitive or exclusive distribution mechanism, or that it should supplant other distribution mechanisms such as platform package management tools;** we are only proposing to improve the experience for the class of applications that already depend on cargo as a distribution platform today, notably small CLI applications and development tools for the Rust ecosystem. 

Additionally, this proposal will make `cargo install` more useful for the larger set of rust applications that leverage the larger, often preferred, platform-specific distribution workflows, which typically want to install into a temporary directory and package the result. This point is discussed further in the “Rationale and Alternatives” section.

Fundamentally, the aim of this RFC is aligned directly with Rust’s value of “productivity”. It aims to make both building and using small developer CLI tools simpler and easier which has a direct positive effect on the ability of all to improve their workflow. 

## Motivation

The motivation for this RFC is the desire to improve developer experience for those writing and using applications built in the [2018 Rust Roadmap](https://blog.rust-lang.org/2018/03/12/roadmap.html) identified [domains](https://blog.rust-lang.org/2018/03/12/roadmap.html#four-target-domains), specifically command line applications. The idea for this RFC originated in a discussion amongst the [CLI Working Group](https://internals.rust-lang.org/t/announcing-the-cli-working-group/6872) about [CLI application distribution](https://paper.dropbox.com/doc/CLI-WG-Berlin-2v99dJ7g6QVkGoT5VavPY) at the Rust All Hands in Berlin.

We were discussing packaging of applications and it became clear that this fundamentally depends on the installation process setting up and generating additional files. Currently applications typically mention such steps in their documentation, leaving them to the user to manually run. To streamline this process for both tool creators and users we wanted to provide reusable functionality to automate those steps instead of providing yet more documentation to follow.

## Guide-level explanation

When running `cargo install`, in addition to installing the compiled binaries, cargo will additionally build and run an `install.rs` script, if one exists. This script exists primarily to install additional files; it may also generate those additional files, such as running code to generate a [man](https://en.wikipedia.org/wiki/Man_page) [](https://en.wikipedia.org/wiki/Man_page)[page](https://en.wikipedia.org/wiki/Man_page) or [shell completion file](https://en.wikipedia.org/wiki/Command-line_completion).

In the common case, crates should make use of common infrastructure crates to implement this extended installation functionality; for instance, a common crate could provide the functionality of generating shell completions from a command-line argument parser. To support this case, we should support and strongly encourage the use of a `metainstall` mechanism that allows one of a crate’s dependencies to provide the `install.rs` functionality, by analogy with [metabuild](https://github.com/rust-lang/rfcs/pull/2196/).

### New Named Concepts

- `install.rs`: a file that contains a set of instructions to occur after `cargo install` is run.
- `metainstall`: a key in `Cargo.toml`  that specifies crate dependencies in an ordered list. Each crate dependency listed must be a library crate that provides a `metainstall` function. The `metainstall` function accepts no arguments, produces no return value, and should panic on failure.


  For example, a `Cargo.toml` leveraging the `metainstall` feature may look like this:
  
  ```toml
    [package]
    name = "the-best-cli-app-ever"
    version = "0.1.0"
    author = "Ferris the Crab"
    metainstall = ["cli-install"]
    
    [dependencies]
    cli-install = "0.1.0"
  ```

### Examples: CLI Applications

For CLI applications installed with `cargo install`, an `install.rs` file could include instructions to generate manpages or a shell completion file. Alternatively, a CLI application could have a `Cargo.toml` that contains a `metainstall` key that points to a crate dependency such as `cli-install` that contains instructions to generate and install manpages and shell completion files.

### Examples: Plugins

For a crate intended as a plugin for an existing program, an `install.rs` file could install the necessary plugin metadata to allow the program to list, enable, disable, and configure the plugin through a UI.

### New Way to Think About Cargo 

This feature extends the new type of thinking that `build.rs` created. Cargo can be seen now as a customizable build and distribution tool, whose functionality can be extended to better fit the specific type of application being built. Cargo is there to ease both the developer experience (by extending builds) as well as a way to ease the end user experience by customizing the distribution functionality of `cargo install` , eliminating the need for extensive documentation and cutting and pasting boilerplate or config.

This feature avoids putting detailed policies and functionality for specific use cases into `cargo install`; instead, it allows the crates.io ecosystem to support various use cases, and facilitates experimentation outside of `cargo` itself.

### New vs. Existing Users

This feature provides an improved developer experience for both new and existing users.

For **existing users**, this feature allows developers a way to improve end-user friendliness; instructions that were once required in extensive documentation can now be moved into code and automated away, creating a significantly more streamlined installation experience for end users.

Additionally, it gives developers the ability to package customized installation steps, which encourages the development of reusable conventions across the ecosystem. **New users** can easily adopt these conventions to provide a consistent experience and consistent functionality.

## Reference-level explanation

This is the technical portion of the RFC. Explain the design in sufficient detail that:

### Interaction with other features

This feature is quite similar to the existing extension functionality of `build.rs`, and `cargo` can be implemented using the same mechanism.

### Corner Cases

- Installing an application should not run the `install.rs` files of any of it’s dependencies, only of the application itself. 
- When re-installing a binary package this might run a different `install.rs` than the one that was initially run. This might lead to inconsistent/ undefined behavior.

More corner cases would appear if `cargo install` was ever extended beyond binary applications. The handling of those corner cases is left as an exercise to the writers of a proposal to extend `cargo install` to libraries or other types of files/applications.

## Drawbacks

- This adds another mechanism to run code from the crate on the user’s computer. However, `build.rs` already supports that, so building a crate already involves running code from that crate. Increasing the surface area of the crate code being run is potentially more concerning. 
## Rationale and alternatives

- There is already a way for custom code to be run during a build step via the `build.rs` file which is run before the build process of a crate. This proposal adds analogous functionality to the install process of a crate.
  - Tasks defined in `install.rs` can depend on a completed build or the specific platform upon which the application is being installed (e.g. paths), while `build.rs` is run before the main compilation step.
- Not allowing for the extension of `cargo install` relegates installation steps to the end user and any sort of post-install documentation. This has several downsides:
  - Forces users to follow additional install documentation when installing tools.
  - Makes the packaging of applications more complicated for developers of tools which can have negative impact on the ecosystem as a whole.
    - Tools may be written in different languages/ toolkits.
    - Developers are forced to implement their own installation mechanisms.
    - Tools only support specific popular platforms because being cross-platform is deemed “too difficult”.
- Making `cargo install` more capable could encourage people to use it as a primary distribution mechanism for a broader class of applications, rather than just for simple command-line tools. On the other hand, this same mechanism can also serve as the basis for distribution packaging, which typically wants to install into a temporary directory and package the result.
- It makes the emergence of conventions significantly more difficult as the option to reuse, share, or automate this task has significantly affordance than integrating it into the familiar `cargo` step.
- Currently, `cargo install` tracks what files it installs, and supports `cargo uninstall`; this extension mechanism does not hook into that. Potentially, `install.rs` could emit a list of installed files and let `cargo install` install them, producing a log of those files for later uninstallation. However, we do not intend for `cargo install` to become a full-featured package management mechanism; rather, we expect `cargo install` to work analogously to `make install`. While the occasional package provides a `make uninstall`, few developers expect such a mechanism.
- Currently, `cargo install` tracks what files it installs, and supports `cargo uninstall`; this extension mechanism does not hook into that. Potentially, `install.rs` could emit a list of installed files and let cargo install install them, producing a log of those files for later uninstallation. However, we do not intend for `cargo install` to become a full-featured package management mechanism; rather, we expect `cargo install` to work analogously to `make install`. While the occasional package provides a `make uninstall`, few developers expect such a mechanism.

## Prior art

This proposal seeks to provide symmetry with the `build.rs`  feature. The ability to customize the build functionality is quite similar to the desire to customize the install functionality. We see this RFC as rounding out the feature of customizable Rust project lifecycles.

## Unresolved Questions:

- An `install.rs` script may want to replace the existing functionality of `cargo install`, such as by installing binaries to a different location. On the other hand, allowing crates to override that behavior could make crates less consistent. We should consider whether to provide a mechanism to allow such overrides, or whether to mandate that `cargo install` always provides the baseline functionality of installing the binaries.
- `cargo install` could optionally have a flag to disable the running of the `install.rs` file.
- `cargo install` currently only supports installing binary crates. This mechanism could potentially make `cargo install` useful for library crates, `cdylib` crates, and similar. A future change to cargo may thus wish to introduce support for installing crates other than binary crates. Such a change may introduce additional concerns, such as Rust library ABI.

## Related RFCs and issues:

- https://github.com/rust-lang/cargo/issues/545
- https://github.com/rust-lang/cargo/issues/2386
- https://github.com/rust-lang/cargo/issues/2729
- https://github.com/rust-lang/rfcs/pull/2196
- https://github.com/rust-lang/rfcs/pull/1200
