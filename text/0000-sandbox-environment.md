- Feature Name: sandbox-environment
- Start Date: 2019-10-26
- RFC PR: [rust-lang/rfcs#2794](https://github.com/rust-lang/rfcs/pull/2794)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This proposes a mechanism to precisely control what environment variables are
available to Rust programs at compilation time.

# Motivation
[motivation]: #motivation

Rust supports the `env!` and `option_env!` macros which allow Rust programs to
query arbitrary process environment variables at compilation time. This is a
very flexible mechanism to pass compile-time information to the program.

However, in many cases it is too flexible. It poses several problems:
1. Environment variables are generally not tracked by build systems, so changing
   a variable is not taken into account. Cargo has an ad-hoc mechanism for doing
   this in build scripts, but there's nothing to make this guaranteed correct
   (i.e. that all variables accessed are tracked).
2. There's no easy way to audit which environment variables a crate accesses.
   This not only exacerbates the problem above, but it also means that
   potentially sensitive information in an environment variable can be
   incorporated into the compiled code.
3. There's no way to override variables if they're needed by the build process
   itself. For example, the `PATH` variable likely needs to be set so that the
   compiler can execute its various components, but there's no way to override
   this so that `env!("PATH")` returns something else. This would be necessary
   where the compilation environment differs from the deployment environment
   (such as when cross-compiling).

This RFC proposes a way to precisely control the environment visible to the
compile-time macros, while defaulting to the current behaviour of making the
entire environment available.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Rust implements the `env!()` and `option_env!()` macros to access the process
environment variables at compilation time. `rustc` supports a number of
command-line options to control the environment visible to the compiling code.

By default all environment variables are available with their value taken from
the process environment. However there are several command-line options to
control this environment:
- `--env-clear` - remove all process environment variables from the logical
  environment, leaving it completely empty.
- `--env-remove VARIABLE` - Remove a specific variable from the logical
  environment. This is an exact match, and it does nothing if that variable is
  not set.
- `--env-pass VARIABLE` - Pass a variable from the process environment to the
  logical one, even if it had previously been removed. This lets specific
  variables to be allow-listed without having to explicitly set their value. The
  variable is ignored if it is not set or not UTF-8 encoded.
- `--env-set VARIABLE=VALUE` - Set a variable in the logical environment. This will
  either create a new variable, or override an existing value.

The options are processed in the order listed above (ie, clear, then remove,
then pass, then set). Multiple `--env-set` options affecting the same variable are
processed in command-line order, so later ones override earlier ones.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The implementation of this RFC introduces the notion of:
- the process environment which is inherited by the `rustc` process from it's
  invoker, and
- a logical environment which is accessed by the `env!`/`option_env!` macros

The logical environment is initialized from the complete process environment,
excluding only environment variables which are not UTF-8 encoded (name or
value).

Once initialized, the logical environment may be manipulated via the `--env-`
command-line options described below.

These environments are fundamentally key-value mappings, which is how they're
represented within the session state - a map from `String` to `String`.

## Processing of the options

These options are processed in order:
1. `--env-clear` - remove all variables from the logical environment.
1. `--env-remove VAR` - remove a specific variable from the logical environment.
   May be specified multiple times.
1. `--env-pass VAR`- set a variable in the logical environment from the process
   environment. Ignored if the variable is not set, or is not UTF-8 encoded.
1. `--env-set VAR=VALUE` - multiple `--env-set` options affecting the same variable are
   processed in command-line order, so later ones override earlier ones.

`rustc` will only accept UTF-8 encoded command-line options, which affects all
these options. This implies that all environment variables must have UTF-8
encoded names and values.

## Compile-time behaviour

The `env!()` and `option_env!()` macros only inspect the logical environment
with no reference to the process environment.

Note that this can't affect other environment accesses. For example, if a
procedural macro uses the `std::env::var()` function as part of its
implementation it will access the process environment. Any process that happens
to be invoked by `rustc` would still see the original process environment, not
the logical environment.

## Cargo

This has no direct effect on Cargo - it can completely ignore these options and
the overall behaviour would be unchanged. However, it's easy to imagine a
corresponding RFC for Cargo where it does more explicitly control the logical
environment. For example, it could constrain the accessible variables to:
1. ones that Cargo itself sets
2. ones that the build script sets via `rustc-env`
3. ones that the build script notes as `rerun-if-env-changed`
4. explicitly listed in the `Cargo.toml`

# Drawbacks
[drawbacks]: #drawbacks

The primary cost is additional complexity in invoking `rustc` itself, and
additional complexity in documenting `env!`/`option_env!`. Procedual macros
would need to be changed to access the logical environment, either by adding new
environment access APIs, or overriding the implementation of `std::env::var`
(etc) for procmacros.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

One alternative would be to simply do nothing, and leave things as-is.

In a Unix/Linux-like system, the environment can be controlled either with the
shell, or the `env` command. However this requires `rustc` to be invoked via a
shell or the `env` command, which may not be convenient for a given build
system. Alternatively, the buildsystem itself could be modified to suitably
configure the environment. However, it would still be strictly less capable, as
it would not be able to override variables or remove needed by:
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

C/C++ compilers typically have the ability set preprocessor macros via the
command-line. They can be set to arbitrary values via the `-D` option. This is
logically equivalent to both of Rust's mechanisms:
- preprocessor macros can be used as predicates in compile-time conditional
  compilation tests, and
- they can be expanded into the text of the program itself

These macros are explicit on the command-line, so they're easy to take into
account as an input to the compilation process. And the tools driving the C
compiler don't need any additional way to control the process environment.

Rust has a couple of mechanisms for compile-time configuration:
- It has the `--cfg` options which set flags which can be tested with
  compile-time predicates. These are strictly binary choices, which allow for
  conditional complilation.
- It has the process environment which can be queried at compile-time with
  `env!()` which evaluate to an arbitrary compile-time `&'static str` constant,
  which cannot be directly used for conditional compilation. The environment is
  not directly set via command-line options, but via another mechanism.

This mechanism doesn't change the semantics of either mechanism, but it does
make the environment a little more like a C preprocessor macro - they can be
precisely set on the command line, and if desired, only via the command line.

In general build systems need to have a precise knowledge of all inputs used to
build a particular artifact. This is especially important when trying to
implement fully reproducable builds, either for auditability reasons or just to
get good hit rates from a build cache. Build systems don't take the environment
into account because most of it isn't relevant to builds. Indeed, Rust is the
only compiled language I know of which allows direct access to environment
variables, so its not a thing that build systems *need* to take into account.
Even Cargo - purpose built for building Rust programs - can't explicitly track
what environment variables a piece of Rust code will access, and currently only
has limited tools for tracking this.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

There are two unresolved questions in my mind:

One is how to handle non-UTF-8 environment variable names and values? The
standard library has `std::env::var_os` to fetch the environment in OS-encoded
form, but there are no corresponing macros for compile-time. So I think just
restricting the logical to pure UTF-8 for names and values is fine.

The second is API extensions for procedural macros to make the logical
environment available to them. This could either be done by adding new APIs, or
perhaps some way to override the implementation of `std::env::var` in the
procmacro.

# Future possibilities
[future-possibilities]: #future-possibilities

## Path handling

Environment variables are frequently used for paths - a common pattern is:
```
include!(env!("SOME_PATH"))
```
If the variable `SOME_PATH` expands to a relative path, it is interpreted
relative to the source file which is doing the include. This requires the
variable to be set with an explicit knowledge of the source file layout. If it
is being using from multiple files, then no one relative path can be made to
work. In practice the only alternative is to always use absolute paths.

To address this, a possible extension would be `--env-set-path VAR=PATH` (and
`--env-pass-path VAR`) where the value is interpreted as a path relative to the
`rustc` current working directory - in other words, it could be set as a
relative path, but it would be interpreted as if it were an absolute path.
Absolute paths would always be treated as absolute.

## Regex Matching
This proposal is intended to be a minimum set of functionality to control the
environment. It requires very explicit control - aside from `--env-clear`, all
variables must be explicitly listed to remove or set their values.

A possible extension would be to allow regular expressions to select which
variable names should be removed from the logical environment or passed through
from the process environment. For example, `--env-remove-re` or `--env-pass-re`.

