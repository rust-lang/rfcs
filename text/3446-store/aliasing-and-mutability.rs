/// The following Rust code illustrates an aliasing issue faced by inline stores in the absence of interior mutability.
///
/// There is a debate as to whether stores (and allocators) should be implemented with interior mutability, or not. The
/// current `Allocator` API uses `&self` for `allocate` and al. and thus requires a form of interior mutability. This
/// is sensible for a global allocator, but perhaps less so for a stack-allocator, or an inline store, for which
/// concurrent accesses are rare.
///
/// However, Rust exposes aliasing issues even in the absence of concurrency, and one such issue is here: while it is
/// reasonable to expect that most collections will not allow mutation of the collection with outstanding borrows, it
/// is also _routine_ to obtain multiple mutable borrows (to distinct elements) within a collection.
///
/// This is not a problem for an `Allocator`, but it is a problem for a `Store`, and most specifically for the
/// `Store::resolve` method. In the absence of interior mutability, a `resolve_mut` method is required to soundly derive
/// a mutable reference from a mutable reference to the store. However, this clashes with the aliasing model typically
/// associated with mutable references, a `&mut Store` reference is formed _while_ `&mut T` references exist pointing
/// within the memory area covered by `&mut Store`.
///
/// The Stacked Borrows model therefore rejects the following program, as it eagerly invalidates outstanding borrows to
/// the memory area when forming a `&mut Store`, whereas the Tree Borrows model accepts it.
///
/// It is unclear to me what semantics the `noalias` LLVM attribute would entail, if closer to the Stacked Borrows model
/// then it may be problematic.

use std::{iter::{Iterator, IntoIterator}, mem::MaybeUninit};

struct DuoStore<T>([MaybeUninit<T>; 2]);

struct Duo<T> {
    length: usize,
    store: DuoStore<T>,
}

impl<T> Duo<T> {
    const fn new() -> Self {
        let length = 0;
        let store = DuoStore([MaybeUninit::uninit(), MaybeUninit::uninit()]);

        Self { length, store }
    }

    fn push(&mut self, value: T) {
        let Some(slot) = self.store.0.get_mut(self.length) else {
            return
        };

        slot.write(value);

        self.length += 1;
    }

    fn iter_mut(&mut self) -> DuoIterMut<'_, T> {
        DuoIterMut { index: 0, duo: self }
    }
}

impl<'a, T> IntoIterator for &'a mut Duo<T> {
    type Item = &'a mut T;
    type IntoIter = DuoIterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

struct DuoIterMut<'a, T> {
    index: usize,
    duo: &'a mut Duo<T>,
}

impl<'a, T> Iterator for DuoIterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.duo.length {
            return None;
        }

        let index = self.index;
        self.index += 1;

        let pointer = self.duo.store.0[index].as_mut_ptr();

        let r = unsafe { &mut *pointer };

        Some(r)
    }
}

fn main() {
    let mut duo = Duo::new();

    duo.push(String::from("0"));
    duo.push(String::from("1"));

    //  One mutable reference at a time is fine.
    for s in &mut duo {
        println!("{s}");
    }

    //  Multiple mutable references (to different elements) at a time falls foul of Stacked Borrows, and may not play
    //  well with the `noalias` LLVM attribute.
    let v: Vec<_> = duo.iter_mut().collect();

    for s in v {
        println!("{s}");
    }
}

/*

MIRI Stacked Borrows error reproduced below:

error: Undefined Behavior: trying to retag from <5099> for Unique permission at alloc875[0x0], but that tag does not exist in the borrow stack for this location
   --> /.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/into_iter.rs:202:27
    |
202 |             Some(unsafe { ptr::read(old) })
    |                           ^^^^^^^^^^^^^^
    |                           |
    |                           trying to retag from <5099> for Unique permission at alloc875[0x0], but that tag does not exist in the borrow stack for this location
    |                           this error occurs as part of retag at alloc875[0x0..0x18]
    |
    = help: this indicates a potential bug in the program: it performed an invalid operation, but the Stacked Borrows rules it violated are still experimental
    = help: see https://github.com/rust-lang/unsafe-code-guidelines/blob/master/wip/stacked-borrows.md for further information

help: <5099> was created by a Unique retag at offsets [0x0..0x18]
   --> src/main.rs:102:21
    |
102 |     let v: Vec<_> = duo.iter_mut().collect();
    |                     ^^^^^^^^^^^^^^^^^^^^^^^^

help: <5099> was later invalidated at offsets [0x0..0x38] by a Unique retag (of a reference/box inside this compound value)
   --> src/main.rs:102:21
    |
102 |     let v: Vec<_> = duo.iter_mut().collect();
    |                     ^^^^^^^^^^^^^^^^^^^^^^^^
    = note: BACKTRACE (of the first span):
    = note: inside `<std::vec::IntoIter<&mut std::string::String> as std::iter::Iterator>::next` at /.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/into_iter.rs:202:27: 202:41

    note: inside `main`
   --> src/main.rs:104:14
    |
104 |     for s in v {
    |              ^


*/
