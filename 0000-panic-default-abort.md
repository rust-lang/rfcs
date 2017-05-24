- Start Date: 2014-12-12
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Currently, a spawned task that panics does not abort the process. A panic on a task is simply silently discarded. This is inappropriate behaviour, as the user's program may not be in a desired working state (a sort of zombie process). Instead, failing tasks should fail the entire process unless explicitly opted out of.

# Motivation

Defaulting to abort improves Rust's correctness, as programmers will have a harder time "forgetting" to handle panics from other tasks. Other systems, such as the Microsoft .NET platform, made the same mistake of silently ignoring background exceptions. For version 2, they made a breaking change to fix this behaviour, presumably because swallowing errors led to buggy programs.

Changing this also makes Rust's behaviour more uniform. Having a task abort only if it panics twice, but silently ignore if it only panics once does not seem very elegant.

Rust can rely on poisoning to abort related tasks. This allows, say, tasks sharing a mutex to all panic when acquiring the mutex after one task has panicked. This may eventually cause the entire program to crash. Relying on this behaviour may be somewhat non-deterministic, and also relies on every shared struct to support poisoning. This is complementary behaviour, and does not replace the need to abort if a task didn't opt into silent ignoring. 

# Detailed design

Consider this program that spawns a task to update some shared state. If the state is no longer being updated, that's a serious condition and the process is rather much like a zombie.

```rust
use std::sync as sync;
use std::sync::atomic as atomic;

fn main() { 
    let state = sync::Arc::new(atomic::AtomicInt::new(0i));

    let spawn_state = state.clone();
    spawn(proc() { 
        spawn_state.store(1, atomic::Ordering::Relaxed);
        panic!("State update failed.");
    });

    for _ in range(0u, 2) { 
        std::io::timer::sleep(std::time::Duration::seconds(1));
        println!("State is {}", state.load(atomic::Ordering::Relaxed));
    }
}
```

Instead, when the spawn'd proc panics, the entire process should crash. This would prevent the rest of the program from depending on now-incorrect state. 

Specifically, the default behaviour for "spawn" should be to abort on panic. If the programmer does not want this behaviour, they can use try_future. Or to avoid having an unwanted return value, consider adding a "try_spawn" method to std::task.

# Drawbacks

This is a breaking change for people depending on spawn's fire-and-forget model. Despite it being for their own good, some users may be relying on this behaviour, and may not with to use try_future as it is marked experimental.

# Alternatives

It is possible to attempt to rely on poisoning for similar behaviour. But some tasks may not access shared state. For instance, a user may spawn a task to send an email, and never check for errors. In that case, there's no shared state to be poisoned, and the process could continue, brokenly attempting to send mail.

Another alternative is to make this some sort of global flag, but that seems hackish.

# Unresolved questions

Decide the exact name for a spawn-with-silent-failure. try_spawn or spawn_try seem likely candidates. 
