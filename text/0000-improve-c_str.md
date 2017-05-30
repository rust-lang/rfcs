- Start Date: 2014-11-03
- RFC PR: 
- Rust Issue: 

# Summary

Relocation and interface changes for the Rust library module `c_str`.
The redesigned module is available as a
[standalone project](https://github.com/mzabaluev/rust-c-compat).

* Move the `c_str` module out of `std` to free the standard library from
  public type dependencies on `libc`.
* `CString` is made generic over a destructor type, allowing arbitrary
  deallocation functions.
* Methods to convert `CString` to Rust slice types are named
  `parse_as_bytes` and `parse_as_utf8` to reflect the scanning cost and
  the possibility of failure (in the UTF-8 case).
* Bring the constructors in line with the current conventions in the Rust
  library.
* Return `Result` for conversions that may fail, with descriptive error types.
* Remove the `Clone` implementation on `CString` due to lack of purpose.
* Remove `.as_mut_ptr()` due to its potential for convenient evil.
* Add an adaptor type `CStrArg` for passing string data to foreign functions
  accepting null-terminated strings.
* Add a trait `IntoCStr` to enable optimized conversions to `CStrArg`.

# Motivation

The current interface in `c_str` has several issues:

* It exposes C character type signatures defined in `libc` as public
  interface in `std`, which may create portability problems for Rust code.
  The `libc` crate does not consistently define `c_char` as `i8`.
* There is no support for strings allocated by other means than the
  standard C `malloc` (or, more precisely, strings that can be safely
  freed by `libc::free`).
* The constructor `CString::new` has somewhat confusing semantics and
  provides a dynamic flag for largely statically determined usage
  scenarios.
  See [this Rust issue](https://github.com/rust-lang/rust/issues/18117)
  for some discussion.
* The implementation of `CString` has a linear performance cost depending
  on the length of the string for `.len()` and `.as_bytes()`. Methods
  with these names have negligible cost on other types. The lack of
  mnemonical distinction may result in latent performance problems for an
  unaware coder.
* Conversion to `&str` returns an `Option`, with no detail as to the cause
  in case of failure. Similar functions elsewhere in `std` have been changed
  to return `Result`, so the C string API ought to follow suit.

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
derived from Rust project source to implement proposed changes.
If the RFC is accepted, this code could be contributed back to the Rust
source tree.

## Generic destructors for CString

The type `CString` is made generic over a destructor type implementing
trait `Dtor`. One destructor is provided in the `c_str` module:
`LibcDtor`, deallocating with `libc::free`.

## Constructors following Rust library conventions

The former `CString::new` is replaced by the following constructors:

* `CString::from_raw_buf` - wraps a raw pointer, will invoke a default
  instance of the destructor when dropped.
* `CString::with_dtor` - wraps a pointer, will invoke the provided destructor
  when dropped.

## Conversion methods

`CString` gets methods `parse_as_bytes`, `parse_as_utf8`, and
`parse_as_utf8_unchecked` to represent the C string's contents as a byte slice
or a string slice. The naming is chosen to reflect the performance cost of
scanning the string; these methods replace `as_bytes_no_nul` and
`as_str`. The return value of `parse_as_utf8` is a
`Result`.

## Deprecate the non-owned usage of CString

A non-owned `CString` in present `std::c_str` is mostly used to
obtain a Rust string or byte slice out of a C string pointer.
This is covered directly by functions `parse_as_bytes`, `parse_as_utf8`,
and `parse_as_utf8_unchecked` taking a raw pointer as the argument.

## Remove problematic methods and trait implementations

### No Clone

An implementation of `Clone` for the generic `CString` would involve
implementing allocation methods in addition to destructors. In absence of
a clear need to facilitate copying C strings in Rust, the trait implementation
is discontinued.

### No .as_mut_ptr()

`CString` is immutable and it largely treats the wrapped
string as immutable, so it should not provide a convenient 
escape into mutability.
For unsafe in-place modifications of string's bytes, there is
always an explicit raw pointer cast:
````rust
let hack_ptr = unsafe { c_str.as_ptr() as *mut libc::c_char };
// Ah-ha, I see we are up to something dangerous here
````

### Remove .owns_buffer()

In the proposed design, `CString` is dedicated to managing allocated strings.
Static polymorphism replaces the dynamic flag with regard to memory management.

## CStrArg and IntoCStr

An adaptor type `CStrArg` serves to pass string data to foreign functions
accepting null-terminated C strings. It has conversion constructors from
byte and string slices, including more ergonomic and optimized support
for static data and literals.

For more optimized ways to get data into `CStrArg`, trait `IntoCStr`
is provided. It allows non-copying transformation of values which own string
buffers.

# Drawbacks

This change breaks established Rust library interface, albeit considered
somewhat outdated.

# Alternatives

There is an [alternative RFC draft](https://github.com/rust-lang/rfcs/pull/494)
with different naming and design choices. In that draft, memory management of
C-allocated strings is not covered, and the API is kept under `std`.

Instead of moving `c_str` into a separate crate, the character type could
be forced to `i8`. However, there may be loss of portability with other code
using `libc::c_char` currently interchangeable with `i8`, should `libc` ever
define it differently for some architecture.

## Rejected designs

A previous iteration of this RFC and `c_compat` explored the following ideas:

* Split the current `CString` into a low-level type `CStrBuf` and
  a length-aware `CString` to make computation costs explicit.
* Custom destructors on `CString` were realised non-generically,
  as a proc closure (in today's Rust that would be a boxed `FnOnce`).

# Unresolved questions

A non-copying implementation of `IntoCStr` for `CString` is possible,
though storing a generic destructor by value involves some boxing
overhead. Need a clearer idea on usefulness and performance gains.

Bring along `c_vec`, perhaps with harmonization regarding
destructor polymorphism?
