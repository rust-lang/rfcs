- Feature Name: any_type_self
- Start Date: 2016-07-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allow self parameter be any type.

# Motivation
[motivation]: #motivation

It allows to avoid wrapper structs.

# Detailed design
[design]: #detailed-design

Currently self parameter type is allowed to be only `Self`, `&Self`, `&mut Self`, `Box<Self>`.

Solution: Allow `self` be any type that depends on `Self`.

```rust
struct Sound {
}

impl Sound {
    fn play(&self) {
        // Implementation details
    }

    fn play_with_continuation<F>(&self, continuation: F)
        where F: FnOnce()
    {
        // Implementation details
        continuation();
    }
}

struct Game {
    first_sound: Sound,
    second_sound: Sound,
}

impl Game {
    fn play_sounds(self: &Rc<UnsafeCell<Self>>) {
        let sound = {
            let self = unsafe {
                &*self.get()
            };
            &self.first_sound
        };
        sound.play(move || {
            let sound = {
                let self = unsafe {
                    &*self.get()
                };
                &self.second_sound
            };
            sound.play()
        });  
    }
}
```

# Drawbacks
[drawbacks]: #drawbacks

No drawbacks.

# Alternatives
[alternatives]: #alternatives

Without it wrapper structs are required.

# Unresolved questions
[unresolved]: #unresolved-questions

No unresolved questions.
