- Feature Name: interrupt\_calling\_conventions
- Start Date: 2015-09-02
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add compiler support for hardware interrupt calling conventions.

# Motivation

Low-level systems software must often include interrupt service routines
(ISRs) to implement certain functionality. In some cases this is a way to
improve the system's efficiency, while in others it is required for correct
operation.

Most bare-metal software (such as a program designed to run on an ARM
microcontroller) contains at least one custom-written ISR, supporting important
system functionality. Oftentimes this is done in the interest of performance, so
providing fast interrupt entry and exit in software is an important goal.

In Rust today, the only option for implementation of such an ISR is to build the
ISR entry point as assembly which may call into Rust code. For example,
[RustOS][rustos], a simple x86 OS kernel implemented in Rust, currently
implements ISRs as callbacks to a single "master" interrupt handler:

[rustos]: https://github.com/ryanra/RustOS

```
.macro make_callback num
  callback_\num\():
.endm

.macro make_all_callbacks, num=50
.if \num+1
   make_callback %num 
      pushal
      pushl $\num
      call unified_handler
      
      addl $4, %esp
      popal
      iret
  make_all_callbacks \num-1
.endif
.endm
make_all_callbacks
```

This pair of assembler macros generate 50 different ISRs each of which simply
call the master handler with the interrupt vector number as a parameter, which
in turn dispatches to code to actually handle the interrupt. This kind of double
dispatch, while functional, is extremely inefficient in all of code size (even
unused ISRs must be generated), runtime memory usage (the double dispatch for
every interrupt requires a minimum of 12 additional bytes on the stack), and
speed (many ISRs are relatively simple, and the overhead of double dispatch may
have a significant effect on performance).

This kind of pattern is common in current bare-metal Rust programs: because the
language provides no satisfactory way to approach the need for ISRs, programmers
resort to inefficient ad-hoc solutions, which are usable but unlikely to be
competitive with other systems programming languages. By comparison, exposing
this information to the compiler both eliminates these inefficiencies and
exposes more information to the compiler's optimizer for even greater potential
gains.

Assembly is required for this task because typically hardware interrupts require
that the ISR preserve the values of all registers. Failure to do so will lead to
unpredictable (inconsistent, even) behavior of the interrupted code. On some
architectures, interrupt exit also requires a special instruction, such as
`iret` on x86.

# Detailed design

Add a family of `interrupt` calling conventions to Rust, one for each
architecture supported by the compiler. Given the current set of machines
supported officially (the `arch` field in target specifications), they are as
follow:

 * `aarch64`
 * `arm`
 * `mips`
 * `powerpc`
 * `x86`
 * `x86_64`

Each of these architectures combines with a `_interrupt` suffix to form the full
name of a calling convention.

Use of a calling convention which does not match the current target architecture
is an error. For portable software, this implies that all ISRs will have
`#[cfg]` guards. For example:

```
#[cfg(target_arch = "x86_64")]
extern "x86_64_interrupt" fn my_isr() { }

#[cfg(target_arch = "arm")]
extern "arm_interrupt" fn my_isr() { }
```

Each of these calling conventions may impose its own requirements on the
signature of a function declared with it. For example, `x86` and `x86_64`
sometimes receive an "error code" from the hardware, which acts like a
parameter to the function. The `x86_interrupt` and `x86_64_interrupt` calling
conventions may thus take one 32-bit-wide parameter.

On `arm`, by comparison, the hardware interrupt context follows the standard C
ABI, where ISRs neither receive any parameters nor return any values. Defining
a non-conforming function with this calling convention is an error.

The requirements for ISR function signatures on any given architecture are
deliberately left unspecified at this time (including the above examples, which
should not be treated as binding contracts), and should be specified at
implementation-time.

# Drawbacks

Support for more calling conventions (particularly platform-specific ones) adds
to the maintenance burden for the Rust compiler (though not in a very large
way), with minimal benefit to the majority of users (though the benefit to the
small proportion of users who *do* implement ISRs is large).

# Alternatives

 * Do nothing. ISRs may still be implemented with in assembly (either inline or
   not), but with some loss of efficiency and convenience.
 * Provide a more generic way to specify custom calling conventions, such as
   naked functions as proposed in [RFC 1201][naked-rfc]. This approach is
   applicable to more use cases than the one proposed here, but has a large
   number of associated safety concerns.

[naked-rfc]: https://github.com/rust-lang/rfcs/pull/1201

# Unresolved questions

It may be useful to provide a single portable interrupt calling convention in
addition to platform-specific ones which are incompatible with the portable
convention. Most systems have a simple expected function signature (no
parameters and no return value), so Rust code need not be concerned with which
calling convention should be used for a given platform in many cases.

Generalizing the family of calling convention names to reserve a subset of
possible names for architecture-specific calling conventions may be useful for
further extension in the future.  In such a case, the prefix `x86_` might be
reserved for 32-bit x86 calling conventions, `arm_` for ARM calling
conventions, and so forth.
