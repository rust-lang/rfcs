- Feature Name: `non_const_drop_check`
- Start Date: 2022-10-17
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)
- **Status:** Pre-RFC

# Summary
[summary]: #summary

Warn when a type that does not implement [`core::marker::Destruct`](https://doc.rust-lang.org/nightly/core/marker/trait.Destruct.html) is dropped.

This extends the `const Drop` check of the [const_trait_impl](https://github.com/rust-lang/rust/issues/67792) proposal to non-const functions, though with a warning rather than an error in case of failure.

Implement non-const `core::marker::Destruct` for most types in the standard library. Add support for `#[derive(Destruct)]`.

# Motivation
[motivation]: #motivation

Scope based implicit drop is a user-friendly way to make cleaning up resources properly easy, and leaking resources hard. For most cases, this works exceedingly well. However, in some APIs it would be nice to force the API user to call some method rather than allowing the implicit drop to clean up (For example, to force the user to think about errors on cleanup, or to recieve extra data needed for cleanup).

A workaround used today is panicking on drop (a "drop bomb"), but this is a runtime check for what could be checked at compile time. Having a strong type system that catches many errors at compile time is one of Rust's strengths; it makes sense to allow types to opt out of implicit drop when they decide that implicit drop is wrong.

Unwinding complicates this picture: for an unwinding panic, the compiler needs to clean up everything on the stack without a chance for the user to provide or recieve data. `Drop::drop` is a good fit for this case: it lets types specify custom cleanup behavior and reduces the amount of boilerplate by recursively dropping the subvalues.

In many cases, is also convenient to have cleanup code that is automatically called when a value goes out of scope, and we want to do the same thing on unwinding and end of scope often enough that it makes sense to combine those two facilities. But while every type needs to handle cleanup on unwind, it is reasonable to declare that, for some types, we don't want the convenience of implicit drop on end of scope.

Unwinding panics are an exceptional case. It is important to handle them without breaking Rust's memory safety guarantees, and useful to allow types to customize their behavior when dropped because of unwinding. But silently and implicitly making every variable which is not consumed before the end of its scope do the same thing (you can check `thread::panicking`, but still) as when handling an unwinding panic is frustrating when the best-effort cleanup can you can do leaks memory, silently ignores errors, or sends a placeholder value. The current use of `drop` combines the exceptional case of unwinding with the extremely common case of ending a scope: we don't necessarily want to do the same thing for both, and don't need to use the API forced by the limitations of unwinding panic in the case of non-exceptional control flow.

All of the above being said, breaking existing Rust code should be avoided. A warning (easy to turn off, easy to turn into a hard error, on a crate by crate or even finer grained level) achieves the purpose of alerting the user when they have accidentally dropped a type which very much does not want to be.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## The `Destruct` Trait

Most types take care of their own cleanup when they go out of scope. A `Vec` deallocates its storage, a `MutexGuard` unlocks the mutex, a `Rc` or `Arc` decrements the reference count, and so on, along with types that don't need any cleanup like integers. These types implement the `Destruct` trait, which means you can drop them whenever you want just by letting their scope expire, or calling `mem::drop`.

However, some types would much rather have an explicit step before they go away. This could take the form of a file that might fail to close and wants to tell you about it, or a request that needs your signature as the last step before completion (and you don't want to just forget about it accidentally), among other possibilities. Types that want an explicit final step do not implement the `Destruct` trait.

If a value of a type that does not implement the `Destruct` trait goes out of scope without being consumed, by default a warning will occur. You are recommended to treat this warning into an error. It is strongly recommended that you do not silence the warning, and instead suggest that you find a function to consume your value. That way the appropriate cleanup can be performed. In the rare case that you actually want to leak an object, `mem::forget` works for all values.

Unlike `Sized`, `Destruct` is not assumed by default for type parameters and associated types. If you are implementing a generic API and need to drop values implicitly, you should add the `Destruct` bound to your type variables.

## Writing types that don't implement `Destruct`

As an example of an API that might choose to make use of non-`Destruct` types, consider [`BufWriter`](https://doc.rust-lang.org/std/io/struct.BufWriter.html). Quoting from the documentation, "It is critical to call flush before `BufWriter<W>` is dropped. Though dropping will attempt to flush the contents of the buffer, any errors that happen in the process of dropping will be ignored. Calling flush ensures that the buffer is empty and thus dropping will not even attempt file operations." This is an example where the implicit drop leads to a footgun: rather than dropping, you want to call a method that first tries to flush, closes if successful, and if there was an error returns back to the caller to handle the error.

The relevant parts of the interface might look like:
```rust

// BufWriter does not #[derive(Destruct]
pub struct BufWriter<W: Write>
{
    inner: W, // the inner writer
    buf: Vec<u8>, // the buffer
}

impl<W: Write> Write for BufWriter<W> {
    fn write(&mut self, buf: &mut [u8]) -> Result<usize>;
    fn flush(&mut self) -> Result<()>;
}

impl<W: Write> Write for BufWriter<W>
where
    W: Destruct // For this example, we will assume that dropping the inner writer is how to close it. There are other reasonable choices here.
{
    // On error, returns both the error code and the self parameter, which has not yet been closed.
    // The user can choose to try again, log the error, or do something else, and can even continue writing to this BufWriter<W>.
    fn close(self) -> Result<(), (IoError, Self)> {
        match self.flush() {
            Ok(()) => {
                // We have just flushed, so it is fine to close without flushing first.
                self.close_without_flush(); // Consume self.
                Ok()
            },
            Err(e) => Err((e, self)) // An error happened, don't drop anything yet.
        }
    }

    // Close without flushing the buffer.
    // This operation is likely to lose data written since the last flush.
    // Generally prefer using `close` instead.
    fn close_without_flush(self) -> () {
        mem::drop(self.inner); // Perform the close by making a partial move out of self and dropping the inner writer.
        ()
    }

    // Close even if flushing causes an error, but report the error.
    fn close_check_error(self) -> Result<()> {
        self.close().map_err(|(e, self_)| {
            self_.close_without_flush();
            e // Forward the error value
        })
    }
}

impl<W: Write> Drop for BufWriter<W> {
    // BufWriter does not implement Destruct, but because implicit drop only emits a warning, we can still be dropped at any time.
    // The best we can do is to flush ourselves, but we have to give up on reporting the error.
    fn drop(&mut self) {
        let _e = self.flush();
    }
}
```

Then to use it, you would need to call `close` to close the writer: if the writer goes out of scope a warning will be issued, which users are recommended to treat as an error. This makes it easy to do the correct thing (handle the errors), and makes it so that ignoring errors on close is intentional, not an accident.
```rust
1:  fn main() -> Result<(), IoError> {
2:      let mut writer = BufWriter::create(...)?;
3:      writer.write("Hello")?;
4:      writer.close_check_error()?
        // Omitting the `close_check_error` call causes a compiler warning:
        // value writer should not be implicitly dropped; type BufWriter<...> does not implement Destruct.
        // value created on line 2, scope ends on line 5.
        // last use does not consume value.
5:  }
```

If there is a `panic!` before the writer is closed, `Drop::drop` will be called as normal: in this case you don't have an opportunity to catch any errors reported by file close because you are already panicking.

Unsafe code can make no new assumptions.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

TODO: This section is not yet written

## Corner cases

### Types with no drop glue but that do not implement `Destruct`.

Even if no component of the type implements `Drop`, so there is no drop glue for the type, the compiler must still produce an error if a type that does not implement `Destruct` is to be dropped. This impacts [the algorithm for elaborating open drops](https://rustc-dev-guide.rust-lang.org/mir/drop-elaboration.html#open-drops), which says "Fields whose type does not have drop glue are automatically Dead and need not be considered during the recursion." In this proposal, only fields that do not have drop glue but do implement `Destruct` can be automatically dead.

This only applies if the computation of when to emit a warning happens during this algorithm. I believe `rustc` currently has a separate pass for `const Drop` checking, which might be a better place to extend to include this feature.

Relevant previous PR: https://github.com/rust-lang/rust/pull/68943

### Partial moves

It should be possible to partially move out of a non-`Destruct` type that does not implement `Drop`. The semantics are the same as today, that any fields which are not moved get dropped (if any of those fields are themselves non-`Destruct`, throw a warning). Similarly destructuring should work.

I would like private fields to block partial moves, but that seems to be a change from how Rust works today. Types that want to block partial moves may need a dummy `Drop` implementation. If the warning message is good enough, having a private non-`Destruct` field may be sufficient, as a warning will be produced if it is dropped as part of a public partial move.

## Destructuring types which do not implement `Destruct`

to resolve: before implementation

If a type has an explicit negative impl for `ScopeDrop`, should we allow or forbid partial moves and destructuring?
I lean towards allowing partial moves from these types, which should be rare.

Most types will use `PhantomIndestructible`, and putting this in a private field effectively prevents partial moves out of your type because you cannot move out the marker and thus the compiler will attempt to drop it, producing an error.

# Drawbacks
[drawbacks]: #drawbacks

* Because `Destruct` is not assumed by default, adapting crates to this feature potentially requires lots of lines of code changes, particularly to crates that commonly drop generic values.
* These are not linear types. This does not eliminate the use of `mem::forget`, so APIs like scoped thread guards are still broken: unsafe code cannot rely on destructors of any kind to be run for safety.
* Adds another auto trait, which adds another piece of magic that happens without the user explicitly requesting it.

# Rationale and alternatives

The blog post that inspired this RFC: http://aidancully.blogspot.com/2021/12/less-painful-linear-types.html

Miscelaneous, non-exhaustive collection of similar prior proposals:
* http://aidancully.blogspot.com/2021/12/less-painful-linear-types.html
* https://users.rust-lang.org/t/private-drop-or-rust-could-be-better-at-raii-with-a-rather-small-change/12322
* https://github.com/rust-lang/rfcs/issues/814
* https://github.com/rust-lang/rfcs/issues/523 (this feature request would seem to be resolved by this proposal)
* https://github.com/rust-lang/rfcs/issues/2642 (an approach with a variation on `#[must_use]`)
* https://internals.rust-lang.org/t/pre-pre-rfc-nodrop-marker-trait/15682

## Unwinding

This is not a proposal for linear types, although hopefully this proposal brings some of the advantages people are looking for in linear types.

How to deal with unwinding is one of the issues that has complicated previous proposals for linear types in Rust. When unwinding is possible, at almost any point the stack could need to be cleaned up: what is to be done with linear types in this case? In postponed RFC https://github.com/rust-lang/rfcs/issues/814, a `Finalize` trait was proposed that behaves identically to `Drop` but is only called in the unwinding case. Here we propose simply reusing the `Drop` trait for custom cleanup in both the unwinding and scope drop cases: If you need different behavior you can check `thread::panicking`.

This is a bit of a "punt": we still allow non-`Destruct` types to be scooped up and disposed of by panic at any point, without any compiler-emitted warnings. I argue that doing so makes integrating `Destruct` for gating implicit drops into Rust much less difficult than trying to forbid contexts where panic is possible.

Types can individually decide whether they want to abort, to leak, or to attempt a best-effort cleanup in the exceptional case of unwinding while being confident that users will not accidentally default to this suboptimal behavior.

## `mem::forget`
[memforget]: #memforget

Should `mem::forget::<T>` require `T: Destruct`? I don't believe so.

If the bound `Destruct` is added to all type parameters and associated types in the standard library, I do not see a way to write
`mem::forget` without using `unsafe`. However, I do not think that this is a promise we want to make.

There are many APIs for which obvious generalization to non-`Destruct` types allows writing `mem::forget` in safe code. For example, while `Rc` needs to destroy the value it is holding at an unpredictable time, as long as we provide a method `fn destroy(self) -> ()` this is a seemingly sound system. But in the case of a reference cycle, `destroy` never gets called, so choosing `fn destroy(self) { panic!() }` will allow writing `mem::forget` in safe code.

As another example, it seems eminently reasonable to pass non-`Destruct` values to a separate thread: the new thread takes ownership which entails responsibility for cleaning up. But if the new thread goes into an infinite loop, then the value never gets cleaned up and we can again safely forget values.

For these reasons, I argue that `mem::forget` should not require `T: Destruct`.

## Why a trait?

Rather than changing the type system, we could consider using a lint to discover when a type we annotate gets implicitly dropped. However, to do this in a compositional way requires computing for generic functions which types they may implicitly drop, and we would want to propagate this information across crates. At this point, it seems clear that adding a lint for implicitly dropped values would require computing and communicating much the same data as using a trait, and using a trait gives a systematic way of integrating the feature into the language.

## Branded types

Branded types allow the function return type to require the production of a value from the specific input parameter, rather than any value of the same type. This lets you encode "You must use *this* item" rather than "You must use *a* item". I see this as solving a different problem. This proposal is about how to avoid accidentally calling drop ever, and is not tied to the lifetime of a single function.

## What are the consequences of not doing this?

If we do not adopt this proposal, types for which implicit drop is a footgun will remain reliant on runtime checking. The case of accidental async future cancellation by drop has been brought up. Another example of an API that would benefit from non-`Destruct` is when you are expected to eventually complete every recieved request with a completion status (the example I am familiar with is Windows Driver Framework requests: https://docs.microsoft.com/en-us/windows-hardware/drivers/wdf/completing-i-o-requests).

The issue of disabling implicit drops seems to come up frequently enough to demonstrate some level of desire for this feature in the Rust community. This feature does require language support: I do not believe it is possible to faithfully emulate compile-time checked `Destruct` without compiler support.

# Prior art
[prior-art]: #prior-art

Vale has `!DeriveStructDrop`, which looks like a similar feature. This blog post [Higher RAII](https://verdagon.dev/blog/higher-raii-7drl), has interesting examples of real-world applications of this feature.

TODO: talk about linear-rust

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## `Destruct` in the Standard Library
[destruct-in-the-standard-library]: #destruct-in-the-standard-library

Modifying the standard library should give a sense of how intrusive this feature is. The question comes down to how often do functions drop values of generic type? A prototype implementation that ignores the complexity of open drops and just errors on static drops of non-`Destruct` types should give a good sense of this.

## `Copy: Destruct`
[copy-destruct]: #copy-destruct

to resolve: after implementation

If the compiler copies types that implement `Copy` too freely, it may be easy to end up with extra copies that need to be consumed and aren't. It might improve usability to require `Copy: Destruct`. In an extension to the restriction that a type which implements `Copy` mustn't have any drop glue, we would essentially be requiring that `Copy` types can be silently dropped as well. This closes off the possibility of having copyable relevant (use at least once) types, but there is no problem with `Clone + !Destruct`.

This would mean that `Copy` types are fully structural, with implicit duplication and dereliction.

# Future possibilities
[future-possibilities]: #future-possibilities

# Documentation Notes
[documentation-notes]: #documentation-notes

A list of places to add documentation for this feature to.

* Add `Destruct` to the list of auto traits in https://doc.rust-lang.org/stable/reference/special-types-and-traits.html#auto-traits
* TODO: fill out this list
