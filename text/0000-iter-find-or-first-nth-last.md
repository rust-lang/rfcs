- Feature Name: iter_find_or_first_nth_last
- Start Date: 2020-11-21
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Provide methods to try to find and return the first item in an `Iterator` satisfying a predicate, or return the first, nth or last item if the predicate is not satisfied by any of the items in the `Iterator`.

# Motivation
[motivation]: #motivation

Some APIs can return different formats or profiles/configurations it supports and for the developer to choose from. Usually these have differing properties, where the developer can choose one according to their requirements or preferences. Examples for such an API can be found in Vulkan, like choosing a color-format where a listing obtained is via [`vkGetPhysicalDeviceSurfaceFormatsKHR`](https://www.khronos.org/registry/vulkan/specs/1.2-extensions/man/html/vkGetPhysicalDeviceSurfaceFormatsKHR.html) ([`Surface::get_physical_device_surface_formats`](https://docs.rs/ash/0.31.0/ash/extensions/khr/struct.Surface.html#method.get_physical_device_surface_formats) in ash, a wrapper library for Rust).

With such a list at hand, `Iterator::find` can be used to find a fitting option, and if none is found, any item should be selected instead. By providing additional methods in the `Iterator` trait, this task can be executed elegantly.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`Iterator::find` can be used to search for an item satisfying a predicate:
```rust
let a = [0, 1, 2, 3];

assert_eq!(a.iter().by_ref().copied().find(|e| *e == 2), Some(2)); // predicate is satisfied by item `2`
assert_eq!(a.iter().by_ref().copied().find(|e| *e == 5), None   ); // predicate is never satisfied, so `find` returns `None`
```

Returning a default item in case no item satisfies the predicate can be done by chaining a call to `Option::unwrap_or`:
```rust
assert_eq!(a.iter().by_ref().copied().find(|e| *e == 5).unwrap_or(42), 42);
```

In case the default item to use is part of the sequence itself, the methods `find_or_first`, `find_or_nth` and `find_or_last` can be used to return, respectively, the first, nth or last item of the sequence:
```rust
assert_eq!(a.iter().by_ref().copied().find_or_first(|e| *e == 5   ), Some(0)); // predicate is never satisfied, so `find_or_first` returns the first item (`0`)
assert_eq!(a.iter().by_ref().copied().find_or_nth(  |e| *e == 5, 1), Some(1)); // predicate is never satisfied, so `find_or_nth` returns the 2nd item (`1`) (count begins from `0`)
assert_eq!(a.iter().by_ref().copied().find_or_last( |e| *e == 5   ), Some(3)); // predicate is never satisfied, so `find_or_last` returns the last item (`3`)
```

If the Iterator is empty, `find`, `find_or_first`, `find_or_nth` and `find_or_last` all return `None`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

A possible implementation of the proposed methods is available in [z33ky/iter-find-fnl](https://github.com/z33ky/iter-find-fnl).  
The methods are pretty straight forward.

# Drawbacks
[drawbacks]: #drawbacks

libstd grows.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The proposed additions are generic with regard to the specific use case, but actual use cases may be few.
The functionality can also be integrated in the application code, or in a separate crate maintained by the community.

# Prior art
[prior-art]: #prior-art

The author is not aware of prior art regarding the specific functionality being requested for Rust or any other programming language.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

For the mentioned use case the sequences that are processed may either have a specific order of preferences, with the most preferred one either first or last, or no communicated preference in the items.  
In the former case (preference present), one would normally either want the first or last items as default, and in the latter it wouldn't matter. Either way, `find_or_nth` seems to not solve a concrete use case.

It is added for some semblance of "symmetry" with other item-retrieval methods already available on `Iterator`. Likewise, `find_or_min` and `find_or_max` could be added as well.  
Maybe none of proposed methods but `find_or_first` and `find_or_last` are really useful to have though and the rest should not be added to libstd.

# Future possibilities
[future-possibilities]: #future-possibilities

None the author can think of. This feature seems pretty self-contained.
