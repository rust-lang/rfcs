- Start Date: 2014-07-05
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)


# Summary

Change the current `extern "ABI"` syntax to either `extern<ABI>` or just `extern 
ABI`.


# Motivation

The available ABIs are conceptually much more like members of an enumeration 
than like arbitrary strings.

In the future, we may gain support for values (as opposed to types) as generic 
parameters. When that happens, we could enumerate the available ABIs in an 
`enum` for real:

    enum ExternAbi {
        Rust,
        C,
        StdCall,
        ...
    }

In which case it becomes possible to write, for instance, a higher-order 
function which is generic over a function pointer of any ABI:

    fn calculate_something<static SOME_ABI: ExternAbi>(some_function: extern<SOME_ABI> fn(int, int) -> int) -> int {
        /* ... use `some_function` ... */
    }

Alternately, the same thing can be encoded today, only slightly less elegantly, 
using plain types:

    // wired into compiler, can't be implemented by user types
    trait ExternAbi { };

    enum Rust { };
    impl ExternAbi for Rust { };

    enum C { };
    impl ExternAbi for C { };

    enum StdCall { };
    impl ExternAbi for StdCall { };

    ...

In which case the previous example is only slightly altered:

    fn calculate_something<SomeAbi: ExternAbi>(some_function: extern<SomeAbi> fn(int, int) -> int) -> int {
        /* ... use `some_function` ... */
    }

# Detailed design

Everywhere an ABI for an `extern` thing is specified, change the syntax from

    extern "ABI"

to either

    extern<ABI>

or

    extern ABI

.


# Drawbacks

Breaking change.

Not clear if it has benefits.


# Alternatives

Don't do it.


# Unresolved questions

Is the ability to abstract over ABIs *useful*?

Should it be `extern<ABI>` or `extern ABI`?

(The former is more suggestive, but there is precedent for omitting the `<>` for 
built-in things, such as `&` references.)
