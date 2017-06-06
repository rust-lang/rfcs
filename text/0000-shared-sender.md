- Feature Name: shared_sender
- Start Date: 2015-09-28
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add a `SharedSender` to `std::sync::mpsc` that implements `Sync`.

# Motivation

The current `std::sync::mpsc::Sender` does not implement `Sync`. This is
because the `Sender` starts as a `spsc` queue, and needs to upgrade to `mpsc` using `Clone::clone`. Accidentally putting the `Sender` into an `Arc` and cloning that would skip the upgrade, and make the `Sender` unsafe. So far, the design is just fine.

However, at times, there is real desire for the `Sender` to implement `Sync`. If passing the `Sender` into something requiring `Sync`, the only options are both sub-optimal: a) put the `Sender` in a `Mutex`, or b) look on crates.io for another mpsc solution.

Both "solutions" are not even truly required, since inside the `mpsc` module, there exists all the code necessary for a thread-safe mpsc, in the `Flavor::Shared` variant that is used when you clone a `Sender`.

# Detailed design

Add the following struct to the `mpsc` module:

```rust
pub struct SharedSender<T> {
    inner: Arc<UnsafeCell<shared::Packet<T>>>
}

unsafe impl<T: Send> Send for SharedSender<T> {}
unsafe impl<T: Send> Sync for SharedSender<T> {}

impl<T: Send> SharedSender {
    fn new(inner: Arc<UnsafeCell<shared::Packet<T>>>) -> SharedSender<T> {
        SharedSender {
            inner: inner
        }
    }
    pub fn send(&self, t: T) -> Result<(), SendError<T>> {
        unsafe { &mut *self.inner.get() }.send(t)    
    }
}

impl<T: Send> Clone for SharedSender<T> {
    fn clone(&self) -> SharedSender<T> {
        let a = self.inner.clone();
        unsafe { &mut *a }.clone_chan();
        SharedSender::new(a)
    }
}

impl<T> Drop for SharedSender<T> {
    fn drop(&mut self) {
        unsafe { &mut *self.inner.get() }.drop_chan();    
    }
}

```

In order to create a `SharedSender`, the following method is proposed:

```rust
pub fn shared_channel<T: Send>() -> (SharedSender<T>, Receiver<T>) {
    let a = Arc::new(UnsafeCell::new(shared::Packet::new()));
    (SharedSender::new(a.clone())), Receiver::new(Flavor::Shared(a)))
}
```

# Drawbacks

This adds more API surface area, and the specific details between `Sender` and `SharedSender` might be confusing.

# Alternatives

An alternative to the `shared_channel()` function could be adding a `shared()` upgrade method to `Sender` instead. Example:

```rust
impl<T: Send> Sender<T> {
    pub fn shared(self) -> SharedSender<T> {
        // upgrade to Flavor::Shared, take shared::Packet, create SharedSender
    }
}
```

