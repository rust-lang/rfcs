# RFC: No (opsem) Magic Boxes

- Feature Name: `box_yesalias`
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#3712](https://github.com/rust-lang/rfcs/pull/3712)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Currently, the operational semantics of the type [`alloc::boxed::Box<T>`](https://doc.rust-lang.org/beta/alloc/boxed/struct.Box.html) is in dispute, but the compiler adds llvm `noalias` to it. To support it, the current operational semantics models have the type use a special form of the `Unique` (Stacked Borrows) or `Active` (Tree Borrows) tag, which has aliasing implications, validity implications, and also presents some unique complications in the model and in improvements to the type (e.g. Custom Allocators). We propose that, for the purposes of the runtime semantics of Rust, `Box` is treated as no more special than a user-defined smart pointer you can write today[^1]. In particular, it is given similar behaviour on a typed copy to a raw pointer.

[^1]: We maintain some trivial validity invariants (such as alignment and address space limits) that a user cannot define, but these invariants only depend upon the value of the `Box` itself, rather than on memory pointed to by the `Box`.

# Motivation
[motivation]: #motivation

The current behaviour of [`alloc::boxed::Box<T>`] can be suprising, both to unsafe code, and to people working on the language or the compiler. In many respects, `Box<T>` is treated incredibly specially by `rustc` and by Rust, leading to ICEs or unsoundness arising from reasonable changes, such as the introduction of per-container `Allocator`s.

In the past, the operational semantics team has considered many ad-hoc solutions to the problem, while maintaining special cases in the aliasing model (such as Weak Protectors) that only exist for `Box<T>`. 
For example, moving a `ManuallyDrop<Box<T>>` after calling `Drop` is immediate undefined behaviour (due to the `Box` no longer being dereferenceable) - <https://rust-lang.zulipchat.com/#narrow/stream/136281-t-opsem/topic/Moving.20.60ManuallyDrop.3CBox.3C_.3E.3E.60>, and the Active tag requirements for a `Box<T>` are unsound when combined with custom allocators <https://rust-lang.zulipchat.com/#narrow/stream/213817-t-lang/topic/Is.20Box.20a.20language.20type.20or.20a.20library.20type.3F>. This wastes procedural time reviewing the proposals, and complicates the language by introducing special case rules that would otherwise be unnecessary.

In the case of `ManuallyDrop<T>`, because `Box<T>` asserts aliasing validity on a typed copy, and is invalidated on drop, it introduces unique behaviour - `ManuallyDrop<Box<T>>` *cannot* be moved after calling `ManuallyDrop::drop` *even* to trusted code known not to access or double-drop the `Box`. No other type in the language or library has the same behaviour[^2], as primitive references do not have any behaviour on drop (let alone behaviour that includes invalidating themselves), and only `Box<T>`, references, and aggregates containing references are retagged on move. There are proposed solutions on the `ManuallyDrop<T>` type, such as treating specially by supressing retags on move, but this is a novel idea (as `ManuallyDrop<T>` asserts non-aliasing validity invariants on move), and it would interfere with retags of references without justification. The proposed complexity is only required because of `Box<T>`.

In the case of allocators[^3], without special handling of them in the language as well, the protectors assigned to `Box<T>` were violated by (almost) any non-trivial allocator that provides the storage itself (without delegating to something like `malloc` or `alloc::alloc::alloc`). This is because the allocators access the same memory that the `Box` stores to mark it as deallocated and available again. In an extreme example, the same memory could even be yielded back to another `Allocator::allocate` call. Solving this requires special casing `Allocator`s, which is a heavily unresolved discussion, only applying the special opsem behaviour to `Global`, which is opaque via the global allocator functions, or forgoing custom allocators for `Box` entirely (thus depriving anyone needing to use a custom allocator from the user-visible language features `Box` alone provides). With the exception of the former, which is desired for other optimization reasons though [heavily debated and not resolved](https://github.com/rust-lang/unsafe-code-guidelines/issues/442), these solutions are merely solving the symptom, not the problem.  

Any `unsafe` code that may want to temporarily maintain aliased `Box<T>`s for various reasons (such as low-level copy operations), or may want to use something like `ManuallyDrop<Box<T>>`, is put into an interesting position: While they can use a user-defined smart pointer, this requires both care on the part of the smart pointer implementor, but also affects the ergonomics and expressiveness of that code, as `Box<T>` has many special language features that surface at the syntax level, which cannot be replicated today by a user-defined type.

[^2]: It is possible to write a type that has the same issue in theory, involving a `&UnsafeCell<T>` field and manually holding an a pointer to an allocation. Holding any other type of reference, however, would cause immediate UB if a function parameter with that type was dropped, due to the strong protector, compared to a weak protector used for `Box<T>`, which allows deallocation, and no protector used for `&UnsafeCell<T>`. For the same reasons as `Box<T>` here, I would personally discourage any type from directly holding a reference that is invalidated in its `Drop`, and for the most part, this is protected against by the borrow checker, and requires unsafe lifetime extension to achieve.
[^3]: Since [rust#122018](https://github.com/rust-lang/rust/pull/122018), this is fixed as only `Box<T>` has the special opsem behaviour described here, which is the second solution.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

[`alloc::boxed::Box<T>`] is defined with a number of magic features that interact directly with the language. To limit the impact, both on users and on the language itself, we remove the language level requirement that `Box` be dereferenceable and unique (like applies to `&mut T`). 
This does not affect library level requirements of uniqueness however - it may remain undefined behaviour to access a `Box` that is being aliased (particularily where the operations produce multiple aliasing mutable references from different `Box`es) or to use `Box::from_raw` to construct multiple aliasing boxes. 

We also do not remove any other language level magic from `Box`, such as the ability to do both partial and complete moves from a `Box`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

For the remainder of this section, let `WellFormed<T>` designate a type for *exposition purposes only* defined as follows:
```rust
#[repr(transparent)]
struct WellFormed<T: ?Sized>(core::ptr::NonNull<T>);
```

(Note that we do not define this type in the public standard library interface, though an implementation of the standard library could define the type locally)

The following are not valid values of type `WellFormed<T>`, and a typed copy that would produce such a value is undefined behaviour:
* Any value that is invalid for a raw pointer to `T` (e.g. a value read from uninitialized memory, or a fat pointer with invalid metadata)
* A null pointer (even when `T` is a Zero-sized type)
* A pointer with an address that is not well-aligned for `T` (or in the case of a DST, the `align_of_val_raw` of the value), or
* A pointer with an address that offsetting that address (as though by `.wrapping_byte_offset`) by `size_of_val_raw` bytes would wrap arround the address space 

The [`alloc::boxed::Box<T>`] type shall be laid out as though a `repr(transparent)` struct containing a field of type `WellFormed<T>`. The behaviour of doing a typed copy as type [`alloc::boxed::Box<T>`] shall be the same as though a typed copy of the struct `#[repr(transparent)] struct Box<T>(WellFormed<T>);`. 

[`alloc::boxed::Box<T>`] shall have a niche of the all zeroes bit pattern, which is used for the `None` value of [`core::option::Option<Box<T>>`] (and similar types). Additional invalid values may be used as niches, but no guarantees are made about those niches. 

When the unstable feature [`allocator_api`] is in use (or after its stabilization), the type [`alloc::boxed::Box<T,A>`] (where `A` is not the type `Global`, `alloc::boxed::Box<T,Global>` is the same type as [`alloc::boxed::Box<T>`]) is laid out as a struct containing a field of type `WellFormed<T>`, and a field of type `A`, and a typed copy as [`alloc::boxed::Box<T,A>`] is the same as a typed copy of that struct. The order and offsets of these fields is not specified, even for an `A` of size 0 and alignment 1.

A value of type [`alloc::boxed::Box<T,A>`] is invalid if either field is invalid.

In addition to the above, [RFC 3336] is amended to remove the `MaybeDangling` behaviour of `ManuallyDrop` as it is no longer necessary. The `MaybeDangling` type itself is preserved as it can be useful in other contexts.

# Drawbacks
[drawbacks]: #drawbacks

This prohibits the compiler from directly optimizing usage of `Box<T>` via the llvm `noalias` attribute or other similar optimization attributes. 
This precludes optimizations made both today, and in the future (both in advancements made on the `rustc` compiler, including via llvm, and on other alternative compilers).

However, past performance benchmarks have shown little to no performance is obtained using current optimizations from attaching `noalias` to `Box` in either position. Many future performance gains that are precluded by this RFC can likely be restored by more granular emission of `noalias` and other optimization attributes for shared and mutable references 

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Alternative 1: Status Quo
    - While the easiest alternative is to do nothing and maintain the status quo, as mentioned this has suprisingly implications both for the operational semantics of Rust
- Alternative 2: Introduce a new type `AliasableBox<T>` which has the same interface as `Box<T>` but lacks the opsem implications that `Box<T>` has.
    - This also does not help remove the impact on the opsem model that the current `Box<T>` has, though provides an ergonomically equivalent option for `unsafe` code.
- Alternative 3: We maintain the behaviour only for the unparameterized `Box<T>` type using the `Global` allocator, and remove it for `Box<T,A>` (for any allocator other than `A`), allowing unsafe code to use `Box<T, CustomAllocator>`
    - Likewise, this maintains the opsem implications. 
    - Unsafe code would be required to rewrite code but would still be able to achieve the same ergonomics by making a custom allocator that calls the global allocator
    - As a future posibility, we could provide `alloc::alloc::GlobalAlias` (or another name), which is exactly this allocator.
- Alternative 4: We could go further than simply removing noalias behaviour, and limit `Box<T>` to containing a `NonNull<T>` field, rather than the `WellFormed<T>` type.
    - This would solve the motivation, but goes too far: It forcloses the compiler optimizing on additional `Box<T>` values that are known to be nonsense (such as unaligned pointers or pointers that would fall past the end of the address space when dereferenced). 
    - In contrast to the special aliasing behaviours of `Box<T>`, which in some ways are unique to `Box`, validity invariants are far better understood and commonly used. 
    - The `WellFormed<T>` type could even be used for other containers and smart pointers, such as `Rc`, `Arc`, or `Vec`.
- Alternative 5: We could simply remove `noalias` from `Box` without modifying the language rules
    - This alternative is, in my opinion, the worst option. It doesn't actually solve the problems outlined, and `noalias` on `Box` has no soundness issues as it is currently used under the current rules. 
    - This neither provides any permissions to `unsafe` code, nor alievates any opsem issues, and simply disregards any optimizations, even theoretical ones, produced by `noalias`.
- Alternative 6: Provide `Unique<T>` instead of `WellFormed<T>`, that provides the current behaviour as box
    - Like other "Leave `Box<T>` as it is" proposals, it doesn't support unsafe code or simplify the opsem, and only benefits code wanting the same nebulous and largely unused optimization capacity.


# Prior art
[prior-art]: #prior-art

I am not aware of any prior art for this change, as I am not aware of prior art (outside of the language) for the special behaviour of `Box<T>`. An equivalent type, C++'s `std::unique_ptr<T>` however, can serve as prior art for `Box<T>` not having the special behaviour, as it does not produce noalias (https://godbolt.org/z/hGexhTa9v).

`Vec<T>` and other collections in the standard library also serve as prior art in this manner, as `Box<T>` is the only standard library type to have special treatment in the manner. `Vec<T>` in particular is quite similar to `Box<[T]>`, but lacks any memory aliasing optimizations like `Box<T>` presently offers. 

The `WellFormed<T>` type, used in this document for *exposition* purposes, is proposed in [RFC 3204], though we do not propose any immediate changes to the layout algorithm implemented by rustc, nor that Rust guarantee the specified niches for `WellFormed<T>`, and only use it to express the full proposed validity invariant of `Box<T>` and `Box<T,A>`, which could be exploited by a Rust compiler for niche optimization. If #3204 is adopted, the implementation of `Box<T>` and `Box<T,A>` required here would be compatible with containing a field of the proposed type.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should we limit the scope of library UB on aliasing `Box`es to the effects that arise from the borrow model on derived mutable references (IE. its ok to `DerefMut` an aliasing `Box` provided that the other `Box`es aren't used throughout the lifetime) or allow library routines to continue to directly exploit uniqueness.
- Should the layout of allocator_api `Box<T,A>` be constrained to specifically containing the `WellFormed<T>` field and `A` field? Should we constrain the layout further (like requiring `repr(C)`, or no other layout salient fields?)
- Should `Box::from_raw` and other functions be able to directly exploit uniqueness, or only exploit it to guarantee that the resulting value satisfies the safety invariant (e.g. could an implementation of `Box::from_raw` temporarily produce a mutable reference to the pointee).

# Future possibilities
[future-possibilities]: #future-possibilities

In the future, `Box<T>` should be completely demagicked, by the introduction of new language features that allow user-defined types to provide the same interface that `Box<T>` does today (such as `DerefMove`/`DerefPlace`, `#[may_dangle]` destructors, and Deref Patterns). 

We may also wish to expose the `WellFormed<T>` type named above, to allow user-defined types to convey a validity invariant of "An Allocation of type `T` could exist here". As mentioned above, [RFC 3204] proposes such a type along with a number of other changes.

Finally, should optimizations begin to be implemented in Rust compilers that could have made specific use of the special behaviour of `Box`, we can introduce a new type, possibly called `UniqueBox<T>`, which reacquires the special behaviours removed from `Box<T>` today. This would be predicate on more developed opsem rules, and a better understanding on how various other language and library features (like `allocator_api`) would interact with those behaviours.

[`alloc::boxed::Box<T>`]: https://doc.rust-lang.org/nightly/alloc/boxed/struct.Box.html
[`alloc::boxed::Box<T,A>`]: https://doc.rust-lang.org/nightly/alloc/boxed/struct.Box.html
[`core::option::Option<T>`]: https://doc.rust-lang.org/nightly/core/option/enum.Option.html
[`core::ptr::NonNull<T>`]: https://doc.rust-lang.org/nightly/core/ptr/struct.NonNull.html
[RFC 3204]: https://github.com/rust-lang/rfcs/pull/3204
[RFC 3336]: https://github.com/rust-lang/rfcs/blob/master/text/3336-maybe-dangling.md