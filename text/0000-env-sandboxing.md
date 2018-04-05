- Feature Name: env-sandboxing
- Start Date: 2018-04-05
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allow `rustc` to be invoked with constraints on which environment variables may
be queried, and which files may be included.

# Motivation
[motivation]: #motivation

This RFC introduces some simple sandboxing for process environment variables and
for include files (collectively "system environment").

This is primarily to allow a build system to more precisely control the inputs
to `rustc` which may affect the generated output. Rust has two mechanisms by
which an input source can access ambient properties of `rustc`'s system
environment: `env!()` and `option_env!()` for reading environment variables, and
`include!()`/`include_str!()`/`include_bytes!()` for reading arbitrary files.

## Environment Variables
`rustc` allows source code to directly access its process environment variables
via the `env!()` and `option_env!()` (pseudo-)macros. There is currently no way
to control which environment variables are accessible, or what their apparent
contents are.

This poses a few problems:

- The build system has no idea what variables the code is using for the purposes
  of tracking its inputs
- Code can read arbitrary contents from arbitrary variables, which may pose
  unwanted information leakage (from build environment to final deployment
  environment, for example).
- It combines the environment `rustc` needs for its own operation (PATH and
  LD_LIBRARY_PATH, for example) with environment the compiled code might want,
  and doesn't allow them to be set independently.

## Include Files
Rust allows any file to be directly included to be logically part of the
including source file, either as more Rust source code (include!()), a text
string (include_str!()) or arbitrary binary data (include_bytes!()). These
macros take a raw string which is used as an arbitrary path which may be
absolute - they therefore allow any file that `rustc` has permission to access
to be used in the compiled output.

(This differs from separating a crate into multiple source files via modules, as
those files are always relative to the top-level lib.rs/main.rs source.)

This causes a couple of problems:

- The build system can't know or constrain what files are actually inputs to the
  compiler.
- The code can't unintentionally leak state from the build environment to the
  deployment environment.

## Note
This RFC specifically does not intend to address any actual security concerns.
There are many other avenues that it does not attempt to control, such as
compiler plugins/proc macros. However, it does help with unintentional problems
which could result in later security problems if unaddressed. Blocking access to
files or environment variables also makes it easier to audit what environment
accesses a codebase is attempting to make.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Environment Variables

Rust supports the `env!()` and `option_env!()` macros to allow the value of a
process environment variable to be logically pasted into the source as a string
(or `Option` string).

`Rustc` has the following command-line options to allow the access to
environment variables to be controlled at a fine-grain level:

- `--env-clear` - completely empty the logical environment visible to
   `env!()`/`option_env!()`, causing them to all fail/return `None`. Without any
   other options, this will completely disable environment variable access.
- `--env-allow NAME` - allow a specific environment variable to be read from the
  process environment.
- `--env-define NAME=VAL` - define a logical environment variable. This does not
  need to be present in the actual process environment, or if it is, its value
  is overridden.

By default, the environment is completely open, leaving the existing behaviour
unchanged. Once one of the options above is specified, accesses to environment
variable becomes controlled accordingly.

## Include Paths

Rust allows arbitrary files to be included into a source file, with one of
`include!()` (include Rust source), `include_str!()` (include a file as a
literal text string), or `include_bytes!()` (include a file as arbitrary bytes).

`rustc` maintains a set of "allowed path prefixes". When it tries to open a file
for an include, it checks the path against a set of allowed prefixes, and only
allows the operation if it matches. For example if we set: `--include-prefix
foo/bar`, then `include!("foo/bar/blat.rs")` will be allowed, but
`include!("bar/bar/blacksheep.rs")` will fail, regardless of whether the file
actually exists or not.

`rustc` has the following command-line options to control the valid prefixes:
- `--clear-include-prefixes` - clear all allowable prefixes, effectively
  disabling all `include*!()` macros unless a new valid prefix is added.
- `--include-prefix PATH` - add PATH to the set of valid prefixes. All included
  paths must match one of the valid path prefixes before it can be opened.

By default, all path prefixes are valid, leaving the current behaviour
unchanged. They are only constrained once one of the options above are
specified.

All paths are canonicalized before matching (turned into an absolute path, all
symlinks followed, etc), so they must exist at the time they're specified. For
example, this means that a path of `foo/../bar` and `bar` are considered
equivalent (assuming that `foo` exists).

Note that a "path prefix" can be an entire pathname, allowing these options to
explicitly specify which individual files may be included.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This feature is implemented in a separate `env-sandbox` crate, which manages the
state for both environment variables and path prefixes. It is used in two
places:

- during command-line option parsing, the sandbox state is built up, and stored
  in the global config structure.
- it is added to `ParseSess`, so that it's available to the various `expand_`
  functions called from libsyntax_ext.

Specifically, `libsyntax_ext::env`'s `expand_option_env` and `expand_env` call
`EnvSandbox::env_get()` to get an environment variable rather than directly
calling `std::env::var()` so that the sandbox can mediate access.

Likewise, `libsyntax::ext::source_util`s `expand_include` uses
`EnvSandbox::path_lookup()` to get a path that either matches one of the
allowable prefixes, or returns an error. Likewise `expand_include_str` and
`expand_bytes` use `EnvSandbox::path_open()` with either successfully returns an
open file, or returns an error. In both cases, the error is `PermissionDenied`
with a description "path does not have a valid prefix".

In all cases, failing to fetch an environment variable or failing to open a file
should behave the same as if the variable really doesn't exist, or if the file
can't be opened for system reasons (non-existence, permission denied, etc).

Implementation is in PR https://github.com/rust-lang/rust/pull/49387.

# Drawbacks
[drawbacks]: #drawbacks

It adds numerous new command line options which ultimately be of limited
usefulness, while adding additional complexity and maintenance burden.

Users may be confused by the errors produced by using these features if they
didn't realize their build system was making use of them.

# Rationale and alternatives
[alternatives]: #alternatives

This design allows a build system to control `rustc`'s effective system
environment at a fine grain from invocation to invocation (aka hermetic
builds).

In Unix systems, its possible to control the environment directly, either via
the parameters passed to the `execve()` call invoking `rustc`, or using commands
such as `env`. These will allow any environment variable to be removed or
changed, but they can't be used to control the effective content of variables
that `rustc` itself (or any tool it invokes, like the linker) uses as part of
its own execution, such as `PATH` or `LD_LIBRARY_PATH`.

I don't know how one controls the environment under Windows.

The `--env-*` options give more control in a consistent way, and don't require
build tools to invoke `rustc` differently aside from adding new command-line
options. They allow the environment to be controlled in a cross-platform way.

Controlling file access is much more complex. The only way to control file
access in a similar way to the `--include-prefix` option is to build a
filesystem sandbox, and run the compiler within a chroot jail. This is a costly
operation which requires root-level permissions; the details of it are also very
system-specific. It also doesn't really solve the problem as it doesn't allow
particularly fine-grained control over what files can be included (any file
that's required to be present to make a runnable `rustc` instance is also
available for including).

# Prior art
[prior-art]: #prior-art

I don't know of other compiled languages which allow direct access to
environment variables (C and C++ don't), so there's no analogous requirement to
control access.

Many build systems have problems sandboxing their compilers. The
[Bazel](https://bazel.build/) and [Buck](https://buckbuild.com/) build tools
allow a limited form of sandbox by symlinking all the required source files (as
nominated by the dependency rules) into a special build directory, so that the
build fails if the compiler accesses files which haven't been symlinked.
However, this does not prevent access to other files outside that build
directory, via absolute path.

The Bazel developers have proposed a user-level filesystem,
[sandboxfs](https://blog.bazel.build/2017/08/25/introducing-sandboxfs.html), to
maintain a lightweight sandbox environment for an instance of a build. In
addition to being complex, system-specific and perhaps a performance hit (all
filesystem access is mediated by a user-mode filesystem), it has the same
disadvantage as other system-level sandboxing/container approaches, where the
entire compiler environment is still available for including.

There is a [cargo issue](https://github.com/rust-lang/cargo/issues/5282)
proposing a way to control the environment of a build script. This RFC provides
a mechanism to implement that.

# Unresolved questions
[unresolved]: #unresolved-questions

The use of the term "environment" is confusing - it refers to the general
concept of the system environment in which the rust compiler runs, but a
specific part of that are the "environment variables", which is often
abbreviated to "the environment". I had considered calling the over-arching
concept a plain "sandbox", but I was concerned that there might be other
potential uses for that term in the context of `rustc`.

The design is focused on a specific set of problems I have encountered while
integrating `rustc` into an existing build environment, but I think it's a
general enough problem and implementation that it's worth putting in rustc. But
I'm open to making changes if they make the feature more generally useful.

I think the RFC is complete enough to land as-is (perhaps barring some naming
changes).

## Environment

- Is matching by literal name powerful enough? Is there a requirement for some
  kind of pattern matching? (My view is that this can be added via a new
  command-line option, and the implementation of `EnvSandbox` can be extended;
  it wouldn't require any other changes.)
- Transformations applied to returned values? Ie, the ability to apply some kind
  of transformation/normalization to a variable? Seems too complex and out of
  scope for this change.
- Should invocations to `option_env!()` which access blocked variables print a
  warning?
- Should invocations of `env!()` which access blocked variables explicitly
  indicate its because of a command-line option, rather than because its
  actually doesn't exist?

## File Prefixes

- Is path prefix good enough? Should it be something like regex? I avoided this
  because I didn't know how to handle path canonicalization, and because it is
  more system-dependent.
- Should path matching be done on the basis of canonicalized paths? I though
  that requiring each file to be accessed via a specific form of path, and
  accessing it via other paths would be too brittle and confusing, and that
  doing everything on a canonical basis is more understandable. But there are
  still issues of how to handle a symlink in the middle of a path.
