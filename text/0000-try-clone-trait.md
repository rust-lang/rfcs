- try-clone-trait
- Start Date: 2017-10-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
The standard library provides the `Clone` trait to duplicate an object. However,
there is no way to indicate fallibility of the operation. This RFC proposes the
`TryClone` trait that supports fallible clones. 

# Motivation
Failing to clone an object is fairly common, for example, the system runs out of resources to represent the underlying type.
Implementations have worked around this in two ways. Some types in the standard library has a `try_clone` method that returns a 
`Result<T>`. A number of [crates](https://github.com/search?l=Rust&q=TryClone&type=Code&utf8=%E2%9C%93) have defined `TryClone` as a trait and implemented it on standard
types.

One example is [Clone](https://doc.rust-lang.org/src/alloc/arc.rs.html#710) for `std::alloc::arc`. This is better represented
by returning a `Result<Arc>`. Doing that will also make the unsafe block unnecessary. Also, this will enable
the caller to handle these errors idiomatically instead of aborting the process.
Currently, it looks like this
```rust
fn clone(&self) -> Arc<T> {
    let old_size = self.inner().strong.fetch_add(1, Relaxed);
    if old_size > MAX_REFCOUNT {
        unsafe {
            abort();
        }
    }
    Arc { ptr: self.ptr }
}
```
This will become
```rust
fn clone(&self) -> Result<Arc<T>> {
    let old_size = self.inner().strong.fetch_add(1, Relaxed);
    if old_size > MAX_REFCOUNT {
        Err("Overflowed MAX_REFCOUNT")
    }
    Ok(Arc { ptr: self.ptr })
}
```

# Detailed design
This trait should be added in `std::clone` and should be called `TryClone`. It should look similar
to `TryFrom`

```rust
pub trait TryClone: Sized {
    type Error;
    fn try_clone() -> Result<Self, Self::Error>;
    fn try_clone_from(&mut self, source: &Self) -> Result<(), Self::Error>;
}
```
The standard library should also provide implementations for the following types:
- `std::fs::File`
- `std::net::UdpSocket`
- `std::net::TcpListener`
- `std::net::TcpStream`
- `std::os::unix::net::UnixDatagram`
- `std::os::unix::net::UnixStream`
- `std::os::unix::net::UnixListener` 

# Drawbacks
One more trait to manage and stabilize. Also, making sure this is backwards compatible
will require some work.

# Alternatives
The status quo, keep everything as-is.

# Unresolved questions
Do we need this functionality? In case we do not need this, implementing the RFC will produce extra maintenance
overhead.
