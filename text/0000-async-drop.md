# Async Drop RFC

- Feature Name: `async-drop`
- Start Date: 2023-12-12
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: TBD

# Summary
[summary]: #summary

This RFC proposes the introduction of native async drop support in Rust. The feature aims to extend Rust's resource management capabilities to include asynchronous dropping of values, aligning the language more closely with the needs of asynchronous programming paradigms. Additionally, it acknowledges the existence of the [`async_dropper`](https://github.com/t3hmrman/async-dropper) crate, which provides an external implementation of async drop behavior. The RFC further proposes the introduction of an `AsyncDrop` trait similar to the existing `Drop` trait.

# Motivation
[motivation]: #motivation

The motivation behind introducing native async drop is to provide a standardized and integrated solution for asynchronous resource cleanup in Rust. While the `async_dropper` crate offers an external solution, having native support in the language simplifies the adoption of async drop functionality and ensures a consistent user experience. This feature addresses the gap in Rust's support for async lifetimes, making it more versatile for modern asynchronous applications.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Native async drop introduces the `async fn drop` method as part of the `Drop` trait, enabling asynchronous cleanup during the dropping of values. While the `async_dropper` crate provides a workaround, native support ensures a more seamless integration into Rust's syntax and semantics. We can leverage async drop without relying on external crates, simplifying the codebase and reducing dependency management overhead.

```rust
use async_trait::async_trait;
use tokio::time::{sleep, Duration};

// This trait should be implement internally
#[async_trait]
trait AsyncDrop {
    async fn drop(&mut self);
}

// Define an AsyncResource struct implementing AsyncDrop
struct AsyncResource {
    data: String,
}

// Implement AsyncDrop for AsyncResource
#[async_trait]
impl AsyncDrop for AsyncResource {
    async fn drop(&mut self) {
        println!("Async cleanup for AsyncResource: {}", self.data);
        // Asynchronous cleanup logic, e.g., releasing resources
    }
}

#[tokio::main]
async fn main() {
    {
        let _async_res = AsyncResource {
            data: String::from("Example Data"),
        };
        // drop will be triggered here
    }
    // AsyncResource is dropped as it goes out of scope, and its asynchronous drop logic may be executed later by the Tokio runtime

    // Sleep to ensure `drop` completed its execution
    sleep(Duration::from_secs(2)).await;
    println!("Main function completed");
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The design introduces native support for async drop through the addition of the `async fn drop` method within the `AsyncDrop` trait. This method is called when the value goes out of scope in an asynchronous context, providing a standardized way to handle asynchronous cleanup. The implementation ensures that the asynchronous drop logic is executed in the appropriate context, facilitating proper resource cleanup in async scenarios.

Additionally, this RFC proposes the introduction of an `AsyncDrop` trait, parallel to the existing `Drop` trait. This trait would enable Rust developers to define asynchronous cleanup behavior for their types.

```rust
trait AsyncDrop {
    async fn drop(&mut self);
}

struct CustomAsyncResource {
    // Fields of the struct

    // Implementing AsyncDrop for custom asynchronous cleanup
}

#[async_trait]
impl AsyncDrop for CustomAsyncResource {
    async fn drop(&mut self) {
        // Asynchronous cleanup logic
    }
}
```

# Drawbacks
[drawbacks]: #drawbacks

Introducing native async drop may increase the complexity of the language and impose additional implementation overhead. However, the benefits of providing a standard solution for asynchronous resource cleanup, along with the proposed `AsyncDrop` trait, outweigh these drawbacks.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The chosen design for native async drop and the introduction of the `AsyncDrop` trait are considered the best approach, as they align with Rust's focus on providing control over resources while accommodating the needs of asynchronous programming. While alternative solutions, such as relying on external libraries like `async_dropper`, exist, having native support ensures a more consistent and streamlined experience for Rust developers.

# Prior art
[prior-art]: #prior-art

The `async_dropper` crate provides an external implementation of async drop behavior. While this crate is a valuable resource, native support in Rust, along with the proposed `AsyncDrop` trait, reduces external dependencies and fosters a more integrated and standardized approach to asynchronous resource cleanup.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- The specific syntax and implementation details of native async drop and the proposed `AsyncDrop` trait may require further refinement through the RFC process.
- The impact on existing codebases and the interaction with other language features need thorough consideration.

# Future possibilities
[future-possibilities]: #future-possibilities

The introduction of native async drop and the `AsyncDrop` trait set the stage for potential extensions related to asynchronous resource management. Future possibilities include optimizations in the async drop process, integration with other async-related features, and exploration of async lifetimes to further enhance the language's capabilities in asynchronous scenarios.
