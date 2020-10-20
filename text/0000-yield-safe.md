- Feature Name: `yield_safe`
- Start Date: 2020-03-27
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

[summary]: #summary

This feature enables giving types a default auto trait `YieldSafe` which permits them to be used in an async context.
Consequently, it's possible to mark a type as `!YieldSafe` which disallows yielding control in an async block, while an
object of such type is alive at that point.

# Motivation

[motivation]: #motivation

Some operations are not safe to be performed within an async context. A trivial example might be locking a standard
mutex in an async block - without proper care we risk deadlocking the process. Rust currently doesn't check such
situations at compile time, which might lead to hard to debug runtime errors. While it's true such dangerous operations
are logic errors, not related to language itself, it would be very helpful if there was a mechanism to mark types as
potentially hazardous to use in async code, which would result in a compile error. This mechanism is similar to
detecting async code inside locks in C#.

# Guide-level explanation

[guide-level-explanation]: #guide-level-explanation

This feature adds a new `YieldSafe` default auto trait which propagates according to the same rules as other auto
traits. Yield-safe types can be alive at a point of yielding control, e.g. `.await`-ing a Future. At the same time, no
`!YieldSafe` object can cross a control yield boundary.

    struct Widget;

    async fn foo() {
    }

    async fn bar() {
        // valid code, since Widget is YieldSafe by default
        let w = Widget;
        foo().await
    }

    struct Gadget;

    impl !YieldSafe for Gadget {}

    async fn invalid() {
        // compile-time error, since Gadget is not YieldSafe
        let g = Gadget;
        foo().await
    }

    // need to explicitly mark type as YieldSafe
    async fn potentially_invalid(value: Box<dyn Debug + YieldSafe>) {
        foo().await
    }

Additionally, an `AssertYieldSafe` wrapper is introduced to mark enclosed variables as `YieldSafe`, thus avoiding
compile errors when they cross yield boundaries:

    use std::sync::Mutex;

    async fn foo() {
    }

    async fn bar() {
        let m = Mutex::new(0);

        // the following would result in a compile error, since MutexGuard is not YieldSafe
        //    let lock = m.lock();
        //    foo().await

        // the following compiles without errors, but the user is responsible for ensuring logical correctness
        let lock = AssertYieldSafe(m.lock());
        foo().await
    }

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

New `YieldSafe` default auto trait needs to be implemented for all primitive types and follow the same propagation rules
as other auto traits. Each control yield point would need to check if only `YieldSafe` objects are alive at that point.
Generic, `impl` and `dyn` types need also take yield safety into account, to avoid circumventing the system.

One example usage would be a standard `MutexGuard` which is error-prone to use across `.await` points. We would need to
identify other standard types which are similarly dangerous.

`AssertYieldSafe` is a counterpart of `AssertUnwindSafe` with the differences of implementing `YieldSafe` instead of
`UnwindSafe`/`RefUnwindSafe` and not implementing `Future`.

# Drawbacks

[drawbacks]: #drawbacks

A new auto trait might involve backwards incompatible changes, especially where generic types currently cross yield
points. The compiler could limit the problem by checking if the effective lifetime of objects of such types actually
cross yield boundary, e.g.

    async fn foo<T>(value: T) {
        // potentially valid code, since value doesn't need to cross .await if it doesn't implement Drop (recursively)
        bar().await
    }

We also could provide a grace period by emitting a warning instead of an error.

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

- Early solution involved a custom attribute which marked functions as potentially dangerous to call in an async
  context, but that solution proved to be inadequate.
- The effect of this solution is close to the one seen in C#.

# Prior art

[prior-art]: #prior-art

C# compiler detects awaiting on async methods inside lock-guarded code and emits an error in such case. Given a lock is
not a special instruction in Rust, it's better to provide a generic mechanism which solves the problem.

# Unresolved questions

[unresolved-questions]: #unresolved-questions

- How to handle more granular control, e.g. yield-unsafe functions?
- How to handle backwards compatibility better?

# Future possibilities

[future-possibilities]: #future-possibilities

This RFC paves a way for extended async code validation. Having syntactically valid code in async blocks is not enough -
we also need to provide a mechanism for logical validation to avoid runtime problems.
