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
very flexible mechanism to pass compile-time information to the program; in
effect it uses the environment as a form of compile-time directive.

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
   compiler can execute various sub-processes, but there's no way to override
   this so that `env!("PATH")` returns something else. This would be necessary
   where the compilation environment differs from the deployment environment
   (such as when cross-compiling).

This RFC proposes a way to precisely control the environment visible to the
compile-time macros, while defaulting to the current behaviour of making the
entire environment available.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Rust implements the `env!()` and `option_env!()` macros to access a logical
compile time environment at compilation time. By default, this logical
environment is initialized from, and is identical to, the inherited process
environment of `rustc` itself. (The only exception being environment variables
whos names or values are not valid utf8.)

There are several command-line options to control this environment and change it
from the default:
- `--env-clear` - remove all process environment variables from the logical
  environment, leaving it completely empty.
- `--env-remove VARIABLE` - Remove a specific variable from the logical
  environment. This is an exact match, and it does nothing if that variable is
  not set.
- `--env-pass VARIABLE` - Pass a variable from the process environment to the
  logical one, even if it had previously been removed. This lets specific
  variables to be allow-listed without having to explicitly set their value. The
  variable is ignored if it is not set or not utf8 encoded.
- `--env-set VARIABLE=VALUE` - Set a variable in the logical environment. This will
  either create a new variable, or override an existing value.

The options are processed in the order listed above (ie, clear, then remove,
then pass, then set). `--env-clear`, `--env-remove` and `--env-pass` are
idempotent, so multiple instances of each are the same as one.. Multiple
`--env-set` options affecting the same variable are processed in command-line
order, so later ones override earlier ones.

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

In general there are two distinct classes of compile-time environment fetch:
1. *operational*, used for running the compilation process itself; the specific
   values of the environment variables have no effect on the compiled code. For
   example, fetching `PATH` to work out how to invoke the linker, or `TMPDIR` to
   work out where to write temporary files.
2. *directive*, where the environment variable values will affect the generated
   code of the crate, generally by being incorporated into it - for example,
   using an environment variable as part of a `const` initializer expression.

In pure Rust code, the `env!()` and `option_env!()` macros query the logical
environment with no reference to the process environment. These can be used in
either an *operational* way:
```rust
include!(concat!(env!("GENDIR"), "/generated.rs"));
```
 ([discussion](#path-handling) about paths in the environment)
 
Or as a *directive*:
```rust
const USER: &str = env!("DEFLUSER");
```

Proc-macros are similar, but since they can run arbitrary code with full access
to libstd, classifying environment fetches has to be done on a case-by-case
basis. However, while it is possible in principle for a proc-macro to use the
environment as a directive, in practice any environment is much more like to be
operational. (See [discussion](#tracked-env) about interaction with he proposed
tracked environment API.)

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

This can be particularly awkward when it isn't clear which variables are needed
by the toolchain - for example, invoking `rustc` via `rustup` uses a wider range
of variables than directly invoking the `rustc` binary without an intermediary.

This proposal gives maximal control when needed, without changing the default
behaviours at all.

When `rustc` is embedded or long-running, such as in `rls` or `rust-analyzer`,
then its necessary to explicitly set the logical environment for each crate,
rather than just inheriting the process environment.

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

# Future possibilities
[future-possibilities]: #future-possibilities

## Path handling
[path-handling]: #path-handling

Environment variables are frequently used for paths - a common pattern is:
```
include!(env!("SOME_PATH"));
```

If the variable `SOME_PATH` expands to a relative path, it is interpreted
relative to the source file which contains the `include!()`. This requires the
variable to be set with an explicit knowledge of which source files will
reference the environment variable, and how they're layed out within the
directory structure. If it is being using from multiple files in different
direcctories (or specifically, at different directory depths), then no one
relative path can be made to work. In practice the practical alternative is to
always use absolute paths.

To address this, an extension would be `--env-set-path VAR=PATH` (and
`--env-pass-path VAR`) where the value is interpreted as a path relative to the
`rustc` current working directory - in other words, it could be set as a
relative path, but it would be interpreted as if it were an absolute path.
Absolute paths would always be treated as absolute.

## Tracked Environment API
[tracked-env]: #tracked-env

The [tracked environment API](https://github.com/rust-lang/rust/pull/74653)
proposal adds an API to allow proc-macro accesses to the environment to be
tracked appropriately. In the context of this RFC, these would be *directive*
accesses, where the environment has a direct effect on the generate crate. As
such it would access the logical environment proposed in this RFC.

By the same token, if the proc-macro performs *operational* environment fetches,
then it could still directly use the `std::env::var` API to fetch directly from
the process environment. It is, of course, the proc-macro author's
responsibility to understand the intent of their environment use, and correctly
choose and use the appropriate API.

## Regex Matching
This proposal is intended to be a minimum set of functionality to control the
environment. It requires very explicit control - aside from `--env-clear`, all
variables must be explicitly listed to remove or set their values.

A possible extension would be to allow regular expressions to select which
variable names should be removed from the logical environment or passed through
from the process environment. For example, `--env-remove-re` or `--env-pass-re`.

