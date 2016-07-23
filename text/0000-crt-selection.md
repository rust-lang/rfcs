- Feature Name: crt_selection
- Start Date: 2016-07-29
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add an environment variable to choose between whether to link the static CRT or dynamic CRT.

# Motivation
[motivation]: #motivation

On Windows code can choose to link to either statically link the CRT or dynamically link to the CRT. Right now the `libc` forces `msvcrt.lib` to be linked so Rust code always dynamically links to the CRT. Statically linking the CRT is desirable however as it avoids the need for consumers of your software to install the appropriate CRT or for you to bundle the CRT with your software.

The choice of an environment variable allows both rustc and build scripts to be able to react to it, and doesn't require any special support in Cargo. The choice of adding the target to the environment variable is so that it doesn't mess up the host target when cross compiling.

# Detailed design
[design]: #detailed-design

Add an environment variable named `target_CRT_KIND` where `target` is replaced with the target so that code targetting `x86_64-pc-windows-msvc` would specify the `x86_64-pc-windows-msvc_CRT_KIND` environment variable. Values for this environment variable will include `static` and `dynamic`, although other values could be added for the debug version of the CRT, or maybe even `none` if the user wishes to link their own custom CRT. Each platform will have its own set of supported values, and specifying a value that the platform doesn't support will result in rustc emitting an error.

Because `rustc` is responsible for invoking the linker, it will post process the linker arguments to ensure the wrong CRT is not passed to the linker. When using the `static` CRT, it would remove any references to `msvcrt.lib` and when using the `dynamic` CRT, it would remove any references to `libcmt.lib`. In addition it will also pass `/NODEFAULTLIB:msvcrt.lib` or `/NODEFAULTLIB:libcmt.lib` as appropriate to ensure that even if C/C++ code was compiled with the wrong choice of `/MD` or `/MT`, it won't automatically pull in the wrong version of the CRT. `rustc` will also pass the correct version of the CRT to the linker. If the environment variable is not specified, then `rustc` will behave the way it does today, with no special behavior towards how the CRT is passed to the linker.

Crates which compile C/C++ code, such as `gcc-rs`, will pay attention to this environment variable when deciding how to compile, and possibly link, C/C++ code. For example, passing `/MD` or `/MT` as appropriate to `cl.exe`.

# Drawbacks
[drawbacks]: #drawbacks

Adds further complexity to the ways in which rustc can be invoked, especially since it is quite platform specific.

# Alternatives
[alternatives]: #alternatives

* Some sort of command line parameter to rustc. Would be a hassle to pass along to all rustc invocations, and build scripts wouldn't be able to take advantage of it, unless special support was added to Cargo to set an environment variable for build scripts and pass it to all rustc invocations.
* Add new targets for using the static version of the CRT. Will result in even more targets that will need to be tested, and a significant amount of package duplication.

# Unresolved questions
[unresolved]: #unresolved-questions

* Should there be other environment variables to control other aspects of compilation and linking?
* Could this be applied to other platforms, perhaps musl?
* Bikeshedding on the name and structure of the environment variable.
