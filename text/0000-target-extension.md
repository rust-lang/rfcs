- Feature Name: `target extension`
- Start Date: 2017-06-27
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Extend Rust target specification to follow more closely LLVM triple specification.

# Motivation
[motivation]: #motivation

[LLVM triple](http://llvm.org/docs/LangRef.html#target-triple) specification is
more precise than the current [Rust
target](https://github.com/rust-lang/rust/blob/7ac844ffb850a73b98cd47cbdec909d1f03c7987/src/librustc_back/target/mod.rs#L228)
specification we have.

In particular, the following elements are missing from the Rust target
definition:
- optional [OS version](https://github.com/llvm-mirror/llvm/blob/343e535d9c38cf57173ace6597380752a18a6a67/include/llvm/ADT/Triple.h#L315)
- optional [environment version](https://github.com/llvm-mirror/llvm/blob/343e535d9c38cf57173ace6597380752a18a6a67/include/llvm/ADT/Triple.h#L303)


Rust language is aimed to be used on different operating systems following themselves
their own rules. In particular, each operating systems have proper way to
deal with breaking changes: if Linux tends to forbid breaking changes by
policy, all others systems doesn't have such rule. As Rust language tends to be
a stable language, having a stable way to describe breaking changes on the OS
would be very valuable and could become necessary as time passes.

LLVM deals with such changes on OS by having a different triple per OS version,
like for the following triples:

- `x86_64-apple-darwin16.0.0`
- `x86_64-unknown-freebsd12.0`
- `x86_64-unknown-freebsd11.0`
- `i386-unknown-openbsd5.8`
- `x86_64-unknown-netbsd7.99`


As examples, consider the following changes in several operating systems (some
are ABI changes, others API changes) and how a crate like `libc` would have to
deal with them. Please note that some are quite old but could be considered as 
representative of something that already occurred in the past.

- **OpenBSD 5.5** does a big breaking changes in order to be compatible with
  [year 2038](https://www.openbsd.org/faq/upgrade55.html#time_t): it switches
  from a signed 32 bit counter to a signed 64 bit time type.
  See [commit message](http://cvsweb.openbsd.org/cgi-bin/cvsweb/src/sys/sys/_types.h?rev=1.6&content-type=text/x-cvsweb-markup)
  and [diff on types](http://cvsweb.openbsd.org/cgi-bin/cvsweb/src/sys/sys/_types.h.diff?r1=1.5&r2=1.6).

- **OpenBSD 6.2** (upcoming) changes `si_addr` type (`char *` to `void *`) in
  `siginfo_t` structure.
  See [commit message](http://cvsweb.openbsd.org/cgi-bin/cvsweb/src/sys/sys/siginfo.h?rev=1.12&content-type=text/x-cvsweb-markup)
  and [diff on sys/siginfo.h](http://cvsweb.openbsd.org/cgi-bin/cvsweb/src/sys/sys/siginfo.h.diff?r1=1.11&r2=1.12).

- **FreeBSD 10** changes the `cap_rights_t` type from `uint64_t` to a structure
  that they can extend in the future in a backward compatible way.
  See [commit R255129](https://svnweb.freebsd.org/base?view=revision&revision=255219).

- **FreeBSD 11** changes signature of `psignal()` to align to POSIX 2008
  (`unsigned int` to `int` argument).
  See [commit R300997](https://svnweb.freebsd.org/base?view=revision&revision=300997)
  and [diff on signal.h](https://svnweb.freebsd.org/base/head/include/signal.h?r1=300997&r2=300996&pathrev=300997).

- **FreeBSD 12** (upcoming) removes `setkey()`, `encrypt()`, `des_setkey()` and
  `des_cipher()` functions.
  See [commit R306651](https://svnweb.freebsd.org/base?view=revision&revision=306651)
  and [diff of unistd.h](https://svnweb.freebsd.org/base/head/include/unistd.h?r1=306651&r2=306650&pathrev=306651).

- **FreeBSD 12** (upcoming) adds a new member `fb_memattr` in the middle of the
  structure `fb_info` (public under `sys/fbio.h`).
  See [commit R306555](https://svnweb.freebsd.org/base?view=revision&revision=306555)
  and [diff of sys/fbio.h](https://svnweb.freebsd.org/base/head/sys/sys/fbio.h?r1=306555&r2=306554&pathrev=306555).

- **FreeBSD 12** wants to switch `ino_t` from 32 bits to 64 bits.
  See [commit R318736](https://svnweb.freebsd.org/base?view=revision&revision=318736),
  [diff on types](https://svnweb.freebsd.org/base/head/sys/sys/_types.h?r1=307756&r2=318736),
  the [Status Update and Call for Testing](https://lists.freebsd.org/pipermail/freebsd-fs/2017-April/024684.html),
  and [diff on lang/rust (ports tree)](https://github.com/FreeBSDFoundation/freebsd/blob/bc50a841851470d98cf1c219b261133536aa7ee8/ports.patch#L402).

- **NetBSD 7.99** (upcoming 8) adds a new member `mnt_lower` in the middle of
  the structure `mount` (public under `sys/mount.h`).
  See [commit message](http://cvsweb.netbsd.org/bsdweb.cgi/src/sys/sys/mount.h?rev=1.221&content-type=text/x-cvsweb-markup)
  and [diff of sys/mount.h](http://cvsweb.netbsd.org/bsdweb.cgi/src/sys/sys/mount.h.diff?r1=1.220&r2=1.221).

- **NetBSD 7.99** (upcoming 8) changes signature of `scandir()` function to conform to `IEEE
  Std 1003.1-2008` (`const void *` to `const struct dirent **`).
  See [commit message](http://cvsweb.netbsd.org/bsdweb.cgi/src/include/dirent.h?rev=1.36&content-type=text/x-cvsweb-markup&sortby=date)
  and [diff to dirent.h](http://cvsweb.netbsd.org/bsdweb.cgi/src/include/dirent.h.diff?r1=1.35&r2=1.36&sortby=date).

- **DragonFly 1.4** switches `ino_t` from 32 bits to 64 bits.
  See [commit message](http://gitweb.dragonflybsd.org/dragonfly.git/commit/f91a71dd15504ebdb04387d0822771ef145b25f9?f=sys/sys/types.h)
  and [diff to sys/types.h](http://gitweb.dragonflybsd.org/dragonfly.git/blobdiff/6f1e2b382f6c2ba9b43a1fc106ba998b45499eea..f91a71dd15504ebdb04387d0822771ef145b25f9:/sys/sys/types.h)

- **MSVC 2015** has several breaking changes.
  See [Visual C++ change history 2003 - 2015](https://msdn.microsoft.com/en-us/library/bb531344.aspx),
  in particular section about *C Runtime* and *STL* breaking changes.
  In particular: some functions like [gets](https://msdn.microsoft.com/en-us/library/2029ea5f.aspx)
  or [`_cgets`](https://msdn.microsoft.com/en-us/library/3197776x.aspx) have been
  removed. In order to conform to C++11, old names for type traits from an earlier
  version of the C++ draft standard have been remamed.
  This one is an example of environment version.


In the current situation, `libc` crate has no way to deal in a stable way with
these changes. It could only support two incompatible OS version together by
only defining the common subset. Depending on the breaking part, it could result
in removed feature in rustc (removing `si_addr` for OpenBSD would break stack
overflow detection), or even breaking rustc itself (removing `ino_t` for
FreeBSD).

Additionally, in order to switch `libc` from one OS version to another, it
would be required to do a breaking change at `libc` level (incrementing major
version of `libc` itself) which is undesirable for this purpose.


The purpose of extending Rust `Target` type to follow LLVM Triple definition is
to be able to deal with such changes at Rust language level. As the target will
be able to distinguish between particular OS or environment versions, it
would be possible to export the information in the same way we export
`target_os`, `target_arch`, `target_endian` or `target_pointer_width`.

This way, a crate like `libc` could export raw bindings of platform
specifically for the targeted version.


It also has been mentioned in
[pre-rfc discussion](https://internals.rust-lang.org/t/pre-rfc-target-extension-dealing-with-breaking-changes-at-os-level/5289/11),
that it could be benefical to some others OS (like Windows) to have versioned
targets too: for the end-user "knowing what version he is targeting means he can
decide what symbols he can link to normally, and what symbols he has to
dynamically load, and what fallbacks he has to implement."


# Detailed design
[design]: #detailed-design


## Language level: what the user will see ?

At language level, new attributes for conditional compilation would be added:

- `target_os_version`
- `target_env_version`

There could be empty ("").

```rust
extern {
    // encrypt() function doesn't exist in freebsd12

    #[cfg(all(target_os="freebsd", not(target_os_version="12")))]
    pub fn encrypt(block *mut ::c_char, flag ::c_int) -> ::c_int;
}
```


Additionally, in order to simplify conditional compilation when changes are
accross several versions, new predicates would be added too in order to do
version comparaison.

```rust
extern {
    // encrypt() function doesn't exist anymore starting with freebsd12

    #[cfg(all(target_os="freebsd", version_lt(target_os_version, "12")))]
    pub fn encrypt(block *mut ::c_char, flag ::c_int) -> ::c_int;
}
```


Another complete (and simple) example: in OpenBSD 6.2, the structure
`siginfo_t` changed:

```rust
pub struct siginfo_t {
    pub si_signo: ::c_int,
    pub si_code: ::c_int,
    pub si_errno: ::c_int,

    // A type correction occured in 6.2.
    // Before it was a `char *` and now it is a `void *`.
    #[cfg(version_lt(target_os_version, "6.2"))]
    pub si_addr: *mut ::c_char,
    #[cfg(version_ge(target_os_version, "6.2"))]
    pub si_addr: *mut ::c_void,

    #[cfg(target_pointer_width = "32")]
    __pad: [u8; 112],
    #[cfg(target_pointer_width = "64")]
    __pad: [u8; 108],
}
```

It would be possible to target `x86_amd64-unknown-openbsd6.1` **and**
`x86_amd64-unknown-openbsd6.2` whereas with [current libc
code](https://github.com/rust-lang/libc/blob/6ddc76a27e0678c04ec7337591f8a0e36c065664/src/unix/bsd/netbsdlike/openbsdlike/mod.rs#L106)
only one version is possible, and switching from one to the other version would
be a breaking change in `libc` (and we would lose OpenBSD 6.1 supported
version).


## Syntax level

The addition of new predicates in attribute is a syntax extension.

It permits to easily make a piece of code available to a range version. It is a
facility to manipulate the new attributes defined at language level.

Predicates would be:

- `version_eq()` : equal
- `version_lt()` : less-than
- `version_le()` : less-or-equal
- `version_gt()` : greater-than
- `version_ge()` : greater-or-equal

The choice of an explicit name is to unhidden that the comparaison is done on
strings with a specific format (`major.minor.micro`).

Having a predicate instead of an operator (`version_lt()` vs `<`) avoid a too
intrusive syntax's modification too.

```rust
#[cfg(version_lt("2", "10"))]
println!("numeric comparaison: 2 < 10");

#[cfg(version_lt("3", "2.0")]
println!("able to deal with any number of '.': 3 < 2.0 (false)");
#[cfg(version_lt("2.0", "2.0")]
println!("able to deal with any number of '.': 2.0 < 2.0 (false)");
#[cfg(version_lt("10.0", "10.0.1")]
println!("able to deal with any number of '.':10.0 < 10.0.1");

#[cfg(version_eq("2", "2.0")]
println!("LLVM assumes \"2\" to be equivalent to \"2.0.0\");

#[cfg(version_le("3", "4", "5")]
println!("allow more than 2 arguments in the predicate: 3 <= 4 <= 5");
```

See `libsyntax/attr.rs`.



## Backend level

### Target structure

At the backend level, the `Target` structure gains two new members:

- `target_os_version: String`
- `target_env_version: String`

to represent the (possibly empty) versions of the OS and environment.

See `librustc_back/target/`.


### Specifics compilations options

It could be noted that some platforms could require additionnal compilation
options, like macOS and `-mmacosx-version-min` for specifying the minimal
version (it affects e.g.  dynamic library loader).

No additional changes is required for this support: `TargetOptions` structure
already contains array for such options: `pre_link_args`, `late_link_args` and
`post_link_args`.

Having a per version target permits to have different options per target.


### Implication on targets number

It should be noted it will implied a new target per OS version (for each
architecture), as soon as a breaking change occurs (new target required), or on
each major release (as it could be more simple for the end user to know which
target to use).

As example, FreeBSD has currently 3 targets (one per supported architecture:
`x86_64`, `i686` and `aarch64`). If we want to be able to express targets for 3
releases (two currently supported and one upcoming), the number of targets will
grow to 9 targets.


### Version tracking per OS

The exact way to tracking the OS version (creating a new target) should be done
per OS, because OS has different expectations regarding when a breaking change
could occur accross versions.

As example, FreeBSD keep ABI/API accross minor versions, and a breaking change
should only occur at major version (but not necessary).

So, the targets should be (for `x86_64` architecture):
- `x86_64-unknown-freebsd10` (currently supported)
- `x86_64-unknown-freebsd11` (currently supported)
- `x86_64-unknown-freebsd12` (in development)

At the opposite, OpenBSD only release major versions (even if expressed with
two digits version), and a breaking change could occur at each version:
- `x86_64-unknown-openbsd6.0` (currently supported)
- `x86_64-unknown-openbsd6.1` (currently supported)
- `x86_64-unknown-openbsd6.2` (in development)


### Default OS version for a target

It could be noted that the current unversioned target (like
`x86_64-unknown-openbsd`) could be still used as an alias of some versioned
target.

If so, the semantic have to be defined (tracking the oldest or most recent
supported version).

It could be convenient thing for compiler users, but any serious work should
rely on versioned OS target (as compiling for one target version could mean
unusable binary on other OS version).

Keeping the unversioned target would avoid a breaking change in command-line.
But the change could be useful too as it permits to downstream to be aware that
targeting particular OS doesn't mean the binary will work on other version.



## Session level

At the session level, rustc should populate and export the new attributes
(values taken from targeted backend) in the default build configuration.


See `librustc/session/config.rs`.



# How We Teach This
[how-we-teach-this]: #how-we-teach-this

If modifying the `Target` struct is a low-level change by itself, the current
RFC proposes it in order to change an implicit paradigm in targets (the
targeting OS will be stable accross version, which is false).

With the RFC, Rust become able to express this lack of stability on the OS, in
a stable way. In a sens, it extents the ability of Rust to targets several OS
by refining to targets several OS version.

From downstream perspective, it permits to use Rust to target several OS
versions whereas the versions are incompatibles.

From Rust developer perspective, it adds a new attributes for conditional
compilation, and extent the syntax with new predicates in convenient way.

Regarding documentation, additions have to be done on _Rust Reference_ in order
to mention new attributes in conditional compilation attribute section, with
related new predicates.

Visible changes should also occur on main rust-lang.org site on pages about
rustc distribution: rustup will be able to distribute binaries for more
platforms (per OS version), or about platform support.


# Drawbacks
[drawbacks]: #drawbacks

At syntax level, it adds additional complexity for defining new predicates
for manipulating version numbers.

At backend level, the number of targets will grow a lot. It means that not all
targets will be testable (too much required ressources and it would require a
particular OS version for testing too).

It will require to regulary deprecate old targets (for unsupported OS version)
in order to not keep too much old stuff. The end-user has still the possibility
to use flexible target using external JSON file for these targets, if the
corresponding code for this particular version is still in `libc` crate.

Projects using Rust with binary distribution will have to update in order to
cover a more important number of platforms. In particular rustc itself (with
and without rustup). It will mean more ressources to build more targets. But
the resulting binaries will work on all targets.


# Alternatives
[alternatives]: #alternatives

## Simply appending the version in the target name

The more simple approch is to use `target_os` with the OS version inside
(`freebsd12`).  But it would require to duplicate all `libc` code (for only
small differences) at each release. Having a separated attribute is more simple.

Having only parts of the current RFC is also possible: new predicates at syntax
level are only a way simplify code expression. It would requires to explicitly
list all affected OS/env version on changes. It is doable if the list of
supported/OS/env versions is controlled in some way.

But without some way to express breaking changes existence at OS level, Rust is
unable to targeting simultaneous several OS version. Regarding
[issue #42681](https://github.com/rust-lang/rust/issues/42681) for FreeBSD 12,
it means Rust should either deprecating older FreeBSD versions support (whereas
FreeBSD itself still support them) or not supporting FreeBSD 12.

## Runtime detection

Runtime detection is already in use for [Android](https://github.com/rust-lang/rust/blob/13157c4ebcca735a0842bd03c3dad1de7c429f9f/src/libstd/sys/unix/android.rs#L70-L93).
It has the advantage to permit rustc to target *several* OS with the same binary.

It is based on OS version detection (with symbol existence for example) and on
providing fallback or alternative for function calls.

But it couldn't cover all aspects of ABI breaking, specially changes in
structures (member size or offset change). It would require to replace
structure's member access by function calls doing runtime detection. The
possible overhead could be removed by using lazy detection and caching.

## Dynamic bindings generation

A possible alternative is to replace `libc` with FFI bindings generation at
compile-time (using [rust-bindgen](https://github.com/servo/rust-bindgen) for
example). But it isn't suitable for cross-building.

## Adding cfg attribute using build.rs

It is possible to do some compile time detection (or using cargo feature) to
select a particular OS version in `libc`. It has been [proposed for resolving
the FreeBSD 12 ABI issue](https://github.com/rust-lang/libc/pull/721).

With such code, the `libc` code is right regarding the selected version.

The drawback is such detection is fragile, and crosscompilation more complex
(it requires cargo feature usage).

Additionally, a larger problem is mixing code with different OS version would
be possible (no error at compile time): for example using libstd from rustup
targeting one version, and using with crate locally compiled for another
version. It would produce bad code and crash could occurs at runtime.

# Unresolved questions
[unresolved]: #unresolved-questions

As unresolved-question, the question about the unversioned target on
command-line is open. Does it makes sens to have it or not ?


# Related previous discussions
[discussions]: #previous-discussions

- [libc#7570: How to deal with breaking changes on platform ?](https://github.com/rust-lang/libc/issues/570)
- [Rust Internals: Pre-RFC: target extension (dealing with breaking changes at OS level)](https://internals.rust-lang.org/t/pre-rfc-target-extension-dealing-with-breaking-changes-at-os-level/5289)
