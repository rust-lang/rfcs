- Feature Name: `freezable_mutex_guards`
- Start Date: 2025-10-15
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This feature should add an associated method `freeze_guard` to the `MutexGuard`. It should unlock the Mutex for a fixed
time to allow other threads to access it and lock it again after the scope. This function should take a mutable
reference instead of the owned type.

Side Note: If I mention the `MutexGuard` here I mean the following Lock Guards:
 - `MutexGuard`
 - `RwLockReadGuard`
 - `RwLockWriteGuard`
 - (`ReentrantLockGuard`) more on this one later


# Motivation
[motivation]: #motivation

Rusts Locks are wonderful, most multithreaded code would be impossible to write safe without them. But handling locks
over more than one scope is just a nightmare. This is because you always require the ownership of the guard if you want
to release the lock. This often leads to call chains where every method requires the owned Guard and the Mutex reference
as parameter and the owned Guard as return type. This produces unreadable code and leads to a lot of useless parameter
passing. The second approach if passing the Mutex is not wanted is to lock and unlock it in every scope needed. With
this style the code is much more readable, but it takes a lot of unnecessary locks and unlocks of the Mutex. This
approach is less performant.

The concept is to add a method with three parameters for the guard. First, obviously the mutable reference to the guard.
The second parameter is a closure containing code that should be run while the Mutex is unlocked. This by itself would
already work, but it might cause problems if another thread panics while holding the data. Therefore, the third
parameter is a "heal" closure. It takes a `PoisonError` with the guard as parameter to allow the thread to fix the inner
data.

This would allow always just using the mutable reference instead of the owned object. Which leads to better readablity
and maintainability while still being safe.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Unlocks the mutex temporary and lock it after the calling `func` again. If the Mutex is poisoned upon trying to lock it 
again, the `heal` function will be called to fix the inner data. After this, the poison will be cleared and the mutex 
guard is valid again.

### Example

```rust
use std::thread;
use std::sync::Mutex;
use std::sync::Condvar;
use std::time::Duration;

pub fn request_data(guard: &mut MutexGuard<usize>, condvar: &Condvar) {
    guard.freeze_guard(
        || { 
            &condvar.notify_one();
            thread::sleep(Duration::from_millis(100));
        },
        |poison| { panic!() }
    );
}

pub fn main() {
    let mutex = Mutex::new(0usize);
    let condvar = Condvar::new();

    thread::scope(|s| {
        let outer_guard = mutex.lock().unwrap();

        s.spawn(|| {
            let inner_guard = mutex.lock().unwrap();
            
            let inner_guard_ref = &mut inner_guard;
            assert_eq!(**inner_guard_ref, 0);
            request_data(inner_guard_ref, &condvar);
            assert_eq!(**inner_guard_ref, 123);

        });

        let outer_guard = &condvar.wait(outer_guard).unwrap();
        **outer_guard = 123;
        drop(outer_guard)
    })
}
```

### Panics:

If the thread panics while running the `func` closure the mutex will remain clear because it is not locked by the
current thread (excluding the case that the thread is acquiring the lock in the `func`). If the thread panics in the
`heal` function the thread mutex remains poisoned.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

```rust
impl<'a, T: ?Sized> MutexGuard<'a, T> {
    pub fn freeze_guard<F, H>(orig: &mut Self, func: F, heal: H) -> () 
    where
        F: FnOnce() -> (),
        H: FnOnce(PoisonError<T>) -> (),
    {
        // Unlocking the mutex
        unsafe {
            self.lock.poison.done(&self.poison);
            self.lock.inner.unlock();
        }
        
        // FIXME: Upon panicking in func the drop of self will be called which will lead to problems because the mutex
        //  is not actually locked by this guard. This might need a small change in the drop logic of the guard.
        
        // calling func
        func();
        
        // trying to acquire lock again
        let lock_result = self.lock.lock();
        let new_guard = match lock_result {
            Ok(guard) => guard,
            Err(poison_error) => {
                heal(poison_error);
                self.lock.clear_poison();
                // consider the case that the mutex is getting healed, another thread grabs the cleared mutex and poisons
                // it again. The heal function is currently an FnOnce and therefore it cannot be cleared again.
                self.lock.lock().unwrap()
            },
        };
        // replacing the old variable with the new
        std::mem::swap(self, &new_guard);
        // forgetting the value to prevent unlocking of the mutex again
        std::mem::forget(new_guard)
    }
}
```

This could be an implementation. It is currently missing two edge cases discussed in 
[Unresolved Questions](#unresolved-questions).

### Safety
During the call of `func` the mutable reference will be in the scope of `freeze_guard` and therefore it is not possible
to access it. The `heal` function will always restore the cleared mutex state and therefore guarantees a valid guard
after `freeze_guard`. If heal panics the mutex guard will remain poisoned as before.


# Drawbacks
[drawbacks]: #drawbacks

The `ReentrantLockGuard` is not a useful implementation. The function would need to take all mutable references to every
existing guard to guarantee a safe unlock process of the Lock. There might be a way to safely implement them with only
one guard reference. This might be useful to discuss.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

There are currently three ways to achieve the same behavior:
1. Passing the owned Guard across the call stack. This results in bad readability and maintainability.
2. Locking and unlocking the Mutex at every call. If the mutex is heavily used, this costs a lot of unnecessary performance.
3. Juggling with alot of unsafe code to somehow access the inner lock and poison to manually achieve the behavior of this
function.

# Prior art
[prior-art]: #prior-art

Currently open. I haven't found a similar concept in another language yet.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

### Panic in `func`
The current implementation proposal would cause the mutex to be poisoned and unlocked upon panicking in the `func` would
lead to an unlock and poison of the mutex. This could be fixed by implementing a frozen state for the guard telling the
drop implementation whenever it should actually run the drop logic of the guard.

### Multiple heals
The current implementation proposal might cause a panic in this scenario:
 - `func` gets called
 - during that time another thread locks the mutex and panics
 - `heal` gets executed
 - a third thread grabs the cleared mutex again and panics
 - the `heal` function is already executed due to the FnOnce state it won't be able to heal the mutex again.

### Name of the feature
The feature name might be confusing in combination with the [Freeze Trait](https://doc.rust-lang.org/std/marker/trait.Freeze.html)
which is implemented by the `MutexGuard` too. I haven't come up with a better name yet.

### `ReentrantLockGuard`
The function would be useless with this implementation for the `ReentrantLockGuard` there might be better ways to 
achieve the wanted behavior for it.

# Future possibilities
[future-possibilities]: #future-possibilities

An extension could be a `try_freeze_guard` method, which essentially does the same as the normal mutex but checks if
other threads require access to the Mutex beforehand.