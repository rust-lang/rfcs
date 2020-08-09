- Feature Name: posix_error_numbers
- Start Date: 2020-08-09
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add a `std::os::unix::ErrorNumber` struct for storing and comparing POSIX error
numbers.

# Motivation
[motivation]: #motivation

The POSIX standard defines a set of named [error numbers], such as `EIO` or
`EPIPE`, for use in communicating error conditions from low-level OS routines.
Many of these are currently exposed in [`libc`] as `c_int` constants, because
they figure prominently in writing bindings to libc on UNIX platforms.

However, there are two problems with this solution:

* POSIX does not specify the type of these error numbers -- they are defined as
  C macros, which are untyped integer literals. Many traditional UNIX APIs
	assume that they can use any integer type for returning error numbers, which
	are unergonomic to compare with a `c_int` const.

* Binaries that interact with the kernel directly (e.g. via `asm!()` syscalls)
  need to interpret error numbers, and should not require a dependency on libc
	to do so.

I propose adding an opaque `ErrorNumber` struct to `std::os::unix`, which would
provide limited conversion and comparison capabilities. By restricting the API
surface it should be possible to provide similar ergonomics to untyped integer
literals without abandoning type safety.

[error numbers]: https://pubs.opengroup.org/onlinepubs/9699919799/functions/V2_chap02.html#tag_15_03
[`libc`]: https://crates.io/crates/libc

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The POSIX standard defines a set of named [error numbers], such as `EPERM` or
`ENOENT`, for use in communicating error conditions from low-level OS routines.
POSIX does not specify the type or value of error numbers, so they vary between
operating systems and architectures. Many traditional UNIX APIs assume that
error numbers may be converted to or compared with any numeric value.

The `std::os::unix::ErrorNumber` struct represents an abstract non-zero error
number, which can be compared with any of the built-in integral types.

```rust
use std::os::raw::c_long;
use std::os::unix::ErrorNumber;

fn some_libc_fn() -> Option<ErrorNumber> {
  while {
    let err: c_long = libc::some_fn();
    if err == 0 {
      return None;
    }
    if err != ErrorNumber::EAGAIN {
      return Some(err);
    }
  }
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

There are two broad categories of users for this type:

* Binaries linking against `std` that call `libc` functions.
* Binaries built with `#![no_std]` that invoke kernel syscalls.

Since `ErrorNumber` is platform-specific functionality it doesn't belong in
`core`, so it must be placed in a separate crate that can be depended on by
`#![no_std]` binaries. `std` would then depend on this crate and re-export
its symbols.

The internal representation of `ErrorNumber` should be either `NonZeroI32` or
`NonZeroI64` depending on the target architecture. This is for performance, to
avoid sign-extending instructions when returning error numbers (e.g. in a tight
loop, or up a long call stack).

```rust
#[cfg(target_arch = "x86")]
type NonZeroFastInt = core::num::NonZeroI32;

#[cfg(target_arch = "x86_64")]
type NonZeroFastInt = core::num::NonZeroI64;

pub struct ErrorNumber(NonZeroFastInt);
```

Most of the user-facing API is in trait implementations, specifically
`PartialEq` for the integers.

```rust
impl PartialEq<i32> for ErrorNumber { ... }
impl PartialEq<ErrorNumber> for i32 { ... }

impl PartialEq<u32> for ErrorNumber { ... }
impl PartialEq<ErrorNumber> for u32 { ... }

/* ... and so on for all {u,i}{16,32,64,size} ... */
```

The implementations of `PartialEq` may also need to be arch-specific, to avoid
unnecessary bounds checks or conversions depending on the inner value.

The POSIX standard's list of error numbers is non-exhaustive, and platforms may
define their own error numbers. The `ErrorNumber` struct supports this through
construction from `core::num::NonZeroI16`, which is the smallest native integer
type I would expect someone trying to run a UNIX system on.

```rust
impl ErrorNumber {
	pub const fn new(n: core::num::NonZeroI16) -> ErrorNumber
}
```

Some applications need to store or transmit the value of an `ErrorNumber`. This
proposal bounds the range of `ErrorNumber` to be no greater than that of `i64`,
so that a non-failable conversion is possible, and also offers failable
conversions to smaller integral types.

```rust
impl From<ErrorNumber> for i64  { ... }
impl From<ErrorNumber> for core::num::NonZeroI64  { ... }

impl TryFrom<ErrorNumber> for i32 { ... }
impl TryFrom<ErrorNumber> for core::num::NonZeroI32 { ... }

/* ... and so on for all {u,i}{16,32,64,size} ... */
```

Values of POSIX error numbers for the current platform are available as associated constants, which may be used with ergonomics similar to the C
macros defined in `<errno.h>`. We'll want to generate these via an auxiliary
script that can compile and run a C program to detect platform values.

```rust
macro_rules! error_number {
	($name:ident, $matches:tt) => {
		pub const $name: ErrorNumber = ErrorNumber::new(unsafe {
			NonZeroI16::new_unchecked(match () $matches)
		});
	}
}

impl ErrorNumber {
	error_number!(EAGAIN, {
		#[cfg(target_os = "linux", target_arch = "x86_64")] _ => 11,
		#[cfg(target_os = "macos")] _ => 35,
		// ...
		#[cfg(doc)] _ => loop {},
	});
}
```

# Drawbacks
[drawbacks]: #drawbacks

This proposal increases the API footprint of the standard library.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The dominant alternative design is to use error number constants in `libc`,
which are of type `c_int` and therefore less ergonomic to compare. Depending on
`libc` is also undesirable for binaries that run in constrained environments.

## Signed values

The choice of signed values was chosen so that the raw return codes of syscalls
could be represented directly, as they traditionally use negative return values
to signal error conditions. Alternatively an `ErrorNumber` could be unsigned,
which better matches the current POSIX standard (it defines error numbers to be
positive integers) but would require the invoker of syscalls to do additional masking and failable type conversions.

A possible second alternative which would maintain ergonomics at the cost of
new types is to define a separate type for negative error numbers, with identical representation and appropriately inverted `PartialEq` implementations. This essentially moves the sign bit into the type system.

```rust
pub struct NegErrorNumber(ErrorNumber);

impl Neg for ErrorNumber {
	type Output = NegErrorNumber;
}

impl PartialEq<i32> for NegErrorNumber { ... }
// ...
```

```rust
use std::os::raw::c_long;
use std::os::unix::ErrorNumber;

unsafe fn some_sycall() -> Result<c_long, ErrorNumber> {
  while {
    let rc: c_long = syscalls::some_syscall();
    if rc >= 0 {
      return Ok(rc);
    }
    if rc != -ErrorNumber::EAGAIN { // compares `rc` to `NegErrorNumber(EAGAIN)`
      return Some(err);
    }
  }
}
```


# Prior art
[prior-art]: #prior-art

Most languages that support running on UNIX systems include symbols for POSIX
error numbers in their standard libraries. For example:

* C/C++: https://man7.org/linux/man-pages/man3/errno.3.html
* Go: https://golang.org/pkg/syscall/#Errno
* Python: https://docs.python.org/3.8/library/errno.html

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* Should the maximum range of `ErrorNumber` be reduced to `i32`? This would
  allow unfailable conversion to `std::io::Error` via `from_raw_os_error()`,
	but would require the internal representation of `ErrorNumber` to differ from
	the native word size on 64-bit platforms (unless the `From` impl contained a
	masking `i64 -> i32` expression).

* Should platform-specific extensions to the error number set be constants on
  `ErrorNumber`, like the generic POSIX consts? Or should they be placed in
	OS-specific modules such as `std::os::linux`?

* Should the `NegErrorNumber` design in the "alternatives" section be used?

# Future possibilities
[future-possibilities]: #future-possibilities

If the platform-specific extensions should be placed in separate modules, then
the stdlib should probably add modules `std::os::macos`, `std::os::freebsd`, and other POSIX platforms.
