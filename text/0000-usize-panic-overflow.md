- Feature Name: `usize_panic_on_overflow`
- Start Date: 2019-02-02
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

In `release` builds, adopt the current `debug` behavior of panicking on overflow
for arithmetic with `usize` (but not other sizes/types of integers).

# Motivation
[motivation]: #motivation

One of Rust's motivations is to make programming safer, particularly against
memory corruption vulnerabilities. In safe Rust code, integer overflows do not
lead to memory unsafety, however when combined with `unsafe`, integer overflow
is a frequent source of security vulnerabilities. We consider four examples:

- [CVE-2018-1000810](https://groups.google.com/d/msg/rustlang-security-announcements/CmSuTm-SaU0/AzVznVcTCgAJ):
  Integer overflow leading to heap buffer overflow in `str::repeat`.
- [CVE-2017-1000430](https://github.com/alicemaz/rust-base64/commit/24ead980daf11ba563e4fb2516187a56a71ad319):
  Integer overflow leading to heap buffer overflow in the `base64` create.
- [CrosVM](https://bugs.chromium.org/p/chromium/issues/detail?id=892904): Integer
  overflow leading to heap buffer overflow in ChromeOS's hypervisor.
- [ring](https://github.com/briansmith/ring/issues/742): Near-miss integer
  overflow, which would have led to heap buffer overflow.

A panic-by-default behavior would have ensured that none of these bugs would
have been a vulnerability. All four of these cases involved operations on
`usize` values. Because Rust is very strict in how integer types are handled, it
is exceedingly likely that overflows which lead to memory corruption will happen
on `usize` values: `usize` is consistently used in `core`/`std` APIs which deal
in lengths, buffer sizes, etc., the kind of values where overflow can be
dangerous, so it's no coincidence that historic integer-overflow vulnerabilities
occured with `usize` values.

Rust's current behavior of `panic` in `debug` builds and twos complement
overflow in `release` builds does not provide protection against these
vulnerabilities.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When an integer overflow is encountered when performing an arithmetic operation
(e.g. `+` or `*`), Rust has two possible different behaviors. In `debug` builds,
this will always cause a `panic`. In `release` builds the operation will succeed
and twos complement wrapping will occur - with one exception, if the operation
is being performed on `usize` integers you'll get the same `panic` as in a
`debug` build.

For most use cases, simply using the default arithmetic operators works well,
however if you need more control, such as to avoid the `panic` and return a
clear error, several methods are available:

- `checked_add` will return an `Option<$integer_type>`.
  `200u8.checked_add(200u8)`  will return `None`.
- `saturating_add` will return the maximum value the type can hold when an
  integer overflow occurs. `200u8.saturating_add(200u8)` will return `255`.
- `wrapping_add` performs the arithmetic with twos complement overflow.
  `200u8.wrapping_add(200u8)` will return `145`.

If you are using `unsafe`, it's important that you be aware that wrapping
overflow can lead to memory corruption. See
[CWE-190](https://cwe.mitre.org/data/definitions/190.html) for more details.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

All current integer overflow semantics would remain the same, with one
exception: `usize` in `release` builds would gain the `panic` behavior it
currently has in `debug` builds.

# Drawbacks
[drawbacks]: #drawbacks

There are three arguments against making this change:

1. Performance: Adding additional overflow checks on all `usize` arithmetic will
   slow down programs.
2. Consistency: Having special behavior for just `usize` is weird, if we want
   this behavior it should apply to all primitive integer types.
3. No changes at all: The current behavior is the desired end state, and thus we
   shouldn't deviate from it.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The three design considerations I was attempting to balance in designing this
solution were performance, protection, and predictability.

Performance is best served by adding the minimum number of checks in `release`
builds. We maximize protection against vulnerabilities by checking as many
things as possible. And a solution which is most consistent and easy to reason
about is best for predictability.

To consider the extremes of these:

- Adding no checks by default in `release` builds is best for performance, but
  worst for protection. Is it predictable (though suffers from the fact that
  many programmers do not think proactively about integer arithmetic as
  fallible).
- Checking *all* arithmetic is worst for performance (and indeed was previously
  rejected on these grounds in RFC 560), but best for protection. It's very
  predictable.
- A data-flow analysis based insertion of overflow checks where the result
  flowed into an allocation would be good for performance, because it would
  avoid unnecessary checks. It would be ok for protection, depending on how
  advanced it was (e.g. would it be inter-procedural?) However it would be
  extremely unpredictable and difficult for programmers to reason about either
  the performance or protection they were getting.

Given these criteria, here's how this proposal fares:

**Protection**: because most of the sinks for APIs which are dangerous when
combined with overflows and unsafe operate on `usize`, this will cover the
majority of cases at risk of becoming vulnerabilities. A review of historical
integer-overflow vulnerabilities in Rust found that they were *all* `usize`
arithmetic, and thus protected by this proposal.

**Performance**: This has a more mild performance impact than the previously
rejected check-all-arithmetic. I hypothesize that programs which rely most
heavily on arithmetic are not using `usize` (e.g. `curve25519-dalek`, which
was suggested to me as a good test of a such a crate, does not use `usize` for
representing limbs), however validating that the performance impact is
acceptable will require implementing this and measuring.

**Predictability**: This proposal makes it relatively easy to look at a piece of
code and see whether it is protected, but the behavior may not be the most
intuitive to those not familiar with it. The biggest challenges I anticipate are
with code that looks like:

```
fn explode(x: u32, y: u32) -> Vec<u8> {
    let v = vec![0; (x * y) as usize];
    for i in 0..x {
        for j in 0..y {
            unsafe {
                v.set_unchecked(i * y + j, VALUE);
            }
        }
    }
    return v;
}
```

Which someone may expect to be protected, but is not.

# Prior art
[prior-art]: #prior-art

This builds on RFC 560, which defined the current semantics for integer
overflows. In many respects I see this as the minimal change to advance towards
the world I see as the desired-but-not-yet-realistic consequence of that RFC -
checking for overflow by default everywhere.

I'm not aware of any other language which varies its overflow behavior by type.
This suggests that the behavior may be surprising to many users.

My assessment is that this is balanced by the fact that this would have
prevented all the vulnerabilities that have been seen thus far.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

There are three major questions I see:

- Is the performance really within the acceptable bounds? What are the
  acceptable bounds?
- Do we need a method for obtaining the previous behavior of `panic` in `debug`
  builds, but no checks in `release` builds? None of the overflow methods
  currently provide this behavior, and so for people who explicitly want it,
  under this proposal they'd need to write their own.
- Are additional options for controlling overflow behavior at the sub-crate
  level required?

# Future possibilities
[future-possibilities]: #future-possibilities

The natural evolution would be towards overflow checking all arithmetic. It is
my hope that one product of this effort will be identifying small places that
optimizations and code generation can be improved to either further minimize the
overhead of overflow checks, and thus help enable this future. Nevertheless, I
do not believe that checking all arithmetic by default is a realistic short-term
possibility.
