- Start Date: 09/09/2014
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Expand the `#[test]` and `#[bench]` features to include before and after hooks.

# Motivation

Rust's current testing and benchmarking system is an extremely valuable part of
the language, as it encourages even new users to write their own tests and
benchmarks without introducing the overhead of learning a new testing
framework.

However, in an effort to be simple, it also makes writing tests or benchmarks
that need to work around certain API constraints very difficult - mostly those
which require one or many time initialization and cleanup.

In other languages, this problem is solved by hooks exposed by outside testing
frameworks which give before, after, before\_each, and after\_each hooks. There
have been attempts to use macros to provide similar functionality in Rust, and
they mostly succeed in providing before\_each and after\_each. But, because
rustc doesn't provide the appropriate hooks, they cannot implement before and
after.

This RFC seeks to remedy this by providing these hooks and making them
ergonomic to use.

# Detailed design

Add `@before_tests`, `@after_tests`, `@before_benches`, and `@after_benches`.

Each of these directives applies to tests and benchmarks defined in the same
module - not any parent or submodules.

The general use and implementation of these attributes is most easily explained
through an example (this example uses the attribute syntax from RFC #208):

```rust
// Our tests, in their own module.
mod test {
    // This will be run once, before all the tests in this module are run.
    //
    // This block must evaluate to ()
    @before_tests {
        let server = start_server(3000);
    }

    // This will be run once, after all the tests in this module have run,
    // regardless of their status.
    //
    // This block must evaluate to ()
    @after_tests {
        // This block is glued together with the before_tests! block, so it
        // has access to variables defined there.
        server.close();
    }

    // Traditional tests.
    @test fn test_server() {
        request(3000).unwrap()
    }

    @test fn test_server2() {
        assert!(request(3000).is_err())
    }

    mod subtests {
        // This might or might not run between the above before_tests and
        // after_tests. Since it is in a different module it is run separately.
        @test fn test_something_else() {
            assert_eq!(78u, 78u);
        }
    }
}
```

`@before_benches` and `@after_benches` act the same way, but with `@bench`.

This provides the necessary hooks for downstream test frameworks to provide
features such as `before_each`, while also allowing them to provide before
and after hooks, which is impossible right now.

# Drawbacks

It complicates the now extremely simple testing and benchmarking system.

# Alternatives

We could continue to disallow this behavior, or leave users to use things like
ONCE to do initialization, though that still does not allow for cleanup after
all tests have run.

# Unresolved questions

The exact syntax of the hooks could change, for instance whether they should be
syntax extensions or attributes, and how much magic they should have.

