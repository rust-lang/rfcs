- Start Date: 2014-10-28
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Allow control of unspecified behaviour via compiler flags

# Motivation

Several things in Rust shouldn't have a well-defined result. However,
safety and debugging, would like some well-defined option.

# Detailed design

There are several things in Rust that don't have a well-specified behaviour.
Among them are:

 * Signed Integer arithmetic overflow
 * Shift amount overflow
 * Checked Array OOB indexing
 * Divide-by-Zero
 * LLVM UB in unsafe intrinsics
 * Unchecked Array OOB indexing (if we take #392 or some variant)

In these cases, there are several things we could want to do – the primary
options are task failure^H^H^H^H^H^H^Hpanic, aborting, undefined behaviour
(which is unsafe except in the first 4 cases), and (in the former 2)
returning a not-entirely-correct result (actually, with shift overflow,
there are *2* such results – either x86-style masking of the shift count,
or "correctly" returning 0/-1).

Add a compiler flag that controls the choice, -S TYPE=ACTION

Where TYPE is one of `signed_overflow`, `shift_overflow`,
`checked_oob`, `divide_by_zero`, `unsafe_intrinsic`, `unsafe_oob` (and
maybe more), and ACTION is one of "default", "fail", "undefined",
or (with the first 2 options) "wraparound". Add "all" and "unsafe_all" flags
to control the defaults (note that actually it is the safe options that
allow unsafety in safe code, which can be confusing). Have the default
be the current choice (currently we have `signed_overflow=wrap`,
`shift_overflow=undefined`, `checked_oob=fail`, `divide_by_zero=fail`,
`unsafe_intrinsic=undefined` and probably `unsafe_oob=undefined`) unless
we make some different decision.

For example, if we're debugging and want fail-fast, we could have
`-S all=fail`, and until we do something with shift amounts people could want
`-S shift_overflow=wrap`.

# Drawbacks

This would tempt people could use `-S checked_oob=undefined` and get 0wned.

# Alternatives

Also allow this on attributes. This would make it more likely to be
mis-used.

# Unresolved questions

None currently.

