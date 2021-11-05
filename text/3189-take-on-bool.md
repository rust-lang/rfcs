- Feature Name: `take-on-bool`
- Start Date: 2021-10-27
- RFC PR: [rust-lang/rfcs#3189](https://github.com/rust-lang/rfcs/pull/3189)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

This RFC proposes adding a method to `bool` called `take`, which will save the value of the bool to a variable, set the bool to false, and then return that variable.

# Motivation

In many applications, deferred execution of routines is preferable, especially around long standing state. Using flags, or booleans which indicate a condition, is a good way to raise those routines. Taking that flag, while also resetting it, makes this pattern more elegant, without complexifying the `std` inordinately.

The current one liner to get this same effect is `mem::take`. Adding `mem` into a project often implies a more complex operation than `.take` on a `Copy` tends to be, since `mem::take` (and `mem::replace` in general) is most useful when the type isn't Copy like `bool` is, which allows users to trivially do operations which otherwise require some non-trivial unsafe code. Moreover, in general, the `mem` module has a collection of relatively low-level operations on memory patterns. This makes it a more intimidating toolbox to reach for than a simple method on `bool` could be.

There are two places where a "Take" pattern is used that is more about API usability, rather than memory handling, but where `mem::take` could be used: `Option` and `bool`. For the former, we already have the `.take` method (and in my Rust experience, anecdotally, is used often). We don't have anything for the latter. This PR's motivation is adding such a method for `bool`.

# Guide-level explanation

All .take() does is return the value of a boolean while also setting that value internally to false. It is just like `mem::take`, except it is called as a method instead of a free function. In places where booleans are commonly read and then reset, like dirty flags, this method is useful.

For example, imagine a common game structure:

```rs
/// A recursive data structure of positions, commonly used in games to allow a parent/child relationship between transforms.
pub struct SceneNode {
    /// a position relative to this Node's parent.
    pub local_position: [f32; 2],
    /// an authoritative position
    pub world_position: [f32; 2],

    pub dirty: bool,
}

impl SceneNode {
    /// We want a new local position!
    pub fn set_local_position(&mut self, new_pos: [f32; 2]) {
        self.local_position = new_pos;
        self.dirty = true;
    }

    /// we magically have the parent in this example.
    pub fn calculate_world_position(&mut self, parent: &SceneNode) {
        // we can take the flag and also unset it in one method call
        if self.dirty.take() {
            self.world_position = [
                self.local_position[0] + parent.local_position[0],
                self.local_position[1] + parent.local_position[1],
            ];
        }

        /*
        // without this RFC, our code would be slightly uglier like this:
        if self.dirty {
            self.dirty = false;
            self.world_position = [
                self.local_position[0] + parent.local_position[0],
                self.local_position[1] + parent.local_position[1],
            ];
        }
        */
    }
}
```

# Reference-level explanation

Implementation should be the following:

```rs
pub fn take(&mut self) -> bool {
    // save the old value to a variable
    let val = *self;
    // and reset ourselves to false. If we are already false,
    // then this doesn't matter.
    *self = false;

    val
}
```

# Drawbacks

Save the usual drawbacks of adding a new method to the standard library, there are no drawbacks.

# Rationale and alternatives

There are two other possible implementations of the method:

1. Conditionally branching:

   ```rs
   pub fn take(&mut self) -> bool {
       if *self {
           *self = false;
           true
       } else {
           false
       }
   }
   ```

2. Using `mem::replace` or `mem::take`:

   ```rs
   pub fn take(&mut self) -> bool {
       // or core::mem::take(self)
       core::mem::replace(self, false)
   }
   ```

In practice, the proposed implementation produces identical code using Godbolt to #2, and the proposed implementation and #2 seem to always produce better code than the #1 alternative above (specifically, they tend to elide jumps more easily). However, in more complex code, these all seem to resolve to more or less the same code, so it's a fairly bike-sheddable difference.

Alternatives to this method entirely for users are, of course, just writing the code out themselves. Sometimes that may even be preferable for simplicity.

Users can also use `Option::<()>` instead of `bool` to get this functionality now, for free. Additionally, they could simply wrap `bool` and Deref into it, with the added `.take`.

This functionality could instead be provided by a crate (e.g. boolinator), but this functionality can be commonly desired and is in keeping with `Option::take`. A crate, however, could provide even more `bool` methods, like `toggle`, which may be useful to some users.

# Prior art

Prior art: `AtomicBool::compare_exchange`, which is used for similar purposes. It is a more complex operation, because it allows specifying memory orderings (irrelevant here) and because it can set the boolean to either true or false, or act effectively as a read without modifying. But, it is closely related in that, for example, code being migrated towards or away from Sync support might replace one with the other.

# Unresolved questions

None

# Future possibilities

We could later add `toggle`.
