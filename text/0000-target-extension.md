- Feature Name: `target extension`
- Start Date: 2017-05-21
- RFC PR: 
- Rust Issue: 

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


Rust language is aimed to be used on different operating systems following
them self there own rules.In particular, each operating systems have proper way
to deal with breaking changes: if Linux tends to forbid breaking changes by
policy, all others systems doesn't have such rule. As Rust language tend to be
a stable language, having a stable way to describe breaking changes on the OS
would be very valuable and could become necessary as time passes.

LLVM deals with such changes on OS by having a different triple per OS version,
like for the following triples:

- `x86_64-apple-darwin16.0.0`
- `x86_64-unknown-freebsd12.0`
- `x86_64-unknown-freebsd11.0`
- `i386-unknown-openbsd5.8`
- `x86_64-unknown-netbsd7.99`


As examples, considers the following changes in several operating systems (some
are ABI changes, others API changes) and how a crate like `libc` would have to
deal with them. Please note that some are quite old but could be considered as 
representative of something that already occurred in the past.

- **OpenBSD 5.5** does a big breaking changes in order to be compatible with
  [year 2038](https://www.openbsd.org/faq/upgrade55.html#time_t): it switches
  from a signed 32 bit counter to signed 64 bit time type.
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

- **FreeBSD ?** wants to switch `ino_t` from 32 bits to 64 bits.
  See [Status Update and Call for Testing](https://lists.freebsd.org/pipermail/freebsd-fs/2017-April/024684.html),
  and [diff on lang/rust (ports tree)](https://github.com/FreeBSDFoundation/freebsd/blob/bc50a841851470d98cf1c219b261133536aa7ee8/ports.patch#L402).

- **NetBSD 7.99** (upcoming 8) adds a new member `mnt_lower` in the middle of
  the structure `mount` (public under `sys/mount.h`).
  See [commit message](http://cvsweb.netbsd.org/bsdweb.cgi/src/sys/sys/mount.h?rev=1.221&content-type=text/x-cvsweb-markup)
  and [diff of sys/mount.h](http://cvsweb.netbsd.org/bsdweb.cgi/src/sys/sys/mount.h.diff?r1=1.220&r2=1.221).

- **NetBSD 7.99** (upcoming 8) changes signature of `scandir()` function to conforms to `IEEE
  Std 1003.1-2008` (`const void *` to `const struct dirent **`).
  See [commit message](http://cvsweb.netbsd.org/bsdweb.cgi/src/include/dirent.h?rev=1.36&content-type=text/x-cvsweb-markup&sortby=date)
  and [diff to dirent.h](http://cvsweb.netbsd.org/bsdweb.cgi/src/include/dirent.h.diff?r1=1.35&r2=1.36&sortby=date).

- **DragonFly 1.4** switches `ino_t` from 32 bits to 64 bits.
  See [commit message](http://gitweb.dragonflybsd.org/dragonfly.git/commit/f91a71dd15504ebdb04387d0822771ef145b25f9?f=sys/sys/types.h)
  and [diff to sys/types.h](http://gitweb.dragonflybsd.org/dragonfly.git/blobdiff/6f1e2b382f6c2ba9b43a1fc106ba998b45499eea..f91a71dd15504ebdb04387d0822771ef145b25f9:/sys/sys/types.h)


In the current situation, `libc` crate has no way to deal in a stable way with
these changes. It could only support two incompatible OS version together by
only defining the common subset. Depending the breaking part, it could result
removed feature in rustc (removing `si_addr` for OpenBSD would break stack
overflow detection), or even breaking rustc itself (removing `ino_t` for
FreeBSD).

Additionally, in order to switch `libc` from one OS version to another, it
would be required to do a breaking change at `libc` level (incrementing major
version of `libc` itself) which is undesirable for this purpose.


The purpose of extending Rust `Target` type to follow LLVM Triple definition is
to be able to deal with such changes at Rust language level. As the target will
be able to make distinction between particular OS or environment version, it
would be possible to export the information in the same way we export
`target_os`, `target_arch`, `target_endian` or `target_pointer_width`.

This way, a crate like `libc` could export raw bindings of platform
specifically for the targeted version.


# Detailed design
[design]: #detailed-design

This is the bulk of the RFC. Explain the design in enough detail for somebody familiar
with the language to understand, and for somebody familiar with the compiler to implement.
This should get into specifics and corner-cases, and include examples of how the feature is used.



## Language level: what the user will see ?

- new `target_os_version` and `target_env_version` attributes
- new operators to deal with symbol (string) comparaison
- examples
  - `#[cfg(all(target_os="openbsd", target_os_version <  "5.5")]`
  - `#[cfg(all(target_os="openbsd", target_os_version >= "5.5")]`
  - `#[cfg(all(target_os="freebsd", target_os_version >= "10", target_os_version < "12")]`
  - `#[cfg(all(target_os="freebsd", any(target_os_version = "10", target_os_version = "11"))]`


## Syntax level

- operators to compare versions in attribute
- behaviour
  - numeric comparaison
    - "2" < "10" : true
  - able to deal with any number of "." inside the symbol:
    - "2" < "2.0" : true
    - "3" < "2.0" : false
    - "2.0" < "2.0" : false
    - "10.0" < "10.0.1" : true


See `libsyntax/attr.rs`, `libsyntax/config.rs`, `libsyntax/parse/parser.rs`
(unsure about these files for now).


## Backend level

- additional members in `Target`
  - `target_os_version: String` (could be empty: "")
  - `target_env_version: String` (could be empty: "")

- drawbacks: it requires a new target per OS version
- add a flexible code to easily create new target (with just
  `target_os_version` changing)

See `librustc_back/target/`.

## Session level

- populate and export the new attributes in the default build configuration

See `librustc/session/config.rs`.


## Workflow with new OS release

- when a new OS release occurs
  - adds to `libc` any changes (using `target_os_version` if required)
  - adds a new target specification
  - remove target of any unsupported OS version



# How We Teach This
[how-we-teach-this]: #how-we-teach-this

What names and terminology work best for these concepts and why? 
How is this idea best presented—as a continuation of existing Rust patterns, or as a wholly new one?

Would the acceptance of this proposal change how Rust is taught to new users at any level? 
How should this feature be introduced and taught to existing Rust users?

What additions or changes to the Rust Reference, _The Rust Programming Language_, and/or _Rust by Example_ does it entail?

# Drawbacks
[drawbacks]: #drawbacks

Why should we *not* do this?

- additional complexity in attributes for conditional compilation
- number of targets will grow a lot
  - not all targets will be testable (requires a particular OS version for
    running testsuite)
  - will require to deprecate unsupported OS version (for example OpenBSD
    officially support only the 2 last releases)

# Alternatives
[alternatives]: #alternatives

What other designs have been considered? What is the impact of not doing this?

- using `target_os` with the version inside. it would require to duplicate all
  `libc` code (for only small differencies) at each release.

- statu quo: while no fundamental breaking changes occurs at OS level, no need
  to do anything. Disclosure: FreeBSD switch to ino64 could occurs soon.


# Unresolved questions
[unresolved]: #unresolved-questions

What parts of the design are still TBD?

- partial implementation with no additional operator at syntax level. It
  requires to explicitly list all affected OS/env version on changes. Doable if
  the list of supported OS/env version is controlled in some way.

