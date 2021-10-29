- Feature Name: `take-on-bool`
- Start Date: 2021-10-27
- RFC PR: [rust-lang/rfcs#3189](https://github.com/rust-lang/rfcs/pull/3189)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

This RFC proposes adding a method to `bool` called `take`, which will save the value of the bool to a variable, set the bool to false, and then return that variable.

# Motivation

In many applications, deferred execution of routines is preferable, especially around long standing state. Using flags, or booleans which indicate a condition, is a good way to raise those routines. Taking that flag, while also resetting it, makes this pattern more elegant, without complexifying the `std` inordinately.

# Guide-level explanation

All `.take` does is return the value of a boolean while also setting that value internally to `false`. It is exactly similar to `Option::take`, except that, of course, it only returns `true/false` instead of some inner value. (In this sense, this `.take` is effectively the same as `Option::take().is_some()`). In any example where flags are commonly read while also being reset, this method is useful.

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
    pub fn calculuate_world_position(&mut self, parent: &SceneNode) {
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

Alternatives to this metohd entirely for users are, of course, just writing the code out themselves. Sometimes they may even be preferable for simplicity.

Users can also use `Option::<()>` instead of `bool` to get this functionality now, for free. Additionally, they could simply wrap `bool` and Deref into it, with the added `.take`.

This functionality could instead be provided by a crate (e.g. boolinator), but this functionality can be commonly desired and is in keeping with `Option::take`. A crate, however, could provide even more `bool` methods, like `toggle`, which may be useful to some users.

# Prior art

None, as far as I know.

# Unresolved questions

None

# Future possibilities

We could later add `toggle`.
