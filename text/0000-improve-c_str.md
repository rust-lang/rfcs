- Start Date: 2014-11-03
- RFC PR: 
- Rust Issue: 

# Summary

Relocation and interface changes for the Rust library module `c_str`.

* Move the `c_str` module out of `std` to rid the latter of type
  dependencies on `libc`.
* Split the current `CString` into a low-level type `CStrBuf` and
  a length-aware `CString` to make computation costs explicit.
* Provide custom destructors and purpose-specific, mnemonically named
  constructors.
* Add some methods and trait implementations to make the types more useful.
* Remove the `Clone` implementation due to lack of purpose.

# Motivation

The current interface in `c_str` has several issues:

* It exposes C character type signatures defined in `libc` as public
  interface in `std`, which may create portability problems for Rust code.
  At present the `libc` crate consistently defines `c_char` as `i8`
  even on ARM where the C `char` is traditionally unsigned, but it's not
  explained as a conscious break from the C standard, so it might
  be considered subject to change. In some exotic ABIs, a C `char` is not
  even a byte. Note how the type aliases from `libc` are resolved to their
  underlying primitive types in the documentation generated for `std`,
  leaving the readers unaware that the string pointer type may vary with
  the architecture.
* There is no support for strings allocated by other means than the
  standard C `malloc`.
* The constructor `CString::new` has somewhat confusing semantics and
  provides a dynamic flag for largely statically decided usage
  scenarios.
  See [this Rust issue](https://github.com/rust-lang/rust/issues/18117)
  for some discussion.
* The implementation of `CString` has a cost linear over the length
  of the string for `.len()` and `.as_bytes()`. This may result in latent
  performance problems for an unaware coder.
* `CString` does not implement `ToCStr`, where its implementation of
  `with_c_str*` would be zero-copy.

The improved `c_str` types should adequately represent zero-terminated
strings allocated by various foreign libraries, as well as provide
efficient adapters for passing string data from Rust into foreign
functions expecting C strings.

# Detailed design

## Character type dependency

To disentangle `std` from the definition of `libc::c_char`,
`c_str` is moved into a separate crate that is free to depend on `std`
and `libc`. A GitHub project
[c_compat](https://github.com/mzabaluev/rust-c-compat) has been
derived from Rust project source as a testbed for the proposed changes.
If the RFC is accepted, this code could be contributed back to the Rust
source tree.

## Low-level string wrapper

To expose the cost of calculating the length of a C string, a new
type `CStrBuf` is introduced. It provides fewer useful operations
and traits than `CString`, the main criterion being whether
the full length of the string needs to be established in order to
implement the operation. `CString` for its part encapsulates a length
field calculated upon construction, so it implements `.len()` and
`.as_bytes()` at constant cost. A `CStrBuf` can be promoted into
`CString` without copying the string using the method
`.into_c_str()`. As a convenience, `CStrBuf` also provides a method
`.to_string()` to copy the string's content into a plain Rust
`String`, provided that the content is valid UTF-8.

## Constructor reform and destructor closures

The former `CString::new` is replaced by the following constructors:

* `CStrBuf::new_unowned` - wraps a buffer without deallocation.
* `CStrBuf::new_libc` - will free the buffer with `libc::free` when dropped.
* `CStrBuf::new_with_dtor` - will run the provided destructor closure
  when dropped.
* `CString::new_unowned` - like `CStrBuf::new_unowned`, but with a
  pre-calculated length.
* `CString::new_libc` - like `CStrBuf::new_libc`, but with a
  pre-calculated length.
* `CString::new_with_dtor` - like `CStrBuf::new_with_dtor`, but with a
  pre-calculated length.

In a difference from `CVec`, the destructor closure receives the raw string
pointer as a parameter. This allows passing plain functions as destructors
in the most common case, potentially optimizing away the empty boxed
closure environment.

## No Clone

The semantics of `Clone` would be somewhat at odds with the newly
introduced custom destructors.
After some discussion on IRC and a few related GitHub issues, it is
perceived by the author that there is no need to facilitate copying
C strings in Rust.

## Implement ToCStr for CString and CStrBuf

What could be better producers of C strings than structures encapsulating
C strings? The implementation of `.with_c_str*()` for them is especially
neat.

# Drawbacks

This change breaks established Rust library interface, albeit considered
somewhat outdated.

The added type `CStrBuf` makes working with C strings more complicated.

# Alternatives

Keep the single type `CString` without the stored length and document
the hidden costs of operations. This would put a knowledge burden on the
programmer, reduce flexibility and discourage optimization.

Instead of moving `c_str` into a separate crate, the character type could be
forced to `i8`. However, there may be loss of portability with other code
using `libc::c_char` currently interchangeable with `i8`, should `libc` ever
define it differently for some architecture.

# Unresolved questions

Bring along `c_vec`, perhaps with additional harmonization regarding the
destructor closure type?

Move the C compatibility modules right into the `libc` crate?
