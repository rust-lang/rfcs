- Feature Name: safe-deref
- Start Date: 2017-01-26
- RFC PR: 
- Rust Issue: 

# Summary
[summary]: #summary

Add `SafeDeref` and `SafeDerefMut` trait to the standard library, equivalent to `Deref` and
`DerefMut` but which are guaranteed to always return the same object.

# Motivation
[motivation]: #motivation

Imagine a C API that looks like this:

```c
// Creates a connection to a database.
connection_t* create_connection();
// Destroys a connection. You must destroy all queries before destroying a connection.
void delete_connection(connection_t*);

// Creates a new SQL query using the connection. Returns the ID of the query.
int create_query(connection_t*, const str*);
// Destroys a query. The query id must be valid for this connection.
void delete_query(connection_t*, int);
// Starts a query. The query id must be valid for this connection.
void start_query(connection_t*, int);
// Gets the results of the query. The query id must be valid for this connection.
some_type get_query_results(connection_t*, int);
```

The usage pretty straight-forward, but take note of the comments and requirements of the API.
In order to make a query you are supposed to call `create_query`, which will return an ID. In order
to manipulate the query, you must then pass both the connection and the ID to the API.

One can wrap around this API like this in Rust (skipping some functions):

```rust
pub struct Connection {
    ptr: *mut ffi::connection_t
}

impl Drop for Connection {
    unsafe { ffi::delete_connection(self.ptr) }
}

pub struct Query<'a> {
    connection: &'a Connection,
    id: i32,
}

impl<'a> Query<'a> {
    pub fn start(&self) {
        unsafe { ffi::start_query(self.connection.ptr, self.id) }
    }
}

impl<'a> Drop for Query<'a> {
    unsafe { ffi::delete_query(self.connection.ptr, self.id) }
}
```

Everything works well, and everything is safe.

But after a few days someone opens an issue because for example they would like to distribute
queries amongst threads and can't because of the lifetime. In order to solve the problem, you
rewrite your code and change the `Query` to be generic:

```rust
pub struct Connection {
    ptr: *mut ffi::connection_t
}

impl Drop for Connection {
    unsafe { ffi::delete_connection(self.ptr) }
}

pub struct Query<P> where P: Deref<Target = Connection> {
    connection: P,
    id: i32,
}

impl<P> Query<P> where P: Deref<Target = Connection> {
    pub fn start(&self) {
        unsafe { ffi::start_query(self.connection.ptr, self.id) }
    }
}

impl<P> Drop for Query<P> where P: Deref<Target = Connection> {
    unsafe { ffi::delete_query(self.connection.ptr, self.id) }
}
```

This way the user can either use a `Query<&'a Connection>` or a `Query<Arc<Connection>>`, depending
on what suits them best.

Everything is fine, right? Wrong! Because objects that implement `Deref`/`DerefMut` are not
guaranteed to return the same object every time. For example the user can do this:

```rust
pub struct MyFancyArc {
    a: Arc<Connection>,
    b: Arc<Connection>,
}

impl Deref for MyFancyArc {
    type Target = Connection;

    fn deref(&self) -> &Connection {
        if rand::random::<f32>() < 0.5 {
            &*self.a
        } else {
            &*self.b
        }
    }
}
```

And then use a `Query<MyFancyArc>`. And if they do so then the wrapper becomes unsound, because it
is possible to call `delete_query` with a wrong connection/query id pair.

## Solving the problem

As a library writer, I can see three ways to solve this problem.

The first way would be to put the `*mut connection_t` inside some sort of internal hidden `Arc`
shared by the `Connection` and the `Query`. But if you do so you might as well force a
`Query<Arc<Connection>>` anyway, as the benefits of using a lightweight reference disappear.

The second way would be to store the `*mut connection_t` inside the `Query` struct, and before
every single FFI call that uses the query we check that the `*mut connection_t` inside the
`Connection` matches the `*mut connection_t` inside the `Query`.

In other words, one would have to write this:

```rust
pub struct Query<P> where P: Deref<Target = Connection> {
    connection: P,
    connection_ptr: *mut connection_t,
    id: i32,
}

impl<P> Query<P> where P: Deref<Target = Connection> {
    pub fn start(&self) {
        let connec = self.connection.deref();
        assert_eq!(connec.ptr, self.connection_ptr);
        unsafe { ffi::start_query(connec.ptr, self.id) }
    }
}
```

This approach has three major drawbacks:

- You add a runtime overhead at every single function call. The `Query` object, which was supposed
  to be lightweight now performs checks that will be false most of the time anyway. Ideologically
  it is pretty bad to have to add a runtime check for what is a weakness of the safe/unsafe
  mechanism of the Rust language.
- It is really painful to write and it is too easy to miss a check somewhere.
- It doesn't prevent the `Connection` from being destroyed while there are queries still alive.
  A rogue implementation of `Deref` can choose to destroy its content at any moment thanks to a
  background thread for example.

The third way to solve the problem, which is proposed in this RFC, is to add new traits named
`SafeDeref` and `SafeDerefMut` that guarantee that they always return the same object and that
their content will outlast them.

# Detailed design
[design]: #detailed-design

Add the following traits to `std::sync`:

```rust
unsafe trait SafeDeref: Deref {}
unsafe trait SafeDerefMut: SafeDeref + DerefMut {}
```

Types that implement this trait are guaranteed to always return the same object and that their
content lives at least as long as they do.
Most of the implementations of `Deref`/`DerefMut` should match these criterias.

Implement these traits on all the types of the standard library that already implement respectively
`Deref` and `DerefMut`.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

This feature is oriented towards people who write unsafe code, and thus doesn't need to appear
in any beginner-oriented book.
A warning should be added to the documentation of `Deref` and `DerefMut` and in the rustnomicon to
make it clear that they shouldn't be relied upon when writing unsafe code.

# Drawbacks
[drawbacks]: #drawbacks

The major drawback is that library writers that create their own `Deref`-implementing objects are
possibly going to add an unsafe trait implementation to their code.

In other words a codebase that could be entirely safe could therefore become unsafe.

# Alternatives
[alternatives]: #alternatives

- One possible alternative is to modify the `Deref` and `DerefMut` traits directly.
In the author's opinion this would be the best thing to do, however this would require adding
`unsafe` to these traits and would be a breaking change.

- One could also argue that C APIs that look like the motivating example are simply badly designed,
and that writing a rogue implementation of `Deref` should always result in either a logic error or
a panic because in the end we're only manipulating memory after all.

- The prefix `Safe` is maybe not great.

# Unresolved questions
[unresolved]: #unresolved-questions

None?
