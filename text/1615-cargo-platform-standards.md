- Feature Name: N/A
- Start Date: 2016-05-14
- RFC PR:
  [rust-lang/rfcs#1615](https://github.com/rust-lang/rfcs/pull/1615)
- Cargo Issues:
  [rust-lang/cargo#1734](https://github.com/rust-lang/cargo/issues/1734)
  [rust-lang/cargo#1976](https://github.com/rust-lang/cargo/issues/1976)

# Summary

Improve Cargo's integration into the host operating system by using
platform-specific paths for config files, temporary files and caches, so it
interacts well with other tools on that platform.

# Motivation

Currently, Cargo puts all its files in a directory named `.cargo` below the
home directory of the user. Using this proposal, it would reuse existing
standard directories of the host platform. This allows other tools oblivious to
Cargo to function in the best way. Using standard directories is the best way
to go unless there are concrete reasons for not doing so, otherwise Cargo just
adds complexity to all systems that try to interoperate with it. Benefits
include:

* Using a standard directory for binary outputs can allow the user to execute
  Cargo-installed binaries without modifying their `PATH` variable. E.g. on
  Fedora this is apparantly already the case, in Debian there's a ticket to
  include it.
* Putting caches in designated cache directories allows backup tools to ignore
  them.
* Using a `.cargo` directory on Windows is not idiomatic at all, no programs
  for Windows would use such a directory.
* Platform specific clean-up tools such as the Disk Cleanup Wizard work with
  Cargo (it wouldn't be very useful to try to modify the Wizard instead of
  Cargo to make this work).

Solving this problem will likely also solve the same problem in Cargo-related
tools such as `rustup` as their strategy is "do what Cargo does".

There seems to prior art for this in pip, the Python package manager.

# Detailed design

In order to maintain backward compatibility, the old directory locations will
be checked if the new ones don't exist. In detail, this means:

1. If there is an override for the Cargo directory, using `CARGO_HOME`, use
   that for all files.
2. Otherwise, if the platform-specific directories exist, use them.
3. If that's not the case, check whether the legacy directory exists (`.cargo`)
   and use it in that case.
4. If everything else fails, create the platform-specific directories and use
   them.

This makes Cargo use platform-specific directories for new installs while
retaining compatibility for the old directory layout. It also allows one to
keep all Cargo related data in one place if one wishes to.

## Windows

We'll obtain each of the following directories using the correct API.

```
cache:    AppData\Local\Temp\Cargo
config:   AppData\Roaming\Cargo
binaries: AppData\Local\Programs\Cargo
```

## Unixy systems (OS X, Linux, BSDs)

Here, we're following the [XDG specification](https://specifications.freedesktop.org/basedir-spec/basedir-spec-0.7.html).
By default, if no further variables are set, this means that we'll be using the
following subdirectories of the home directory.

```
cache:    .cache/cargo
config    .config/cargo
binaries: .local/bin
```
# Drawbacks

* This increases the complexity of where to find the files Cargo uses. This can
  be alleviated by providing library functions for Cargo related tools and a
  command line to tell which paths are in use.

* It can also complicate tutorials that tell users how Cargo works. The
  tutorials should probably reference the Cargo subcommand that displays the
  used locations. However it's still more complicated than static directory
  name (it's in a weird location for Windows users though).

# Alternatives

* OS X could also use the `Library` folder for storing its data. This is mostly
  done by UI applications.

* One could only change the Windows paths, as the Windows integration is
  currently the worst.

# Unresolved questions

* None so far.
