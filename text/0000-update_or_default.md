- Feature Name: update_or_default
- Start Date: 2025-04-19
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC proposes adding a new method, `Option::update_or_default`, to the standard library’s `Option<T>` type. The method replaces the current value with the result of applying a user-provided closure to it, or, if the option is `None`, inserts `T::default()` before applying the closure. This streamlines a common pattern of initializing an `Option` with a default and transforming it in place, reducing boilerplate and improving readability.

# Motivation
[motivation]: #motivation

Rust programmers frequently encounter a pattern when working with `Option<T>`:

1. Check if the `Option` is `None`.
2. Provide a default value (often `T::default()`).
3. Apply a transformation to the value.
4. Store the result back in the `Option`.

This pattern is prevalent in scenarios like configuration parsing, state machines, or incremental state updates. Currently, this requires verbose boilerplate, such as:

```rust
let mut opt: Option<String> = None;
let s = opt.get_or_insert_with(|| String::new());
*s = s.clone() + " world";
```

or:

```rust
let mut opt: Option<String> = None;
opt = Some(opt.take().unwrap_or_default() + " world");
```

These approaches are cumbersome, requiring multiple steps or temporary variables, which increases cognitive load and the risk of errors (e.g., forgetting to update the value or introducing borrowing issues). The proposed `update_or_default` method simplifies this into a single operation:

```rust
opt.update_or_default(|mut s| { s.push_str(" world"); s });
```

This method:

- Reduces boilerplate by combining default insertion and transformation.
- Improves readability with a clear, purpose-built API.
- Aligns with ergonomic APIs in `std`, such as `HashMap::entry().or_default()` or `Vec::push`.
- Minimizes error-prone manual reference handling compared to `get_or_insert_with`.

### Real-World Use Cases

1. **Configuration Parsing**:

   ```rust
   struct Config { verbose: bool, log_level: u8 }
   impl Default for Config { fn default() -> Self { Config { verbose: false, log_level: 0 } } }
   
   let mut config: Option<Config> = None;
   config.update_or_default(|mut c| { c.verbose = true; c.log_level += 1; c });
   // Result: Some(Config { verbose: true, log_level: 1 })
   ```

2. **State Machines**:

   ```rust
   struct State { counter: i32 }
   impl Default for State { fn default() -> Self { State { counter: 0 } } }
   
   let mut state: Option<State> = None;
   state.update_or_default(|mut s| { s.counter += 1; s });
   // Result: Some(State { counter: 1 })
   ```

3. **Incremental Updates**:

   ```rust
   let mut buffer: Option<String> = None;
   buffer.update_or_default(|mut s| { s.push_str("hello "); s });
   buffer.update_or_default(|mut s| { s.push_str("world"); s });
   // Result: Some("hello world")
   ```

These examples show how `update_or_default` simplifies workflows, making code more maintainable and less error-prone.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `update_or_default` method updates an `Option<T>` in place, either by transforming its current value or by initializing it with `T::default()` and then transforming it. Rust programmers can think of it as a combination of `unwrap_or_default` and `map`, but performed in-place to avoid unnecessary allocations.

### Example Usage

```rust
let mut opt: Option<String> = None;
// If `opt` is `None`, default to String::new(), then append " world"
opt.update_or_default(|mut s| { s.push_str(" world"); s });
assert_eq!(opt, Some(" world".to_string()));

// On second call, transform the existing string in place
opt.update_or_default(|mut s| { s.insert_str(0, "Hello"); s });
assert_eq!(opt, Some("Hello world".to_string()));
```

Compare this to using `get_or_insert_with`:

```rust
let mut opt: Option<String> = None;
let s = opt.get_or_insert_with(|| String::new());
s.push_str(" world");
// Second update
s.insert_str(0, "Hello");
```

The `update_or_default` approach is more concise and encapsulates the transformation logic.

### When to Use

Use `update_or_default` when you need to:

- Initialize an `Option` with `T::default()` if `None`.
- Apply a transformation to the value.
- Store the result back in the `Option`.

For initialization without transformation, use `get_or_insert_with`. For expensive defaults, consider `get_or_insert_with` with a custom closure.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The proposed implementation is:

```rust
impl<T: Default> Option<T> {
    /// If `self` is `Some(t)`, replaces it with `Some(f(t))`;
    /// otherwise, inserts `T::default()`, applies `f`, and stores the result.
    #[unstable(feature = "option_update_or_default", issue = "rust-lang/rust#0000")]
    pub fn update_or_default<F>(&mut self, f: F)
    where
        F: FnOnce(T) -> T,
    {
        let value = self.take().unwrap_or_default();
        *self = Some(f(value));
    }
}
```

### Semantics

- **Input**: A mutable `Option<T>` and a closure `f: FnOnce(T) -> T`.
- **Behavior**:
  - If `self` is `Some(t)`, extracts `t` using `take()`.
  - If `self` is `None`, uses `T::default()`.
  - Applies `f` to the value (existing or default).
  - Stores the result as `Some(f(value))`.
- **Guarantees**:
  - The closure `f` is called exactly once.
  - The `Option` is always `Some` after execution.

### Performance Notes

- **Default Cost**: `T::default()` is called only when `self` is `None`. For types with trivial defaults (e.g., `i32::default()` → `0`), this is allocation-free. For types like `String` or `Vec`, `T::default()` may allocate (e.g., `String::new()` creates an empty `String` with metadata). Users with expensive defaults should use `get_or_insert_with`.
- **Closure**: The closure `f` may allocate (e.g., `|mut s: String| { s.push_str(" world"); s }` may resize the buffer).
- **Allocation**: The method’s logic (`take()`, `unwrap_or_default()`, `Some`) is allocation-free, introducing no additional allocations beyond those from `T::default()` or `f`. For types like `String`, allocations may occur in `T::default()` or `f`, but no intermediate `T` values are created.

### Interaction and Corner Cases

- **Expensive Defaults**: Use `get_or_insert_with` for costly `T::default()`.
- **Borrowing**: Consumes the inner value via `take()`, avoiding borrowing issues.
- **Return Value**: Does not return a reference, keeping the API simple.

# Drawbacks
[drawbacks]: #drawbacks

- **API Surface**: Adds a new method to `Option<T>`. However, `Option` is a core type, and `update_or_default` targets a common pattern, justifying inclusion like `unwrap_or_default`.
- **Overlap**: Some may argue `get_or_insert_with` or `map` suffice. However, `update_or_default` is more ergonomic for transformations, reducing boilerplate and errors.
- **Allocation for Heap-Allocated Types**: For types like `String`, `T::default()` or `f` may allocate. Users in performance-critical code may prefer `get_or_insert_with` for control over defaults.
- **Learning Curve**: New users may need to learn when to use `update_or_default` versus `get_or_insert_with`. Clear documentation mitigates this.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### Why This Design?

The `update_or_default` method is the best solution because it:

- Combines default insertion and transformation in one ergonomic call.
- Aligns with `std`’s ergonomic APIs (e.g., `HashMap::entry().or_default()`).
- Reduces boilerplate and errors compared to multi-step alternatives.
- Uses a simple, allocation-free implementation.

### Why `update_or_default`?

The name is clear and consistent:

- **Update**: Emphasizes in-place transformation.
- **Or_default**: Mirrors `unwrap_or_default` and `or_default`, signaling default insertion.
- Alternatives:
  - `map_or_default`: Suggests a new type `U`, not in-place mutation.
  - `modify_or_default`: Less common in `std` naming.
  - `replace_or_default`: Ambiguous about transformation.

### Comparison with `get_or_insert_with`

While `get_or_insert_with` supports default insertion, it returns a mutable reference, requiring separate transformation:

```rust
let mut opt: Option<String> = None;
let s = opt.get_or_insert_with(|| String::new());
s.push_str(" world");
```

`update_or_default` combines both steps:

```rust
opt.update_or_default(|mut s| { s.push_str(" world"); s });
```

This is more concise, ensures in-place updates, and avoids reference management. `update_or_default` targets transformations, while `get_or_insert_with` is for initialization.

### Alternatives

1. **Library Crates**:

   - Crates like `OptionExt` could add this, but `std` inclusion ensures discoverability.

2. **No New Method**:

   - Accepting boilerplate sacrifices ergonomics and readability.

3. **Compose Existing Methods**:

   - Combining `take`, `unwrap_or_default`, and `map` is verbose and error-prone:

     ```rust
     let mut opt: Option<String> = None;
     opt = Some(opt.take().unwrap_or_default() + " world");
     ```

### Impact of Not Doing This

Without `update_or_default`, Rust programmers will continue using verbose patterns, leading to less readable and maintainable code. This could hinder adoption in scenarios where ergonomic state updates are critical.

### Library vs. Language

This is a library addition, not a language change, and belongs in `std` due to `Option`’s core status. A macro or external crate is less discoverable and less consistent with `std`’s ergonomic APIs.

# Prior art
[prior-art]: #prior-art

- **Rust**:

  - `Option::unwrap_or_default()`: Extracts `T` with a default, no in-place transformation.
  - `Option::get_or_insert_with(default_fn) -> &mut T`: Initializes but requires separate transformation.
  - `Option::map_or(default, f) -> U`: Maps to a new type, not in-place.
  - `HashMap::entry().or_default().and_modify(f)`: Similar in-place update pattern.

- **Other Languages**:

  - **JavaScript**: `Map.prototype.set(key, map.get(key) ?? defaultValue)` updates or initializes values, analogous to `update_or_default`.
  - **Haskell**: `maybe` with a default and transformation resembles `map_or` but lacks in-place mutation.
  - **C++**: `std::optional::value_or` extracts with a default, no in-place transformation.

- **Libraries**: Libraries like Lodash provide default-and-transform utilities, though not in-place.

Rust’s focus on ergonomics and zero-cost abstractions makes `update_or_default` a natural fit, diverging from languages with less emphasis on in-place mutation.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- **Return Value**: Should `update_or_default` return `&mut T` for chaining mutations? E.g.:

  ```rust
  opt.update_or_default(|s| { s.push_str(" world"); s }).make_ascii_uppercase();
  ```

  This could complicate the API and introduce borrowing issues.

- **Generalization**: Should there be `update_or_insert_with(default, f)` for separate default and update closures? E.g.:

  ```rust
  fn update_or_insert_with<D, F>(&mut self, default: D, f: F)
  where
      D: FnOnce() -> T,
      F: FnOnce(T) -> T;
  ```

  This is more flexible but less ergonomic for `T::default()`.

- **Feature Gate**: Is `option_update_or_default` the best feature gate name?

These will be resolved through RFC discussion and nightly experimentation before stabilization.

# Future possibilities
[future-possibilities]: #future-possibilities

- **In-Place Mapping**: `Option::transform` for in-place mapping without default fallback.
- **Extend to** `Result`: `Result::update_or_else(default, f)` for similar patterns.
- **Chaining APIs**: Explore returning `&mut T` or a builder-like API for complex updates.

These extensions would build on `update_or_default`’s ergonomics, aligning with Rust’s roadmap for improving standard library usability.
