# Async Drop

- Feature Name: `async-drop`
- Start Date: 2023-12-12
- RFC PR: [rust-lang/rfcs#3541](https://github.com/rust-lang/rfcs/pull/3541)
- Rust Issue: TBD

## Summary
[summary]: #summary

This RFC is a comprehensive proposal to introduce native support for asynchronous resource cleanup in Rust through the `AsyncDrop` trait. The need for this enhancement arises from the growing significance of asynchronous programming paradigms in modern software development (e.g. Async cleanup on observable sequence termination) [^1^]. Currently, some developers are relying on external solutions like the [`async_dropper`](https://github.com/t3hmrman/async-dropper) crate to manage resources asynchronously [^2^]. This proposal aims to streamline and standardize asynchronous cleanup by introducing a native trait that allows developers to define custom asynchronous cleanup logic for their types.

The introduction of the `AsyncDrop` trait aligns with Rust's commitment to providing robust resource management capabilities while adapting to the evolving needs of the programming landscape. By integrating native support for asynchronous resource cleanup, Rust aims to simplify the codebase and reduce dependency management overhead associated with external crates. The proposed enhancement focuses on empowering Rust developers to write more expressive, maintainable, and performant asynchronous code, enhancing Rust's suitability for modern software development.

## Motivation
[motivation]: #motivation

The motivation behind this RFC is rooted in the increasing prevalence of asynchronous programming and the challenges developers face in managing resources within this paradigm. Asynchronous programming introduces unique complexities, especially regarding resource cleanup. The existing reliance on external solutions, such as the `async_dropper` crate, highlights the need for a standardized and integrated solution within the Rust language. The motivation is not just to address a specific pain point but to provide a holistic and coherent approach to asynchronous resource management, reinforcing Rust's position as a language that evolves with the changing needs of developers.

Consider a scenario where a database connection is established asynchronously, and proper cleanup is necessary when the connection goes out of scope. With the proposed `AsyncDrop` trait, developers can encapsulate the necessary cleanup logic within the type, promoting encapsulation and code organization. For instance:

```rust
use async_trait::async_trait;

#[async_trait]
trait AsyncDrop {
    async fn drop(&mut self);
}

struct DatabaseConnection {
    // Fields of the struct

    // Implementing AsyncDrop for custom asynchronous cleanup
}

#[async_trait]
impl AsyncDrop for DatabaseConnection {
    async fn drop(&mut self) {
        // Asynchronous cleanup logic for releasing database resources
    }
}
```

This example illustrates how the `AsyncDrop` trait allows developers to encapsulate asynchronous cleanup logic within the type, fostering a more modular and organized code structure.
**Detailed Explanation:**

let's now consider a real-world example. The [`IggyClient`](https://github.com/iggy-rs/iggy/blob/master/iggy/src/clients/client.rs#L78) type within the `Iggy` crate stands as a concrete representation of a client entity in the broader context of the `Iggy` crate. As part of the ongoing effort to align with modern asynchronous programming paradigms, the developers have sought to implement the `AsyncDrop` trait for the `IggyClient` type. This trait, decorated with the `#[async_trait]` attribute, signifies its ability to handle asynchronous cleanup operations when instances of the `IggyClient` type go out of scope.

The central piece of this implementation is the `async_drop` method defined within the `AsyncDrop` trait for the `IggyClient`. This method encapsulates the logic responsible for initiating asynchronous cleanup. Specifically, it orchestrates a logout operation for the client by calling the `logout_user` method on the asynchronously obtained client reference. This operation, in turn, triggers the execution of the logout process, facilitating proper resource cleanup associated with the `IggyClient`.

The crucial aspect highlighted by this example is the reliance on the `async_dropper` crate as a pragmatic workaround. In the absence of native support for asynchronous dropping in Rust at the time of this implementation, developers turn to external solutions to fulfill the need for proper asynchronous resource cleanup. The `async_dropper` crate, which is not part of the standard Rust library, offers a mechanism to define and execute asynchronous cleanup logic when values go out of scope.

```rust
#[async_trait]
impl AsyncDrop for IggyClient {
    async fn async_drop(&mut self) {
        let _ = self.client.read().await.logout_user(&LogoutUser {}).await;
    }
}
```

In this code snippet, the `IggyClient` type implements the `AsyncDrop` trait, and the `async_drop` method encapsulates the logout logic. The method is asynchronous, denoted by the `async` keyword, allowing it to seamlessly integrate into asynchronous codebases. The usage of the `async_dropper` crate encapsulates the asynchronous cleanup process, adhering to the limitations posed by Rust's existing support for asynchronous resource management.

This implementation serves as a testament to the practical challenges faced by developers in handling asynchronous resource cleanup and the innovative approaches they employ, such as leveraging external crates like `async_dropper`. The existence of such workarounds underscores the importance of introducing native async drop support in Rust, providing a standardized, integrated, and idiomatic solution to address these challenges.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

### Async Drop Trait

The proposed `AsyncDrop` trait serves as the cornerstone of this enhancement. Similar to the existing `Drop` trait, it enables developers to define custom asynchronous cleanup logic for their types. This trait is annotated with the [`async_trait`](https://docs.rs/async-trait) attribute to indicate its compatibility with asynchronous operations. For example:

```rust
use async_trait::async_trait;

#[async_trait]
trait AsyncDrop {
    async fn drop(&mut self);
}
```

Here, the `AsyncDrop` trait declares a single method, `async fn drop(&mut self)`, which developers can implement to define asynchronous cleanup logic specific to their types.

### Async Drop in Structs

Understanding how native async drop operates within a struct is crucial for developers. In Rust, the compiler ensures that fields within a struct are dropped in a Last-In-First-Out (LIFO) order. To illustrate, consider the following example:

```rust
struct DataStorage {
    data: AsyncData,
    index: AsyncIndex,
}

async fn process_data() -> usize {
    let storage = DataStorage::new();
    return storage.process().await;
}
```

In this scenario, the `DataStorage` struct contains two asynchronous resources, `AsyncData` and `AsyncIndex`. The Rust compiler guarantees that these fields are dropped in a LIFO order when the `DataStorage` instance goes out of scope, facilitating predictable and proper asynchronous resource cleanup (`index` is droped first, followed by the `data` field).

### Async Drop Invocation

For futures with async drop logic, executors must recognize the `AsyncDrop` trait and invoke the `drop` method appropriately. This involves integrating async drop invocation into the executor's lifecycle. Consider the following example:

```rust
async fn execute_future<T: AsyncDrop + Future>(mut fut: T) {
    // Execute the future
    let result = fut.await;

    // Perform async drop before releasing resources
    fut.drop().await;
}
```

In this example, the `execute_future` function takes a generic parameter `T` that must implement both the `AsyncDrop` trait and the `Future` trait. After the future completes its execution, the executor invokes the `drop` method on the future, ensuring proper resource cleanup. This explicit invocation aligns with Rust's emphasis on manual control over resource management.

### Context Propagation

Maintaining a consistent execution context for async drops is crucial to ensure predictable and coherent asynchronous resource cleanup. To achieve this, executors should propagate the appropriate `Waker` and `Context` information. Here's an example illustrating context propagation:

```rust
async fn execute_future<T: AsyncDrop + Future>(fut: T) {
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

In this example, the `execute_future` function sets up a `Waker` with the `noop` method from the `Waker` trait, creating a placeholder waker with no associated behavior. This waker is then used to create a `Context`, and the future is executed within this context. After the future is polled, the `drop` method of the future is invoked, ensuring that the async drop logic is executed within the same context established earlier. This crucial context propagation guarantees consistent and predictable execution of asynchronous drop logic.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The design introduces native support for async drop through the addition of the `async fn drop` method within the `AsyncDrop` trait. This method is called when the value goes out of scope in an asynchronous context, providing a standardized way to handle asynchronous cleanup. The implementation ensures that the asynchronous drop logic is executed in the appropriate context, facilitating proper resource cleanup in async scenarios.

Additionally, the introduction of the `AsyncDrop` trait, running in parallel with the established `Drop` trait, represents a pivotal advancement in Rust's capabilities for managing resources in asynchronous scenarios. This trait, defined with the asynchronous `drop` method, empowers Rust developers to articulate specialized asynchronous cleanup procedures for their custom types. Asynchronous cleanup logic, encapsulated within the `drop` method, can address the complexities of releasing resources in a concurrent and non-blocking fashion, contributing to more expressive and maintainable asynchronous code.

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

In this example, the type `CustomAsyncResource` implements the `AsyncDrop` trait, showcasing how developers can leverage this trait to define custom asynchronous cleanup logic. The `drop` method within the implementation becomes the designated space to orchestrate the necessary cleanup operations for instances of `CustomAsyncResource`. This addition to Rust's feature set not only aligns with the language's commitment to safety and efficiency but also reflects its adaptability to the evolving landscape of modern asynchronous programming paradigms.

# Drawbacks
[drawbacks]: #drawbacks

While the introduction of native async drop enhances Rust's support for asynchronous programming, potential drawbacks include an increase in language complexity and potential additional implementation overhead. The complexity may arise from the need to manage asynchronous resource cleanup alongside other language features, potentially impacting the learning curve for new developers. Moreover, the implementation overhead may affect the overall performance of the language, requiring careful consideration during the design and implementation phases.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The chosen design for incorporating native async drop and introducing the `AsyncDrop` trait is considered the optimal approach, aligning cohesively with Rust's overarching emphasis on resource control and responsiveness to the demands of asynchronous programming. This design is driven by the objective of presenting a standardized solution that seamlessly integrates into the current set of language features. The aim is to develop a unified and intuitive development experience, streamlining the process for developers as they navigate the complexities of managing resources asynchronously.

As mentioned, an alternative approach could involve further reliance on external crates like `async_dropper`. While external solutions can address the immediate need for asynchronous resource cleanup, they may introduce fragmentation within the Rust ecosystem, leading to inconsistencies in coding practices. Native support ensures a more consistent and standardized approach, reducing external dependencies and making Rust more self-contained.

# Prior art
[prior-art]: #prior-art

The `async_dropper` crate provides an external implementation of async drop behavior. While this crate is a valuable resource, native support in Rust, along with the proposed `AsyncDrop` trait, reduces external dependencies and fosters a more integrated and standardized approach to asynchronous resource cleanup.

Considering prior art involves examining practices from other programming languages supporting native async drop or similar features. Analyzing how languages like C# handle asynchronous resource cleanup offers valuable insights into best practices and potential challenges. However, it's imperative to tailor any insights from other languages to align with Rust's unique design principles and community expectations.

In the exploration of prior art, a relevant resource is the article titled "Learn .NET Advanced Programming Memory Management" [^3^]. This article delves into the implementation of advanced memory management techniques in `.NET` and introduces the `System.IAsyncDisposable` interface, a feature integrated as part of C# 8.0. Specifically, the article comprehensively covers the implementation of the `IAsyncDisposable.DisposeAsync` method, enabling resource cleanup with the capability for asynchronous operations. Notably, this method returns a `ValueTask`, symbolizing the asynchronous disposal operation. The article also delves into the practice of implementing both synchronous and asynchronous disposal, with the `IAsyncDisposable` interface designed to accommodate either scenario. While it's not obligatory to incorporate both types of disposal, the guidance for implementing the disposal/drop pattern remains consistent, assuming a foundational understanding of implementing a Dispose method.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Several aspects of the proposed enhancement may require further exploration and refinement through the RFC process:

1. **Syntax and Implementation Details:**
   The specific syntax and implementation details of native async drop and the proposed `AsyncDrop` trait may require iterative refinement. Ensuring that the chosen syntax aligns with Rust's overall design philosophy and is intuitive for developers is crucial.

1. **Impact on Existing Codebases:**
   The potential impact on existing codebases needs careful consideration. Developers should be able to seamlessly transition to the new feature without significant disruptions. Compatibility with existing code and libraries is crucial.

1. **Performance Considerations:**
   The performance implications of introducing native async drop need careful examination. Ensuring that the feature does not introduce undue overhead and aligns with Rust's commitment to efficiency is critical.

1. **Community Feedback and Adoption:**
   The success of the native async drop feature depends on community acceptance and adoption. Soliciting and incorporating feedback from the Rust community during the RFC process is essential to address concerns, refine design choices, and ensure widespread support.

# Future possibilities
[future-possibilities]: #future-possibilities

The introduction of native async drop and the `AsyncDrop` trait lays the foundation for potential future extensions and optimizations related to asynchronous resource management in Rust. Several possibilities include:

1. **Optimizations in Async Drop Process:**
   Future iterations could explore optimizations in the async drop process. Enhancements to make asynchronous resource cleanup more efficient and responsive to different scenarios could be considered.

1. **Integration with Other Async Features:**
   The proposed feature sets the stage for potential integration with other asynchronous features in Rust. Exploring synergies with async lifetimes or async-specific constructs could further enhance the language's capabilities.

1. **Tooling and Documentation Improvements:**
   As the feature matures, improvements in tooling and documentation can provide developers with better support and guidance. Tools for analyzing and optimizing asynchronous resource cleanup could be developed, and comprehensive documentation can aid in widespread adoption.

1. **Ecosystem-wide Adoption:**
   Encouraging and supporting the adoption of native async drop across the Rust ecosystem is crucial. Establishing best practices, providing migration guides, and fostering community awareness can contribute to the widespread and successful adoption of the feature.

1. **Feedback-Driven Refinement:**
   Ongoing refinement based on community feedback and real-world usage is integral to the success of the feature. Regular feedback loops with the Rust community will help identify areas for improvement, address unforeseen challenges, and ensure the feature remains aligned with evolving programming practices.

In conclusion, the proposed native async drop feature is not just a standalone enhancement but a step towards a more versatile and resilient asynchronous programming experience in Rust. As the feature evolves, it opens up exciting possibilities for the language, shaping its trajectory in the dynamic landscape of modern software development.

[^1^]: Async cleanup on observable sequence termination, Stack Overflow thread, Asked 6 years, 4 months ago. [Link](https://stackoverflow.com/questions/45407562/async-cleanup-on-observable-sequence-termination)

[^2^]: Iggy-Rs. (n.d.). iggy/iggy/src/clients/client.rs at master · iggy-rs/iggy. GitHub. https://github.com/iggy-rs/iggy/blob/master/iggy/src/clients/client.rs#L797-L802

[^3^]: IEvangelist. (2023, May 12). Implement a DisposeAsync method - .NET. Microsoft Learn. https://learn.microsoft.com/en-us/dotnet/standard/garbage-collection/implementing-disposeasync
