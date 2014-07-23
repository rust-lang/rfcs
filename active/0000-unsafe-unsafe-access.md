Start Date: 2014-07-23
RFC PR #: (intentionally left blank)
Rust Issue #: (intentionally left blank)

# Summary

Make it unsafe to access the value of an ```&Unsafe<T>```.

# Motivation

Allow shared references to ```Unsafe<T>``` to be made safe while maintaining
the ability to initialize the ```Unsafe<T>``` statically.

Additionally, ensure that undefined behaviour can't happen within safe code, even after potentially bad (but defined) actions by unsafe code, as in issue
rust-lang/rust/#15920


# Detailed Design

As discussed in the "static mut" issues (#177, etc.), being able to
access the value field of ```Unsafe<T>``` objects forbids having such objects
accessible by an ```&Unsafe<T>``` reference, which would be useful in e.g. statics.

Making the ```Unsafe<T>```, or the value field of ```Unsafe<T>```, private currently is
not a solution, because private fields can't be statically initialized (for
a good reason).

The problem here is that ```Unsafe<T>``` is safe to initialize (because
it has no invariants, and nothing can reference a fresh ```Unsafe<T>```), but
an existing ```&Unsafe<T>``` can be referenced mutably, and therefore can't
be safely accessed. Compile-Time Function Evaluation would allow making the
field properly private, but it is rather complex and not planned for 1.0.

This can be solved by making access to fields of the "unsafe" lang item
unsafe (in the effect-checking pass). Note that one needs to prevent not only
direct access but at least by-ref destructuring, to prevent code such as this:

```Rust
    let x = Unsafe { value: 1u, marker1: InvariantType };
    let mut_alias : &mut uint = unsafe { &mut *x.get() };
    //...
    // The behaviour up to here was completely defined.
    // The unsafe introduced no undefined behaviour
    // Now, lets introduce some UB
    let Unsafe { value: ref alias, .. } = x;
    // Here alias and mut_alias are aliases.
```

Reducing backwards compatability breakage when we introduce CTFE would
suggest forbidding by-value destructuring and access as well.

# Drawbacks

This change is somewhat ugly and adds some "magic" to Unsafe<T>.

# Alternatives

Compile-Time Function Evaluation would allow a simple solution by
making ```value``` private and providing an initializing function, but is
a rather large and complex extension and is not planned for Rust 1.0.

A general extension that allows for publicly-initializable-but-not-
publicly-accessible fields would also do the job, but it would be more
complicated, and it doesn't seem that it would be useful outside this
case.

# Unresolved Questions

Should we really prevent by-value/by-mut-ref destructuring and access?
