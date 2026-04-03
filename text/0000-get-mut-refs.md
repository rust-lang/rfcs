- Feature Name: get-mut-many
- Start Date: 2021-03-11
- RFC PR: TBD
- Rust Issue: TBD

# Definitions:

- "child place(s)": similar in concept to a struct field but opaque to the borrow checker, examples being an array index or a hash map's entry.
- "dependant trait": a trait which abstracts over some requirement of another trait

# Summary
[summary]: #summary

Add a new trait and dependant unsafe trait which together abstract over the retrieval of multiple `&mut` to child places.
The normal trait would be added as part of the v1 prelude so that these functions are easy to find and use.

Example: (assume the trait function is called `get_mut_many`)
```rust
let place1: usize = ...;
let place2: usize = ...; // where place1 != place2

let mut x = [0, 1, 2, 3, 4];

if let Some([x1, x2]) = x.get_mut_many([place1, place2]) {
  swap(x1, x2);
}

dbg!{x};
```

# Motivation
[motivation]: #motivation

This is currently something that is not possible to do in safe code in rust because the borrow checker doesn't know how to deal with indexing or other forms of child places.
This mechanism would provide a standard way for types to implement the retrieval of multiple `&mut` to child places.
Because this exposes a safe API it can be used by everyone easily and thus reduces the difficulty of wanting to get multiple `&mut` into data structures at arbitrary locations.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Two new traits are added the `core` called `GetMutMany` and `GetMutManyRaw`.
It is not needed to import `GetMutMany` as it is part of the prelude.
There is a provided implementation of `GetMutMany` in terms of types that implement `GetMutManyRaw`.
It is then possible to implement either trait to get this functionality.
Unless there is a good reason to, it is recommended to implement `GetMutManyRaw`.
This is to reuse the `unsafe` code within the default implementation as much as possible.

Normally, when rust code uses the `IndexMut` or `BorrowMut` trait it borrows the whole owned value and returns a single `&mut`.
However, for some data structures, such as arrays and vectors, it is possible and sometimes desirable to get more than one `&mut` out.
This is because these data structures represent a managed collection of values which are logically (and from a memory safety point of view) disjoint from each other.

This trait is the common way for a data structure to expose this disjoint nature of its child places.
When the `get_mut_many` method is called it checks that all keys are valid and that none of the resulting `&mut` would violate the `&mut` contracts.
If either of those are false then `None` is returned.

## Examples:

### Individual Entries:

```rust
let mut x = [0, 1, 2, 3, 4];

if let Some([x1, x2]) = x.get_mut_many([0, 4]) {
  swap(x1, x2);
}

dbg!{x}; // [4, 1, 2, 3, 0]
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Traits:

```rust
/// This trait is unsafe to implement because it is used in upholding the
/// safety guarantees of `GetMutMany`
unsafe trait GetMutManyRaw<Key> {
    type Value;
    type RawEntry;

    /// should return true if both `&self` and `other` "point" to the entry or
    /// entries within a parent instance
    fn would_overlap(&self, e1: &Self::RawEntry, e2: &Self::RawEntry) -> bool;

    /// convert the item into a reference
    ///
    /// safety: the resulting reference must not break any of the `&mut`
    /// rules. Namely if you have a collections of `Self`'s and mean to
    /// call  `to_entry()` on all of them. Then all distinct pairs in that
    /// collection must return `false` from a theoretical call to
    /// `would_overlap`
    unsafe fn entry_to_ref<'a>(*mut self, entry: Self::RawEntry) -> &'a mut Self::Value;

    /// If `key` is not in `self` return None, otherwise return Some tuple
    /// of the pointer to the start of the collection of Values and an
    /// offset number of elements within. This is needed for ZSTs.
    fn get_raw_entry<'a>(&'a mut self, key: Key) -> Option<Self::RawEntry>;
}

trait GetMutMany<Key> {
    type Value;

    /// Gets `N` mutable references to `Value`'s within `Self` if all the
    /// `keys` are valid and wouldn't result in overlapping `&mut`'s.
    fn get_mut_many<'a, const N: usize>(
        &'a mut self,
        keys: [Key; N]
    ) -> Option<[&'a mut Self::Value; N]>;
}

impl <T, Key, Value> GetMutMany<Key> for T
where
    T: GetMutRefsRaw<Key>
    <T as GetMutRefsRaw<Key>>::Value = Value
{
    type Value = Value;

    fn get_mut_many<'a, const N: usize>(
        &'a mut self,
        keys: [Key; N]
    ) -> Option<[&'a mut Self::Value; N]> {
        let mut arr: [MaybeUninit<Self::RawEntry>; N] = unsafe { MaybeUninit::uninit().assume_init() };

        for (key, place) in array::IntoIter::new(keys).zip(arr.iter_mut()) {
            unsafe {
                place.as_mut_ptr().write(self.get_raw_entry(key)?);
            }
        }

        let arr: [Self::RawEntry; N] = unsafe { MaybeUninit::array_assume_init(arr) };

        for (i, x) in arr.iter().enumerate() {
            for (j, y) in arr.iter().enumerate() {
                if i != j && self.would_overlap(x, y) {
                    return None;
                }
            }
        }

        let mut res: [MaybeUninit<&'a mut Self::Value>; N] = unsafe { MaybeUninit::uninit().assume_init() };

        for (raw_entry, place) in array::IntoIter::new(arr).zip(res.iter_mut()) {
            unsafe {
                place.as_mut_ptr().write((self as *mut T).entry_to_ref(raw_entry));
            }
        }

        unsafe {
            Some(MaybeUninit::array_assume_init(res))
        }
    }
}
```

- The default implementation is present to make this feature easier for implementers to use.
- Because of the default implementation there is an `unsafe trait` to abstract over the type used for `RawEntry`.
- This allows for the easy adding of new impls.
- `RawEntry` is also required to correctly handle ZSTs, because two different ZSTs of the same type can exist at the same memory address.
So it is necessary to have some way of differentiating between them.

For example the following could be an implementation of this trait for arrays.
Even accounting for ZSTs:

```rust
struct ArrayRawEntry<T> {
    offset: usize,
}

unsafe impl<T, const LENGTH: usize> GetMutManyRaw for [T; LENGTH] {
    type Value = T;

    fn would_overlap(&self, e1: &Self::RawEntry, e2: &Self::RawEntry) -> bool {
        e1.offset == e2.offset
    }

    unsafe fn entry_to_ref<'a>(*mut self, entry: Self::RawEntry) -> &'a mut Self::Value {
        self.add(self.offset)
            .as_mut()
            .expect("NonNull plus usize should be NonNull")
    }

    fn get_raw_entry<'a>(&'a mut self, key: usize) -> Option<Self::RawEntry> {
        if key < self.len() {
            Some(Self::RawEntry {
                offset: key,
            })
        } else {
            None
        }
    }
}
```

This second examples will be for the `BTreeMap<K, V>` type.
Since this type works over user defined types (for its keys) this example shows the validity of the unsafe code in the default impl.

```rust
unsafe impl<K, V> GetMutManyRaw for Handle<NodeRef<marker::Owned, K, V, marker::LeafOrInternal>> {
    type Value = T;

    fn would_overlap(&self, e1: &Self::RawEntry, e2: &Self::RawEntry) -> bool {
        e1.eq(e2)
    }

    unsafe fn entry_to_ref<'a>(*mut self, entry: Self::RawEntry) -> &'a mut Self::Value {
        entry.into_val_mut()
    }

    fn get_raw_entry<'a>(&'a mut self, key: K) -> Option<Self::RawEntry> {
        let root_node = self.root.as_ref()?.borrow_mut();
        match root_node.search_tree(key) {
            Found(handle) => Some(handle),
            GoDown(_) => None,
        }
    }
}
```

# Drawbacks
[drawbacks]: #drawbacks

1. This adds another trait to the standard library which could be a crate.
2. This adds another trait to the prelude (for visibility reasons)
3. This could in theory be implemented in some form by the borrow checker in the future and having these traits might lead some to question if that is needed.
4. Doesn't support ranges currently because each range is a different type and the return type would have to be between individual items (`&mut T`) and multiple items (`&mut [T]`).

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- `get_mut_many` might panic instead of returning `None` like other indexing traits.
If this was the case then it might also be a good idea to have a `try_get_mut_many` which returns `None` instead of panicking.
- Add some form of multiple disjoint (and arbitrary) child place borrowing in the language instead of on top of it.
- Add more specific array patterns into the language (specifying indices).
Though this has been noted as not being something that is currently desired (non-`RangeFull` in patterns).
- Implement such functionality separately for all types with no trait backing.
This might be a good idea because the trait doesn't seem very helpful to begin with (it is rather complicated) and might not be useful to be able to abstract over types that can `get_mut_many`.
- A different name, more in line with the other indexing operational traits might be better, for instance something like `IndexMutMany`. 
However, since this RFC is not proposing augmenting the indexing syntax that might get a bit confusing.

# Prior art
[prior-art]: #prior-art

This was discussed on [rust internals](https://internals.rust-lang.org/t/add-as-mut-ref-for-slice-or-array/14199/31).
It started as a way to transform just arrays into an array of borrows but was soon found that while that already basically exists, it does not allow for arbitrary "picking" because of the limitations of array pattern.
However, that doesn't really help with other types that will probably never get structural patterns against them (ie `HashMap`).

This was sort of inspired by the [deferred borrows paper](https://cfallin.org/pubs/ecoop2020_defborrow.pdf).
This is not in fact a solution at all to the problems set out in that paper, but arguably a stepping stone.
This is because that paper points to a possible future where types can define what sort of deferred borrows they support.
Whereas in this feature types can define what sort of multiple child place borrows they support.

HashBrown has recently added a similar sort of API ([currently unstable](https://github.com/rust-lang/hashbrown/pull/239)).
Its API is slightly different as each element is returned as `Result<&mut T, UnavailableMutError>`.
This means that each element has to be checked by the caller as well.
That is a possible alternative to the API but makes the simple general case (all or nothing) much more difficult to use.
Perhaps supporting both would be useful as it doesn't add too much complexity.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- The names of the traits
- The names of the functions
- Individual impls or a trait

# Future possibilities
[future-possibilities]: #future-possibilities

Either individual impls or a trait backing, it seems reasonable to add support for range based child place slice borrowing.
However, because this involves at least 6 distinct types:
- [`Range`](https://doc.rust-lang.org/std/ops/struct.Range.html)
- [`RangeFrom`](https://doc.rust-lang.org/std/ops/struct.RangeFrom.html)
- [`RangeInclusive`](https://doc.rust-lang.org/std/ops/struct.RangeInclusive.html)
- [`RangeTo`](https://doc.rust-lang.org/std/ops/struct.RangeTo.html)
- [`RangeToInclusive`](https://doc.rust-lang.org/std/ops/struct.RangeToInclusive.html)
- [`usize`](https://doc.rust-lang.org/std/primitive.usize.html)

It doesn't seem likely that this would be `trait` backed.
Instead would probably be macro implemented like the old array impls.
Until such time as Rust gains a variadic generics support (at the very least).
