- Feature Name: `implicit_drop_warning`
- Start Date: 2022-10-17
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)
- **Status:** Pre-RFC

# Summary
[summary]: #summary

Add an attribute `#[must_cleanup]` for structs and traits, and warn when a value of a type marked with the `#[must_cleanup]` attribute is dropped.

# Motivation
[motivation]: #motivation

Scope based implicit drop is a user-friendly way to make cleaning up resources properly easy, and leaking resources hard. For most cases, this works exceedingly well. However, in some APIs it would be nice to force the API user to call some method rather than allowing the implicit drop to clean up (for example, to force the user to think about errors on cleanup, or to recieve extra data needed for cleanup).

A workaround used today is panicking on drop (a "drop bomb"), but this is a runtime check for what could be checked at compile time. Having a strong type system that catches many errors at compile time is one of Rust's strengths; it makes sense to allow types to opt out of implicit drop when they decide that implicit drop is wrong.

Unwinding panics are an exceptional case. It is important to handle them without breaking Rust's memory safety guarantees, and useful to allow types to customize their behavior when dropped because of unwinding. But silently and implicitly making every variable which is not consumed before the end of its scope do the same thing (you can check `thread::panicking`, but still) as when handling an unwinding panic is frustrating when the best-effort cleanup can you can do leaks memory, silently ignores errors, or sends a placeholder value. The current use of `drop` combines the exceptional case of unwinding with the extremely common case of ending a scope: we don't necessarily want to do the same thing for both, and don't need to use the API forced by the limitations of unwinding panic in the case of non-exceptional control flow.

All of the above being said, breaking existing Rust code should be avoided. A warning (easy to turn off, easy to turn into a hard error, on a crate by crate or even finer grained level) achieves the purpose of alerting the user when they have accidentally dropped a type which very much does not want to be. This warning should also be minimally invasive, and require small to no changes in existing Rust code.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## The `#[must_cleanup]` attribute

Most types take care of their own cleanup when they go out of scope. A `Vec` deallocates its storage, a `MutexGuard` unlocks the mutex, a `Rc` or `Arc` decrements the reference count, and so on, along with types like integers that don't need any cleanup. These types can be dropped whenever you want just by letting their scope expire, or calling `mem::drop`.

However, some types would much rather have an explicit step before they go away. This could take the form of a file that might fail to close and wants to tell you about it, or a request that needs your signature as the last step before completion (and you don't want to just forget about it accidentally), among other possibilities. Types that want an explicit final step are marked with the `#[must_cleanup]` attribute.

If a value of a type marked with the `#[must_cleanup]` attribute goes out of scope without being consumed (is dropped), a warning will occur. The warning will go away if you consume the value or return it. In the rare case that you actually want to leak an object, `mem::forget` works for all values.

## An example of a `#[must_cleanup]` type

As an example of an API that might choose to make use of `#[must_cleanup]`, consider [`BufWriter`](https://doc.rust-lang.org/std/io/struct.BufWriter.html). Quoting from the documentation, "It is critical to call flush before `BufWriter<W>` is dropped. Though dropping will attempt to flush the contents of the buffer, any errors that happen in the process of dropping will be ignored. Calling flush ensures that the buffer is empty and thus dropping will not even attempt file operations." This is an example where the implicit drop leads to a footgun: rather than dropping, you want to call a method that first tries to flush, closes if successful, and if there was an error returns back to the caller to handle the error.

The relevant parts of the interface might look like:
```rust

#[must_cleanup]
pub struct BufWriter<W: Write>
{
    inner: W, // the inner writer
    buf: Vec<u8>, // the buffer
}

impl<W: Write> Write for BufWriter<W> {
    fn write(&mut self, buf: &mut [u8]) -> Result<usize>;
    fn flush(&mut self) -> Result<()>;
}

// For this example, we will assume that dropping the inner writer is how to close it. There are other reasonable choices here.
impl<W: Write> Write for BufWriter<W>
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

Then to use it, you would need to call `close` (or `close_check_error` or `close_without_flush`) to close the writer: if the writer goes out of scope a warning will be issued. This makes it easy to do the correct thing (handle the errors), and makes sure that ignoring errors on close is intentional, not an accident.
```rust
1:  fn main() -> Result<(), IoError> {
2:      let mut writer = BufWriter::create(...)?;
3:      if let Error(e) = writer.write("Hello") {
4:          writer.close_check_error()?;
            return Error(e);
        }
5:      writer.close_check_error()?
        // Omitting the `close_check_error` call causes a compiler warning:
        // value writer should not be implicitly dropped; type BufWriter<...> is marked #[must_cleanup].
        // value created on line 2, scope ends on line 6.
        // last use does not consume value.
6:  }
```

If there is a `panic!` before the writer is closed, `Drop::drop` will be called as normal: in this case you don't have an opportunity to catch any errors reported by file close because you are already panicking.

Unsafe code can make no new assumptions.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

There is a new attribute `#[must_cleanup]`, which can only be attached to structs and traits.

For each function, we compute the list of types which may be dropped (types may depend on generic parameters). Some types we can determine `#[must_cleanup]` or not immediately: types which only depend on lifetime parameters are always in this class. For each type which may be dropped, if we can determine that it is `#[must_cleanup]`, emit a warning explaining why the type may be dropped. If we can determine that it is not `#[must_cleanup]`, we do not need to do anything. For types we cannot determine `#[must_cleanup]` (which always depend on non-lifetime generic parameters), add them to a list associated with the function. This list gets propagated to callers, where we can substitute actual generic arguments for the generic parameters.

The list of generic types which may be dropped for each function should be propagated to crates that depend on this one. Furthermore, for each function we can track if any `#[must_cleanup]` types were dropped, regardless of whether or not the crate author silenced the warning. Then when a function with silenced `#[must_cleanup]` warnings is used, we can emit a warning that this function may drop `#[must_cleanup]` values. This provides the guarantee that if there are no `#[must_cleanup]` warnings, then types marked with `#[must_cleanup]` will never be dropped except while unwinding.

## Partial moves

Partial moves and destructuring are valid ways to clean up a struct marked with `#[must_cleanup]`.
Internally, crates are expected to use these along with `mem::forget` to implement their public cleanup functions.

Because destructuring is possible if all fields are public and partial moves are possible if any field is public, adding `#[must_cleanup]` to a struct with zero fields or any crate-public fields should itself produce a warning that users of the crate can clean up the type without going through the provided cleanup functions.

## Trait objects

For traits, `#[must_cleanup]` means that `dyn Trait + ...` is `#[must_cleanup]`. For traits which are not object-safe, `#[must_cleanup]` is meaningless, and should warn.

When a value is coerced into `dyn Trait`, if `Trait` is marked with `#[must_cleanup]`, treat the coercion as a drop of the initial type of the value.

# Drawbacks
[drawbacks]: #drawbacks

* Rust is expected to look at the implementation of functions to determine which types they may drop, and this information gets exported as part of the function interface.
* Whether a trait object is `#[must_cleanup]` or not makes more sense as a modifier such as `dyn Trait + Destruct` rather than putting it on `Trait`, because different uses of trait objects need to drop or not.
* These are not linear types. This does not eliminate the use of `mem::forget`, so APIs like thread join guards are still broken: unsafe code cannot rely on destructors of any kind to be run for safety.

# Rationale and alternatives

The blog post that inspired this RFC: http://aidancully.blogspot.com/2021/12/less-painful-linear-types.html

Miscelaneous, non-exhaustive collection of similar prior proposals:
* http://aidancully.blogspot.com/2021/12/less-painful-linear-types.html
* https://users.rust-lang.org/t/private-drop-or-rust-could-be-better-at-raii-with-a-rather-small-change/12322
* https://github.com/rust-lang/rfcs/issues/814
* https://github.com/rust-lang/rfcs/issues/523 (this feature request would seem to be resolved by this proposal)
* https://github.com/rust-lang/rfcs/issues/2642 (an approach with a variation on `#[must_use]`)
* https://internals.rust-lang.org/t/pre-pre-rfc-nodrop-marker-trait/15682

## Using a trait

The trait `core::marker::Destruct` does a similar job tracking `const` drop types. General feeling was that adding this lint to the type system was too invasive for not enough benefit.

The computation of which generic types may be dropped by a generic function, and the propagation of that information, is extremely similar to the trait system.

## Why not an error?

Making this an error *requires* crates to adapt, and as such is a breaking change. By making this a warning, we do not change the compiler behavior except for diagnostics. Feedback on previous proposals strongly indicated that a warning was preferrable.

## Unwinding

This is not a proposal for linear types, although hopefully this proposal brings some of the advantages people are looking for in linear types.

How to deal with unwinding is one of the issues that has complicated previous proposals for linear types in Rust. When unwinding is possible, at almost any point the stack could need to be cleaned up: what is to be done with linear types in this case? In postponed RFC https://github.com/rust-lang/rfcs/issues/814, a `Finalize` trait was proposed that behaves identically to `Drop` but is only called in the unwinding case. Here we propose simply reusing the `Drop` trait for custom cleanup in both the unwinding and scope drop cases: If you need different behavior you can check `thread::panicking`.

This is a bit of a "punt": we allow `#[must_cleanup]` types to be scooped up and disposed of by panic at any point, without any compiler-emitted warnings. This is much less difficult than trying to forbid contexts where panic is possible.

Types can individually decide whether they want to abort, to leak, or to attempt a best-effort cleanup in the exceptional case of unwinding while being confident that users will not accidentally default to this suboptimal behavior.

## Branded types

Branded types allow the function return type to require the production of a value from the specific input parameter, rather than any value of the same type. This lets you encode "You must use *this* item" rather than "You must use *a* item". I see this as solving a different problem. This proposal is about how to avoid accidentally calling drop ever, and is not tied to the lifetime of a single function.

## What are the consequences of not doing this?

If we do not adopt this proposal, types for which implicit drop is a footgun will remain reliant on runtime checking. The case of accidental async future cancellation by drop has been brought up. Another example of an API that would benefit from `#[must_cleanup]` is when you are expected to eventually complete every recieved request with a completion status (the example I am familiar with is Windows Driver Framework requests: https://docs.microsoft.com/en-us/windows-hardware/drivers/wdf/completing-i-o-requests).

The issue of disabling implicit drops seems to come up frequently enough to demonstrate some level of desire for this feature in the Rust community.

# Prior art
[prior-art]: #prior-art

Vale has `!DeriveStructDrop`, which looks like a similar feature. This blog post [Higher RAII](https://verdagon.dev/blog/higher-raii-7drl), has interesting examples of real-world applications of this feature.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## Destructuring a zero-field struct

If a struct has no fields, can it still be destructured? Or does the whole struct get dropped, which would trigger a warning? This affects which structs are private enough for `#[must_cleanup]` to make sense for.

# Future possibilities
[future-possibilities]: #future-possibilities

## Specify interface

Add a way to specify the interface of a generic function, perhaps an attribute `#[may_drop(Type1, Type2, ...)]` for functions to specify a superset of the list of generic types they expect the compiler to infer.

## Warning default behavior

This warning might start out as ignore-by-default/opt-in, before graduating to warn by default and then to error by default.
