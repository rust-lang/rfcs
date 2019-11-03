- Feature Name: `recycle_vec`
- Start Date: 2019-11-03
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add method `Vec::recycle` that allows safe reuse of the backing allocation of the `Vec`.

# Motivation
[motivation]: #motivation

While custom allocators allow for great customizability, they are a complicated feature;
the full story for them is still in flux and most of the APIs are unstable. 

However, there are many use cases where the user might want to avoid calling `free` and `malloc`
repeatedly, but a very simple way of reusing the same buffer would suffice.

A prominient use-case is doing zero-copy parsing from a stream:

1. Get a chunk of bytes: a `&[u8]` that has only the lifetime of a single loop iteration.
2. Deserialize the chunk and store the deserialized objects into a `Vec`.
To achieve zero-copy parsing, the `Vec` holds references to the chunk.
3. Do whatever processing one does with the `Vec`.
4. Loop ends. Because the chunk lifetime is going to end, the `Vec` containing references to the chunk
must be de-allocated.

Note that having a permanently allocated `Vec` outside the loop doesn't work,
because the it isn't allowed to outlive the chunk. But re-allocating and freeing each iteration
isn't desirable either.

This RFC presents a simple API for `Vec` that reconciles this situation and allows safe reuse of the
backing allocation of `Vec`.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Sometimes you find yourself doing performance-sensitive processing where you want to avoid creating
new `Vec`s in a loop, but instead reuse a single `Vec` over and over again. If the values you are
storing in the `Vec` are `'static`, this is easy to achieve:

```
    let mut objects: Vec<Object> = Vec::new();

    while let Some(byte_chunk) = stream.next() {                      // `byte_chunk` lifetime starts

        deserialize(byte_chunk, &mut objects)?;
        process(&objects)?;
        objects.truncate(0);

    }                                                                 // `byte_chunk` lifetime ends
```

However, in these kinds of performance-sensitive contexts, it's not uncommon to do *zero-copy parsing*;
that is, reusing parts of the original byte buffer as-is, to save the cost of copying the data over
(and possibly allocating storage for the data) from the original buffer. This means that the `Object`s
parsed from `byte_chunk` have references to it:

```
    let mut objects: Vec<Object<'_>> = Vec::new();                    // `objects` lifetime starts

    while let Some(byte_chunk) = stream.next() {                      // `byte_chunk` lifetime starts

        // Zero-copy parsing; Objects has references to `byte_chunk`
        deserialize(byte_chunk, &mut objects)?;
        process(&objects)?;
        objects.truncate(0);

    }                                                                 // `byte_chunk` lifetime ends
                                                           // `objects` is still alive after the loop
```

This proves to be a problem:

1. `byte_chunk` must outlive the references to it for the references to stay valid.
2. The references are contained in `Object`s, which means `byte_chunk` must outlive the `Object`s.
3. The `Object`s in turn are contained in the `objects` `Vec`, which means `byte_chunk` must outlive `objects`.
4. However, `byte_chunk` doesn't do that; instead, `objects` outlives it, since we want to reuse
the allocation.

This leads to a lifetime conflict.

Note that since we `truncate` `objects` in the end of the each loop, it doesn't *actually* contain any
`Object`s that would outlive `byte_chunk`! However, statements like "this `Vec` is empty, therefore it's contents
oughtn't cause any lifetime conflicts" is not a statement that the type system understands or keeps track of.
The borrow checker just sees that the type of `objects` is `Vec<Object<'a>>` where `'a` must be outlived
by the lifetime of `byte_chunk`.

However, `Vec` has an API that allows us to fix the situation. Calling the `recycle` method allows us to
decouple the type – including the lifetime – of `objects` during each loop from the original type:

```
                                                // The lifetime here can be anything, including 'static
    let mut objects: Vec<Object<'static>> = Vec::new();

    while let Some(byte_chunk) = stream.next() {                      // `byte_chunk` lifetime starts

         let mut objects_temp: Vec<Object<'_>> = objects.recycle();   // `objects_temp` lifetime starts
 
        // Zero-copy parsing; Objects has references to `byte_chunk`
        deserialize(byte_chunk, &mut objects_temp)?;
        process(&objects_temp)?;
 
        objects = objects_temp.recycle();                             // `objects_temp` lifetime ends

    }                                                                 // `byte_chunk` lifetime ends
```

From the viewpoint of the borrow checker, `objects_temp` is a new, separate object that has nothing
to do with `objects`. This means that when we "return" it to `objects` at the end of the loop,
we have achieved our objective: `objects_temp` has a shorter lifetime than `byte_chunk`.

Note that `recycle` internally empties the `Vec` to preserve the soundness of the API;
not doing so would accidentally transmute unrelated types!
Additionally, `recycle` checks that the size and alignment of the source and target types match.
This is to ensure that the backing allocation has compatible memory layout.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The implementation of this RFC is very clear and concise; it's as follows:

```
impl Vec<T> {
    fn recycle<U>(mut self) -> Vec<U> {
        self.truncate(0);
        assert_eq!(core::mem::size_of::<T>(), core::mem::size_of::<U>());
        assert_eq!(core::mem::align_of::<T>(), core::mem::align_of::<U>());
        let capacity = self.capacity();
        let ptr = self.as_mut_ptr() as *mut U;
        core::mem::forget(self);
        unsafe { Vec::from_raw_parts(ptr, 0, capacity) }
    }
}
```

The implementation, however, includes unsafe code and thus deserves some scrutiny;
the reasoning why this API is able to provide a safe interface is as follows:

1. It truncates the `Vec` to zero length, dropping all the values.
This ensures that no values of arbitrary types are transmuted
accidentally.
2. It checks that the sizes and alignments of the source and target
types match. This ensures that the underlying block of memory backing
`Vec` is compatible layout-wise.
3. It creates a new `Vec` value using `from_raw_parts`, instead of
transmuting, an operation whose soundness would be questionable.

The major unresolved question whether the API is sound is its interaction with
possible future allocator features. Namely, while preserving the layout of the
backing memory, it might change its type. If there is going to be allocator APIs
that care about the type, they might not expect the pointer passed in upon
deallocation to be of different type from what was originally allocated.

This is also the main reason why this API deserves a places in the standard library:
Besides providing a useful tool for the situations described in Guide-level explanation,
its existence as a safe primitive is also a statement about the soundness
of the operation it performs.
The writer expects the Allocation WG to take a stance about whether
the proposed API is sound in presense of possible interactions with anticipated future
allocator APIs.

There is a complication around stabilizing this API: as defined like above,
it panics at runtime when called with a `Vec` that has an incompatible type.
However, when the const evaluation support gets improved, it is likely that
we will have a support for compile-time assertions that raise a compilation error.
As type sizes and alignments are knowable at compile-time,
it would be even better to – instead of panicking – detect errors early and show a helpful
error message. However, if stabilized as-is, changing it later to use compile-
time asserts will be a breaking change, as it might cause crates that used
to compile stop compiling.

# Drawbacks
[drawbacks]: #drawbacks

- It expands the standard library API surface with a method that could,
and in fact, is currently provided by a crate. (https://crates.io/crates/recycle_vec)
- It provides a safe primitive that can't be implemented in safe code alone.
Thus, is adds expressivity of safe code and in a sense decreases the guarantees
that other code could depend upon. (I.e. that `Vec`s are dropped as the same type
they were initialized as.) The author thinks that this is worth the tradeoff,
as the motivation for this API is clear, and on the other hand,
it's unclear whether there's any merit to be had in the said guarantee,
especially as it seems like something that unsafe code might be already doing
in the crates ecosystem.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Another design that was more focusedly trying to tackle the lifetime problem presented
in the Guide-level explanation was considered; if the API allowed to only reinterpret
the `Vec` type to have other lifetimes, it would be more restricted,
especially in the sense that lifetimes are guaranteed to be erased before code generation,
so the reinterpretation could never change the memory layouts of the types stored in the `Vec`.

However, this was deemed to be:
1. Hard or impossible to achieve using the current type system, as it doesn't allow expressing
directly subtyping relations between two generic types in where clauses. [1]
2. Unnecessary as the API doesn't actually transmute any values, and thus ought to be safe anyway.

Thus, it was deemed to be sufficient to limit verification of the memory layout to the checking
of size and alignment of the stored type.

The impact of not doing is this is that those who need to reuse allocations and run into
lifetime problems as described in the Guide-level explanation, either
1. use a crate such as https://crates.io/crates/recycle_vec
2. implement the feature themselves using `unsafe` code.

The author sees the 2. option as something that we, as members of the Rust community,
should strive to avoid, and instead provide and use APIs that wraps the interfaces safely.
The 1. option remains a valid altertanive to providing this API in the standard library.

However, as
1. there is generally some resistance against using small utility crates
2. they suffer from discoverability problem
3. they don't have a similar "mandate" over what's sound and what's not as the standard library is perceived to has,
the author believes that including this API in standard library is the best alternative.

[1] This was explored in this Reddit thread without success:
https://www.reddit.com/r/rust/comments/dmvsm0/a_question_about_where_clauses_and_subtyping_of/

# Prior art
[prior-art]: #prior-art

The same API is provided by the crate `recycle_vec` via an extension trait on `Vec`: https://crates.io/crates/recycle_vec

The author is unaware about prior art outside of the Rust language and its ecosystem;
it might be hard to find as the proposed API is positioned in the cutting point of high performance processing,
low-level control over memory allocation and separation between unsafe and safe code.
In this space, Rust remains quite alone.


# Unresolved questions
[unresolved-questions]: #unresolved-questions

- The interaction between (typed?) allocators and a `Vec` having a different type on drop from what it had on allocation.
  - If the Allocation WG expects that the future allocator APIs have no interactions with higher-level types, this makes the discussion straightforward.
  - The standard library already having methods like `String::into_bytes` and `String::into_boxed_str` essentially resolves this question:
  reinterpreting the type, even in "owned form" is allowed if the invariants of the types allow it. It would still nice to have a confirmation about whether this author's interpretation is correct.
- The method name can be bikeshedded.
- There are other collections that might benefit from similar API.
Should we add this API to other, or possibly all collections in the standard library?
- The backwards-compatibilty hazard before having compile-time asserts must be reconciled somehow
or the stabilization must be delayed until compile-time asserts are available.
- Anything else?

# Future possibilities
[future-possibilities]: #future-possibilities

In the case we add this API to `Vec` but don't end up adding it to other collections,
those collections still remain as plausible targets for extending them with a similar
API in future.
