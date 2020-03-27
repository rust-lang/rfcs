- Feature Name: `no_async`
- Start Date: 2020-03-27
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This feature enables tagging functions as dangerous/error-prone to run within an async context.

# Motivation
[motivation]: #motivation

Some functions are not safe to call within an async context. A trivial example might be locking
a standard mutex in an async block - without proper care we risk deadlocking the process. Rust
currently doesn't check such situations at compile time, which might lead to hard to debug runtime
errors. While it's true such dangerous calls are logic errors, not related to language itself,
it would be very helpful if programmers could mark functions as potentially hazardous to call in
async code, which would result in a compile warning. This mechanism is similar to detecting async
code inside locks in C#.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This feature adds a new `#[no_async]` attribute which is permitted on functions. Whenever a call
to such annotated function is called inside a `impl Future` code, a warning is raised to indicate
a potentially dangerous operation:

    #[no_async]
    fn foo1() {
    }

    #[no_async("custom message")]
    fn foo2() {
    }

    async fn bar() {
        // generates a generic compile-time warning
        foo1();

        // generates a "custom message" compile-time warning
        foo2();
    }

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

In each `impl Future` code, including implicitly generated, a check needs to be made at function calls
for the presence of the new attribute, and a warning needs to be raised if such is present.

# Drawbacks
[drawbacks]: #drawbacks

Checking for such attribute at call sites might increase compile time.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This attribute enables programmers to explicitly communicate which API is not intended to be used in async
code. Alternatively, we can make this process automatic (which is not trivial) or keep the situation as it
is now - no checking and potential runtime errors. Having an attribute seems like an easy solution, which 
can be used by external library authors.

# Prior art
[prior-art]: #prior-art

C# compiler detects awaiting on async methods inside lock-guarded code and emits an error in such case. Given
a lock is not a special instruction in Rust, it's better to provide a generic mechanism which solves the
problem the other way around.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Maybe a hard error is more appropriate than a warning?

# Future possibilities
[future-possibilities]: #future-possibilities

This RFC paves a way for extended async code validation. Having syntactically valid code in async blocks is
not enough - we also need to provide a mechanism for people to add logical validation to avoid runtime
problems.
