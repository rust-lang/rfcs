- Feature Name: N/A
- Start Date: 2016-05-14
- RFC PR:
  [rust-lang/rfcs#1615](https://github.com/rust-lang/rfcs/pull/1615)
- Cargo Issues:
  [rust-lang/cargo#1734](https://github.com/rust-lang/cargo/issues/1734)
  [rust-lang/cargo#1976](https://github.com/rust-lang/cargo/issues/1976)
  [rust-lang/cargo#2127](https://github.com/rust-lang/cargo/pull/2127)
- Rustup Issues:
  [rust-lang-nursery/rustup.rs#247](https://github.com/rust-lang-nursery/rustup.rs/issues/247)
  [rust-lang-nursery/rustup.rs#473](https://github.com/rust-lang-nursery/rustup.rs/issues/473)

# Summary

Improve Cargo's integration into the host operating system by using
platform-specific paths for config files, cache files and executables,
so it interacts well with other tools on that platform.

# Motivation

Currently, Cargo puts all its files in a directory named `.cargo` below the
home directory of the user. Using this proposal, it would reuse existing
standard directories of the host platform. This allows other tools oblivious to
Cargo to function in the best way.

Benefits include:

* Using a `.cargo` directory is violating the recommendations/rules on most
  operating systems. _(Linux, Windows, macOS. Especially painful on Windows,
  where dotfiles are not hidden.)_
* Putting caches in designated cache directories allows backup tools to ignore
  them. _(Linux, Windows, macOS. Example: Time Machine ignores the cache
  directory on macOS.)_
* It makes it easier for users to manage, share and version-control their
  configuraton files, as configuration files from different applications end up
  in the same place, instead of being intermingled with cache files. _(Linux
  and macOS.)_
* Cargo contributes to the slow cleanup of the `$HOME` directory by stopping to
  add its application-private clutter to it. _(Linux.)_
* Using a standard directory for binary outputs can allow the user to execute
  Cargo-installed binaries without modifying their `PATH` variable. _(Linux)_

Solving this problem will likely also solve the same problem in Cargo-related
tools such as `rustup` as their strategy is "do what Cargo does".

This seems to be implemented in pip, the Python package manager, already.

# Detailed design

We are going to introduce new environment variables:
```
CARGO_BIN_DIR
CARGO_CACHE_DIR
CARGO_CONFIG_DIR
```

For the default values of these variables if they are set to the empty string
or not set (which will be the common case), see below.

These will be used to split the current `.cargo` (`CARGO_HOME`) directory up:
The cached packages (`.cargo/git`, `.cargo/registry`) will go into
`CARGO_CACHE_DIR`, binaries (`.cargo/bin`) installed by Cargo will go into
`CARGO_BIN_DIR` and the config (`.cargo/config`) will go into
`CARGO_CONFIG_DIR`.

In order to maintain backward compatibility, the old directory locations will
be checked if the new ones don't exist. In detail, this means:

1. If any of the new variables `CARGO_BIN_DIR`, `CARGO_CACHE_DIR`,
   `CARGO_CONFIG_DIR` are set and nonempty, use the new directory structure.
2. Else, if there is an override for the legacy Cargo directory, using
   `CARGO_HOME`, the directories for cache, configuration and executables are
   placed inside this directory.
3. Otherwise, if the Cargo-specfic platform-specific directories exist, use
   them. What constitutes a Cargo-specific directory is laid out below, for
   each platform.
4. If that's not the case, check whether the legacy directory exists (`.cargo`)
   and use it in that case.
5. If everything else fails, create the platform-specific directories and use
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

## Unixy systems (Linux, BSDs)

Here, we're following the [XDG specification](https://specifications.freedesktop.org/basedir-spec/basedir-spec-0.7.html).
By default, if no further variables are set, this means that we'll be using the
following subdirectories of the home directory.

```
cache:    .cache/cargo
config    .config/cargo
binaries: .local/bin
```


## MacOS

On macOS we follow the standard paths as specified in the [Library Directories](https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/FileSystemProgrammingGuide/MacOSXDirectories/MacOSXDirectories.html)
and will retrieve these from the [API](https://developer.apple.com/documentation/foundation/1414224-nssearchpathfordirectoriesindoma?language=objc).

```
cache:    Library/Caches/org.rust-lang.Cargo
config    Library/Application Support/org.rust-lang.Cargo
binaries: /usr/local/bin
```

**There is currently an on-going discussion about standardizing the location of
`.local/bin` together with a new XDG variable `XDG_BIN_HOME`. The
implementation of this RFC should be delayed until that discussion has finished
and use the result. This RFC will be amended with that result.**


## Rustup

Rustup will replicate Cargo's priorisation algorithm. If the results differ
from what the executed version of Cargo will do, Rustup will add environment
variables `CARGO_BIN_DIR`, `CARGO_CACHE_DIR`, `CARGO_CONFIG_DIR` for the new
versions of Cargo, and add symlinks for the old versions of Cargo.


## New subcommand

Cargo (and Rustup) are going to gain a new subcommand, `cargo dirs`. It will
display the directories currently in use, in a human-readable format. In order
to support other programs and scripts, this subcommand will also have switches
to print the data in machine-readable form, at least `--json` for JSON output
and maybe `--shell` for env-compatible output.

Example JSON (specifics left to the implementation):
```
{
  "bin_dir": "C:\\Users\\User\\AppData\\Local\\Programs\\Cargo",
  "cache_dir": "C:\\Users\\User\\AppData\\Local\\Temp\\Cargo",
  "config_dir": "C:\\Users\\User\\AppData\\Roaming\\Cargo"
}
```

Example (env-compatible):
```
CARGO_BIN_DIR=/home/user/.local/bin
CARGO_CACHE_DIR=/home/user/.cache/cargo
CARGO_CONFIG_DIR=/home/user/.config/cargo
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

* macOS could also use the `Library` folder for storing its data. This is mostly
  done by UI applications.

* One could only change the Windows paths, as the Windows integration is
  currently the worst.


# Unresolved questions

* None so far.
