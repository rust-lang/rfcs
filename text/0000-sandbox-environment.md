- Feature Name: sandbox-environment
- Start Date: 2019-10-26
- RFC PR: [rust-lang/rfcs#2794](https://github.com/rust-lang/rfcs/pull/2794)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This proposes a mechanism to precisely control what process environment is available to Rust programs at compilation time.

# Motivation
[motivation]: #motivation

Rust supports the `env!` and `option_env!` pseudo-macros which allow Rust programs to query arbitrary process environment
variables at compilation time. This is a very flexible mechanism to pass compile-time information to the program.

However, in many cases it is too flexible. It poses several problems:
1. Environment variables are generally not tracked by build systems, so changing a variable is not taken into account.
   Cargo has an ad-hoc mechanism for doing this in build scripts, but there's nothing to make this guaranteed correct
   (i.e. that all variables accessed are tracked).
2. There's no easy way to audit which environment variables a process accesses. This not only exacerbates the problem
   above, but it also means that potentially sensitive information in an environment variable can be incorporated into
   the compiled code.
3. There's no way to override variables if they're needed by the build process itself. For example, the `PATH` variable
   likely needs to be set so that the compiler can execute its various components, but there's no way to override this
   so that `env!("PATH")` returns something else. This would be necessary where the compilation environment differs from the
   deployment environment (such as when cross-compiling).

This RFC proposes a way to precisely control the environment visible to the compile-time macros, while defaulting to the
current behaviour.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Rust implements the `env!()` and `option_env!()` macros to access the process environment variables at compilation time.
`rustc` supports a number of command-line options to control the environment visible to the compiling code.

By default all environment variables are available with their value taken from the environment. There are several
additional controls to control the logical environment accessed by `env!()`/`option_env!()`:
- only allow access to a specific set of variables
- override specific variables to other values
- add new variables without them being present in the environment

These options are:
- `--env-allow REGEX` - match the REGEX against all existing process environment
  variables and allow them to be seen. The regex is matched against the entire variable name
  (that is, it is anchored).
- `--env-deny REGEX` - match the REGEX against the environment and remove those variables from the logical environment; this
  is equivalent to unsetting them from the Rust code's perspective.
- `--env-set VAR=VALUE` - set the logical value of an environment variable. This will override the value if it already exists
  in the process environment, or create a new logical environment variable.

These options are processed in order. For example:
```
rustc --env-deny '.*' --env-allow 'CARGO_.*' --env-set HOME=/home/system [...]
```
will clean all environment variables from the logical environment. It then allows access to all Cargo-set variables, and overrides
the value of `$HOME`.

Note that these options act on the logical environment, so:
```
rustc --env-set FOO=BAR --env-deny FOO
```
will leave `FOO` unset.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The implementation of this RFC introduces the notion of a logical environment which is accessed by the `env!`/`option_env!` macros,
distinct from the actual process environment. By default they are the same, but the additions of the `--env-allow`,
`--env-deny` and `--env-set` options allow the logical environment to be tailored as desired.

## Processing of the options

The `--env-` options are processed in the order they appear on the command-line, left to right. The logical environment is
initialized from the process environment. Then each each `--env` option is processed in turn, as it appears, to update the logical
environment. Specifically:

- `--env-allow REGEX` - Any name which doesn't match the REGEX is removed from the logical environment,
  as if it had never been set. This is symmetric with `--env-deny`.
- `--env-deny REGEX` - Any name which does match the REGEX is removed from the logical environment, as if it had never
  been set. This is symmetric with `--env-allow`.
- `--env-set VAR=VALUE` - Set a logical environment variable with the given value. This either sets a new variable, or
  overrides an existing variable's value.

Note that `--env-allow` and `--env-deny` affect variables set with previous `--env-set` options, possibly removing them.

If there are no `--env-` options then the logical environment is left in its initial state, which is identical to the process
environment.

`rustc` will only accept UTF-8 encoded command-line options, which affects all these options. TBD: What happens if the 
process environment has non-UTF-8 names or values?

## Compile-time behaviour

The `env!()` and `option_env!()` macros strictly act on the logical environment variables with no reference
to the process environment.

Note that this can't affect other environment accesses. For example, if a procedural macro uses the `std::env::var()` function
it will access the process environment. Any process that happens to be invoked by `rustc` would still see the original
process environment, not the logical environment.

(TBD: describe internal API for accessing the logical environment.)

## Cargo

This has no direct effect on Cargo - it can completely ignore these options and the overall behaviour would be unchanged. However,
it's easy to imagine a corresponding RFC for Cargo where it does more explicitly control the logical environment. For example,
it could constrain the accessible variables to:
1. ones that Cargo itself sets
2. ones that the build script sets via `rustc-env`
3. ones that the build script notes as `rerun-if-env-changed`
4. explicitly listed in the `Cargo.toml`

# Drawbacks
[drawbacks]: #drawbacks

The primary cost is additional complexity in invoking `rustc` itself, and additional complexity in documenting
`env!`/`option_env!`. Procedual macros would need to be changed to access the logical environment, either by
adding new environment access APIs, or overriding the implementation of `std::env::var` (etc) for procmacros.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

One alternative would be to simply do nothing, and leave things as-is.

In a Unix/Linux-like system, the environment can be controlled either with the shell, or the `env` command. However
this requires `rustc` to be invoked via a shell or the `env` command, which may not be convenient for a given build
system. Alternatively, the buildsystem itself could be modified to suitably configure the environment.
However, it would still be strictly less capable, as it would not be able to override
variables or remove needed by:
- `rustc` itself to run - such as `HOME`, `LD_PRELOAD` or `LD_LIBRARY_PATH`
- `rustc` to invoke the linker, such as `PATH`
- the linker for its own operation (`PATH`, and so on)

This can be particularly awkward when it isn't clear which variables are needed by the toolchain - for example,
invoking `rustc` via `rustup` uses a wider range of variables than directly invoking the `rustc` binary without
an intermediary.

This proposal gives maximal control when needed, without changing the default behaviours at all.

When `rustc` is embedded or long-running, such as in `rls` or `rust-analyzer`, then its necessary to explicitly
set the logical environment for each crate, rather than just inheriting the process environment.

# Prior art
[prior-art]: #prior-art

C/C++ compilers typically have the ability set preprocessor macros via the command-line. They can be set
to arbitrary values via the `-D` option. This is logically equivalent to both of Rust's mechanisms:
- preprocessor macros can be used as predicates in compile-time conditional compilation tests, and
- they can be expanded into the text of the program itself

These macros are explicit on the command-line, so they're easy to take into account as an input to the
compilation process. And the tools driving the C compiler don't need any addition way to control the
process environment.

Rust has a couple of mechanisms for compile-time configuration:
- It has the `--cfg` options which set flags which can be tested with compile-time predicates. These are strictly
  binary choices, which allow for conditional complilation.
- It has the process environment which can be queried at compile-time with `env!()` which evaluate to an
  arbitrary compile-time `&'static str` constant, which cannot be directly used for conditional compilation.
  The environment is not directly set via command-line options, but via another mechanism.

This mechanism doesn't change the semantics of either mechanism, but it does make the environment a little more like
a C preprocessor macro - they can be precisely set on the command line, and if desired, only via the command line.


In general build systems need to have a precise knowledge of all inputs used to build a particular artifact.
This is especially important when trying to implement fully reproducable builds, either for auditability reasons
or just to get good hit rates from a build cache. Build systems don't take the environment into
account because most of it isn't relevant to builds. Indeed, Rust is the only compiled language I know of which
allows direct access to environment variables, so its not a thing that build systems *need* to take into
account. Even Cargo - purpose built for building Rust programs - can't explicitly track what environment variables
a piece of Rust code will access, and currently only has limited tools for tracking this.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

There are two unresolved questions in my mind:

One is how to handle non-UTF-8 environment variable names and values? The standard library has `std::env::var_os` to
fetch the environment in OS-encoded form, but there are no corresponing macros for compile-time. So I think just
restricting the logical to pure UTF-8 for names and values is fine.

The second is API extensions for procedural macros to make the logical environment available to them. This could either be
done by adding new APIs, or perhaps some way to override the implementation of `std::env::var` in the procmacro.

# Future possibilities
[future-possibilities]: #future-possibilities

In addition to the environment, Rust also allows file contents to be directly read as either as code, a literal string or
binary data. The compiler puts no constraints on what paths can be read. The include mechanism is often used in
conjunction with `env!()` so that an environment variable can be used to determine the location of a path.
Even the paths to module sources can be overridden to arbitrary paths.

I plan to propose a realted RFC to apply similar controls to what paths can be accesssed by the various include
mechanisms so that they can be constrained (for example, to within the crate's sources, rather than anywhere on the
system).

More generally, bounding the amount of state available to the compiler outside of the sources and command-line options
is important to get good determinstic builds. Another source of currently unconstraint non-determinism is procedural
macros. I've [previously proposed](https://internals.rust-lang.org/t/pre-rfc-procmacros-implemented-in-wasm/10860)
compiling them into Web Assembly and running them within a wasm sandbox within `rustc`;
David Tolnay has [prototyped this in `watt`](https://github.com/dtolnay/watt/).