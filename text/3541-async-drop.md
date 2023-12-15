# Async Drop RFC

- Feature Name: `async-drop`
- Start Date: 2023-12-12
- RFC PR: [rust-lang/rfcs#3541](https://github.com/rust-lang/rfcs/pull/3541)
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

Moreover, to seamlessly integrate native async drop support into Rust, it is essential to outline how executors should handle asynchronous resource cleanup. This section also provides guidelines for developers using executors to ensure consistent and correct execution of async drop logic.

1. **Recognition of AsyncDrop Trait**
   Executors, such as `MyExecutor` in the following example, should be updated to recognize types implementing the `AsyncDrop` trait. This can be achieved through a mechanism such as trait bounds or associated types to identify futures that require special handling for asynchronous cleanup.

   ```rust
   trait Executor {
       fn spawn(&self, future: impl Future);
   }

   impl<T: AsyncDrop> Executor for MyExecutor {
       // Your custom cleanup logic goes here
   }
   ```

   Here, `T: AsyncDrop` indicates that this executor can work with futures (`T`) that implement the AsyncDrop trait. The `AsyncDrop` trait serves as a marker, signaling that the associated type requires special handling for asynchronous cleanup.

1. **Async Drop Invocation**
   Executors must ensure that the async `drop` method is invoked when a future implementing the `AsyncDrop` trait goes out of scope or completes. This involves tracking the lifecycle of futures and, upon termination, triggering the async drop logic before releasing associated resources.

   ```rust
   async fn execute_future<T: AsyncDrop>(&self, fut: T) {
       // Execute the future
       let result = block_on(fut);
   
       // Perform async drop before releasing resources
       fut.drop().await;
   }
   ```

   The `futures::executor::block_on` function is employed to execute the future (`fut`), pausing the current thread until the future completes and produces a result. Following the completion of the future's execution, the executor invokes the `drop` method on the future (`fut`). This ensures the execution of asynchronous drop logic, allowing the future to perform any necessary cleanup operations before resources are released.

1. **Context Propagation**
   To maintain a consistent execution context for async drops, executors should propagate the appropriate `Waker` and `Context` information to the async drop method. This ensures that async drops can interact with the executor environment and make executor-specific decisions during cleanup.

   ```rust
   async fn execute_future<T: AsyncDrop + Future + Copy>(&self, fut: T) {
       // Set up the execution context
       let waker = Waker::noop();
       let mut cx = Context::from_waker(&waker);
       
       // Execute the future with the provided context
       let mut pinned = std::pin::pin!(fut);
       let _ = pinned.as_mut().poll(&mut cx); 
       
       // Perform async drop within the same context
       fut.drop().await;
   }
   ```

   After the future is polled, the `drop` method of the future (`fut`) is invoked. This ensures that the async drop logic is executed in the same context established earlier.

Here is a fully working example:

```rust
#![feature(noop_waker)]

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

// AsyncDrop trait that should be implemented internally
trait AsyncDrop {
    async fn drop(&mut self);
}

// Future type for demonstration purposes
#[derive(Clone, Copy)]
struct MyFuture;

impl Future for MyFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Your custom future's poll logic goes here
        Poll::Ready(())
    }
}

impl AsyncDrop for MyFuture {
    async fn drop(&mut self) {
        // Your asynchronous cleanup logic goes here
        println!("Async cleanup executed");
    }
}

// Custom executor
async fn execute_future<T: AsyncDrop + Future + Copy>(mut fut: T) {
    // Set up the execution context
    let waker = Waker::noop();
    let mut cx = Context::from_waker(&waker);

    // Execute the future
    let mut pinned = std::pin::pin!(fut);
    let _ = pinned.as_mut().poll(&mut cx);

    // Perform async drop
    fut.drop().await;
}

#[tokio::main]
async fn main() {
    let my_future = MyFuture;
    execute_future(my_future).await;
}
```

By following these recommendations, executors can seamlessly integrate with the native async drop support, providing a standardized and reliable mechanism for handling asynchronous resource cleanup in Rust. These guidelines ensure that async drops are executed appropriately within the executor's lifecycle, enhancing the overall consistency and predictability of asynchronous resource management in Rust.

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
