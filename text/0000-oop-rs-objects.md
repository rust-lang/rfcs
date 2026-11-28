# Compiler-supported objects based on oop_rs

- Feature Name: `oop_rs_objects`
- Start Date: 2026-11-28
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

Allow Rust libraries to define objects whose behavior can be extended with reusable, per-instance state. This proposal adopts a subset of oop_rs's declarations for classes, interfaces, and ordered mixins. A mixin can extend a method by calling its earlier implementation on the same object. Under the proposed built-in attribute, Rust checks the complete composition and uses it to generate storage, construction, view conversions, and method dispatch. An opaque reference-counted handle lets callers share an object through different views.

## Motivation
[motivation]: #motivation

Consider a command library. A caller holding a command can invoke `run()` whether the command performs a check or an export. Both commands need the same preparation before they execute, so the base method `Command::run()` does the preparation and calls `self.execute()`. The check and the export each supply their own implementation of `execute()`.

Now suppose another library wants to count completed runs. Calling `run()` through a command handle should update the count for that particular command. A fresh export command should still report zero, and running a check should leave the export's count alone. The author of the counter should be able to reuse both the field and the counting code across command types.

Rust can already express the shared preparation as a default trait method. The extra work comes when the counter is packaged as a separate extension with its own state: someone must arrange where each count lives, how callers read it, and which `run()` body gets called next. With oop_rs, the author can declare a mixin containing a count field and a `run()` override. For an export command, the mixin calls the earlier `Command::run()`. That method still sees the export command as `self`, so its call to `execute()` performs the export. When it returns, the mixin increments the count on that object. Applying the same mixin to a check command gives it the same behavior and its own count.

Why involve the compiler if oop_rs already expresses this? Today the macro and runtime translate these declarations into Rust types, accessors, and dispatch code, and Rust checks the generated program. This proposal would make the declared relationships themselves part of Rust's type checking. For example, a mixin might name an earlier operation that has no implementation in a particular host. The compiler would check that call against the host's actual composition and identify the offending call and host. For a valid composition it would use the same checked method order to generate dispatch, including calls made through a different view of the object.

The case for compiler support is strongest when libraries supply commands and extensions independently, so the author of an extension cannot enumerate the combinations in advance. The guide follows one combination from declaration to execution. The alternatives section compares what its authors would have to specify using ordinary Rust or the existing oop_rs library.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

### Declaring a command and a reusable counter

The runner invokes each command through `run()`. The base method performs the shared preparation before calling `execute()` to do the command's own work. We will give check and export commands a counter that records how many runs have returned. The small integer returned by each command makes it easy to see which implementation ran.

The following examples use the proposed built-in attribute `#[core::oop::class]`. Its declaration forms come from oop_rs. The existing library's `#[class]` invokes a procedural macro. The qualified attribute asks the compiler to check an object declaration and generate its implementation. Within that attribute, `type` names an object view, `class` introduces its state and methods, and a `let` member declares a field on each instance.

```rust
#![feature(oop_rs_objects)]
extern crate alloc;
use alloc::oop::CRc;

#[core::oop::class]
type Command = class<{
    let mut prepared: bool = false;

    pub fn new() -> Self { Self { .. } }

    pub fn run(&self) -> u32 {
        self.set().prepared(true);
        self.execute()
    }

    pub fn execute(&self) -> u32 { 0 }

    pub fn is_prepared(&self) -> bool { self.get().prepared() }
},>;
```

`Command::run` contains the preparation step. Its ordinary call to `self.execute()` allows a derived command to supply the work. The default implementation returns zero. The generated `get()` and `set()` accessors read and write the `bool` field through `&self` using cell-style interior mutability.

The counter uses an interface for reading the count and a mixin that supplies the field and the counting logic.

```rust
#[core::oop::class]
type RunStats = interface<{
    pub fn run_count(&self) -> u32;
},>;

#[core::oop::class(on(Command), implements(RunStats))]
type CountedRun = mixin<{
    let mut count: u32 = 0;

    #[method(override(Command))]
    pub fn run(&self) -> u32 {
        let result = super.run();
        self.set().count(self.get().count() + 1);
        result
    }

    #[method(override(RunStats))]
    pub fn run_count(&self) -> u32 { self.get().count() }
},>;
```

`on(Command)` requires each complete object using this mixin to support the `Command` view. `override(Command)` identifies the operation that the mixin extends. `super.run()` calls the earlier implementation for the same object. Once that call returns, the mixin increments its own field and passes the result back to the caller. `implements(RunStats)` adds a view through which callers can read the counter. The mixin supplies the first implementation of the interface's `run_count` operation.

Both commands use this mixin. Each supplies its own implementation of `execute()`.

```rust
#[core::oop::class(extends(Command), with(CountedRun))]
type CheckCommand = class<{
    pub fn new() -> Self { Self { ..Super::new() } }

    #[method(override(Command))]
    pub fn execute(&self) -> u32 { 1 }
},>;

#[core::oop::class(extends(Command), with(CountedRun))]
type ExportCommand = class<{
    pub fn new() -> Self { Self { ..Super::new() } }

    #[method(override(Command))]
    pub fn execute(&self) -> u32 { 2 }
},>;
```

`extends(Command)` contributes the base fields and methods. `with(CountedRun)` adds a fresh counter to each constructed object. A constructor written `-> Self` creates one complete object and returns an owning handle, here `CRc<CheckCommand>` or `CRc<ExportCommand>`. Capitalized `Super::new()` initializes the base portion inside that allocation.

### Following a run on the same object

Calling `export.run()` on an `ExportCommand` starts in the counting mixin. Each step in the trace below uses `export` as its receiver.

```text
CountedRun.run
  super.run() enters Command.run
    set export.prepared to true
    self.execute() enters ExportCommand.execute and returns 2
  increment export.count
  return 2
```

The two calls inside that trace answer different questions. At `super.run()`, the compiler looks back from the mixin's position in this composition for the earlier `run` body. At `self.execute()`, it selects the final implementation for the object running the command. Both calls use the same receiver. With a `CheckCommand` receiver, the same two `run` bodies execute and `self.execute()` returns `1`.

```rust
fn main() {
    let export = ExportCommand::new();
    let untouched = ExportCommand::new();
    let check = CheckCommand::new();

    assert_eq!(export.run(), 2);
    assert_eq!(check.run(), 1);
    assert!(export.is_prepared());
    assert!(check.is_prepared());
    assert!(!untouched.is_prepared());

    assert_eq!(export.run_count(), 1);
    assert_eq!(check.run_count(), 1);
    assert_eq!(untouched.run_count(), 0);
}
```

The base records preparation, the concrete command supplies the result, and the mixin records the completed run. The untouched export command keeps its initial field values.

### Sharing commands and reading their statistics

A command runner can hold a `CRc<Command>` obtained from a more specific command. A separate caller can use the `RunStats` view of the same object to inspect the effect of a run.

```rust
let export = ExportCommand::new();
let cmd: CRc<Command> = CRc::upcast::<Command>(&export);
let stats = CRc::cast::<RunStats>(&cmd).unwrap();

assert_eq!(cmd.run(), 2);
assert_eq!(stats.run_count(), 1);
assert!(CRc::ptr_eq(&cmd, &stats));
assert!(CRc::cast::<CheckCommand>(&cmd).is_none());
```

The upcast is guaranteed by `extends(Command)`. Given only the `Command` view, the cast to `RunStats` checks the actual object's views. Each successful conversion acquires an owning share of the same allocation. The failed cast leaves `cmd` usable. Cloning a handle shares the object. Borrowing it yields a reference through its current view, with calls and borrowed conversions tied to the reference's lifetime.

A command may hold a child command and run it when requested. Its class could include the following field and methods.

```rust
let ref mut child: Option<CRc<Command>> = None;

pub fn set_child(&self, child: CRc<Command>) {
    *self.set().child() = Some(child);
}

pub fn run_child(&self) -> Option<u32> {
    let child = {
        let guard = self.get().child();
        guard.as_ref().map(CRc::clone)
    };
    child.map(|cmd| cmd.run())
}
```

`let ref mut` gives access to the field through borrow guards. Here the block clones the child's handle and releases the guard before calling the child, so a callback during `run()` can access the field again. A callback that should keep only a weak link to a command can capture `CRc::downgrade(&cmd)` from its owning handle and use `CWeak::upgrade` when invoked.

### Checking an invalid command composition

Applying `CountedRun` to a class without the `Command` view fails the mixin's `on(Command)` requirement. A different error would arise if the body of `CountedRun::run_count` called `super.run_count()` in either example class. The `RunStats` interface declares that operation, but there is no earlier concrete body for the call to select. A diagnostic can point to the failing call and identify the class composition being checked.

The same check on each concrete class verifies that its override targets exist, its operations have implementations, and its earlier calls have valid targets. The compiler also checks relationship kinds and rejects cycles when processing declarations. These checks apply to the composition from which the compiler generates the object's state and calls.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

### Entry point and initial scope

Introduce a built-in attribute at `core::oop::class`, initially gated by `oop_rs_objects`, and object handles in `alloc::oop`. An outer declaration has the form `type Name = class<{ members },>;`, or uses `interface` or `mixin`. The parser preserves the annotated declaration body until attribute resolution selects the built-in, then parses object members. Selection uses the resolved built-in identity; a similarly named third-party macro does not select it. The result is a compiler object declaration that will drive executable lowering.

Classes allow an optional `abstract` marker, one `extends` class, ordered `with` mixins, and `implements` interfaces. For example, `#[core::oop::class(abstract, extends(Command))]` declares an abstract derived class. Mixins allow ordinary `on` requirements and `implements` interfaces. An `on` target may be a class, interface, or mixin. Interfaces contain abstract instance signatures. The initial subset excludes interface inheritance/default bodies, source-level type/const/lifetime parameters, and repeated application of one mixin in a complete inheritance chain. Separate classes may each apply the same mixin. The library form `on(#[mixin] M)` is outside this subset.

Instance methods use `&self`, dynamically compatible signatures, and ordinary Rust expressions. Method-local lifetime parameters are supported; type/const parameters, consuming or mutable receivers, `async`, `const`, and unsafe method signatures are outside the subset. `Self` is implicit in the receiver and is permitted in a constructor's `-> Self` and `Self { ... }` initializer. Other explicit `Self` type expressions in signatures or bodies are rejected; use named views such as `&Command` or `CRc<Command>`. The hidden final-host type is not a source-level `Self` type available to user methods. Associated types, constants, ordinary associated functions, and `where Self: Sized` methods are outside the subset. Ordinary unsafe blocks in a method body remain subject to Rust's usual unsafe rules. Stored field types must be sized and `'static`; method and constructor arguments may be borrowed for shorter lifetimes under ordinary Rust rules. Views are unsized and cannot be stored by value. This first model uses allocation and single-threaded ownership, without stack objects or an `Arc` variant.

An associated declaration without a receiver, returning `Self`, and with a single `Self { field: expression, ..., .. }` initializer body is a constructor. A derived initializer uses `..Super::constructor(arguments)`. Constructor names are not restricted to `new`; several distinct names are permitted, with no overloading. Constructors are not inherited. A class with no constructor has no allocating entry point of its own. Parameters have ordinary Rust types. Executable statements, object access, and lifecycle hooks inside constructor bodies are outside the subset; initializer expressions may call ordinary functions. Mixins have default initializers for all fields and no constructors. These rules define construction without exposing partially initialized objects to user code.

### Names, views, and complete objects

Each declaration has a nominal identity owned by its crate. Imports and re-exports preserve it. Its source name denotes an unsized view, represented by a compiler-generated object-safe trait object. The generated trait's implementations are controlled by the object compiler. Each concrete class also has a hidden sized complete-object type containing all composition state; that type implements its applicable views.

The compiler may implement those traits for a downstream class using the built-in after checking its relationships. Ordinary Rust `impl` blocks cannot implement or alter generated view traits, implement traits for hidden complete-object types, or forge view descriptors. Users may implement ordinary traits for their own wrappers around handles. Generated methods and accessors retain source visibility; cross-crate inheritance grants no access to private fields.

Each view has a fixed set of statically guaranteed views. An interface guarantees itself. A mixin guarantees itself, its declared interfaces, its `on` targets, and their guarantees. A class guarantees itself, its base, its applied mixins, its declared interfaces, and their guarantees. Take the least fixed-point closure of these rules. This set determines static lookup and upcasts. In contrast, the actual views of a complete object come only from its real `extends`, `with`, and `implements` composition: `on` adds requirements, not actual views. Check requirements against this actual set, so `on(I)` cannot manufacture its own satisfaction.

An operation is identified by its declaring view and method declaration. Equal names in different views are separate operations. Unqualified lookup uses the declaration's fixed guarantee set, deduplicating the same operation; several remaining identities require an explicit view. Resolve operation identities when checking the declaring body, before a final host chooses their implementations. Later host layers cannot change this lookup by adding names. Within object bodies, `(self as View).f(...)` selects an operation through a guaranteed view and retains final-object dispatch. Outside object bodies, `core::oop::cast_ref::<View>(borrowed).unwrap().f(...)` selects a view through the borrowed-conversion API. Visibility applies in both cases. The parenthesized object syntax does not change ordinary Rust `as` casts. Views added incidentally by a later host require dynamic conversion.

For an operation directly declared by V, `V::f(view, ...)` is also supported when `view` has type `&V`. This qualified call uses final-object dispatch; it does not select V's earlier body. Borrowed conversion can supply the required `&V`.

### Composition, on requirements, and overrides

For a class, recursively expand its base, apply `with` entries in written order, then include its own declarations. Reject cycles, repeated mixin identities, and invalid relationship kinds. The resulting sequence defines state order and each body's position. Interface implementation adds operations and no state.

Compute an operation map before and after each position. A new view introduces operations; an existing view keeps its current bodies. An unmarked method declares an operation in its own view. `#[method(override(A, B))]` supplies a body for each named view's operation after checking receiver, argument, return, and lifetime compatibility. Each target must directly declare the named operation and be available through the declaration's relationships or `on` requirements. First implementations of abstract interface methods are valid. An abstract declaration never clears an earlier body.

A later override replaces the earlier selection. Signature compatibility means identical resolved argument and return types and lifetime-binder structure after renaming bound lifetime names; there is no additional argument or result subtyping rule. The receiver contract is the same shared complete object. Duplicate targets, duplicate implementations in one declaration, and signature mismatches are errors. An ordinary class must satisfy all operations, `on` requirements, and earlier-call obligations even if it declares no constructor. A class explicitly marked `abstract` may retain these host-dependent obligations for a concrete descendant. Its constructors are available only as base initializers, not allocating calls. Names, relationship kinds, cycles, and signature errors are checked immediately in abstract classes and mixins as well.

Each ordinary `on(V)` checks the actual views of the final complete object. In oop_rs terminology, `__Self` denotes that final host, including layers added after the current mixin. A required mixin must occur in that composition; a later layer may satisfy the requirement. `on` satisfaction does not establish an earlier body. Abstract bases and unapplied mixins retain host-dependent obligations until an ordinary class is checked.

Deferring an earlier-call check does not change its position. A later layer cannot repair a missing body in the fixed prefix before an abstract base's method.

### Ordinary calls and super calls

Ordinary calls select the final implementation of the resolved operation for the actual complete object. Every generated body borrows that same complete object with its declaring layer's field permissions. Reusable bodies are instantiated for the host while preserving receiver identity.

An unqualified `super.f(...)` obtains ordered candidate views from its containing method's `override(...)` list. The requested name may differ from the containing method's name. Resolve each requested operation and examine the state before this body's position. A nonexistent operation is an error; skip views absent from that earlier state and operations without earlier concrete bodies, selecting the first available body. An unmarked method has no default list. Explicit `(super as V).f(...)` uses only V, including in an unmarked method, and fails if that earlier view or body is unavailable.

The compiler checks and generates the selected earlier invocation. Its complete-host argument is the current hidden concrete type. Each body's earlier context is the prefix of the composition before that body's own layer, including when called from a later layer. In oop_rs terminology, `__Self` names the final host while `__Super` describes this earlier prefix. These two roles do not change the receiver's identity. Visibility checks apply, and an inherited operation is named through its declaring view in this subset.

In the command example, `super.run()` in `CountedRun` selects `Command::run`. The call to `self.execute()` inside that body selects `CheckCommand::execute` on a check command and `ExportCommand::execute` on an export command. Both calls borrow the object on which `run()` was invoked. The result is passed back to `CountedRun`, which increments that object's counter and returns the result.

### Field storage and borrowing

Each state-bearing layer contributes a region within one complete allocation. A base or mixin view does not create another object. Fields have nominal declaring-layer identities, so equally named fields in different layers remain distinct. The initial accessor contract is:

The method names `get` and `set` are reserved for generated accessors; source methods and constructors cannot reuse them. Accessors select fields directly declared by the receiver's static view, using the lexical declaring view for `self` inside a method body. Access to inherited fields, including equally named fields, requires selecting their declaring view explicitly and remains subject to visibility.

| Source form | Storage and access |
| --- | --- |
| `let field: T` | Immutable `T`; `get().field()` returns a clone when `T: Clone`; otherwise no getter is generated. No setter. |
| `let mut field: T` | Requires `T: Copy`; `Cell<T>` semantics; `get().field()` copies and `set().field(value)` replaces. |
| `let ref field: T` | `RefCell<T>` semantics; `get().field()` returns `Ref<'_, T>`; no mutable accessor. |
| `let ref mut field: T` | `RefCell<T>` semantics; `get().field()` returns `Ref<'_, T>` and `set().field()` returns `RefMut<'_, T>`. |

Fields accept ordinary visibility prefixes, such as `pub let` and `pub(crate) let`. Omitting a visibility prefix makes the field private to its declaring context. Generated accessors have that visibility, and inherited views do not expose private fields. A non-Clone immutable field can still be initialized and dropped. Guard lifetimes are tied to the borrowed object. A mutable guard excludes other borrows of the same field until dropped; a conflicting borrow panics as with `RefCell`. A callback invoked while a guard is live retains that restriction, so reentrancy may panic. No whole-object exclusive reference is formed from `&self`. Method and initializer expressions undergo ordinary type and lifetime checking, including guards returned from methods.

The accessor rules above also govern migration. For example, a library field holding an optional owning handle to a subcommand has a non-`Copy` value. If the enclosing command needs to replace that value, its declaration in the proposed subset uses `let ref mut`. The guide shows how to clone the stored handle and release the borrow before invoking the subcommand. Other policies for cloning, lazy initialization, or borrowing require explicit field types or a future extension.

### Construction, publication, and failure

A concrete constructor allocates one uninitialized complete-object region and an allocation owner kept private to generated construction code. Public constructor arguments evaluate left to right before construction. A derived initializer next evaluates `Super::constructor` arguments left to right and initializes the base portion in the same allocation. This occurrence of `Super::constructor` is an initializer invocation, not another allocation.

After the base, initialize newly applied mixins in `with` order and then the class's own fields, each in field declaration order. Explicit field expressions replace defaults and run at their field's turn regardless of their order in the initializer list. Every field needs exactly one explicit initializer or a default; duplicate or missing values are errors. Defaults can use constants and ordinary paths; explicit class initializers can also use constructor parameters. Neither may access `self`, initialized object fields, or a handle under construction. Root `Self { .. }` uses field defaults; a derived initializer names exactly one direct base constructor. Mixins use their field defaults.

Generated code records which fields have completed. It creates no view reference, weak reference, or public owner before all fields are valid and the selected dispatch information is installed. It then publishes the initial `CRc<Class>`. Registration and virtual lifecycle calls occur on that returned handle after construction.

If initialization unwinds, completed fields are dropped in reverse completion order and the unpublished allocation is released. No usable object is returned. Temporaries and arguments use ordinary Rust drop rules, including ordinary double-panic behavior during cleanup and process termination under panic abort. Allocation failure follows the allocation API's behavior.

### Owning handles and view conversions

`alloc::oop::CRc<V>` is opaque and is not an alias for `Rc<V>`. Semantically it contains one strong ownership share of the complete allocation and a compiler-created descriptor for V. The descriptor connects an allowed view to operations, field projections, and complete-object destruction. The owner retains the allocation provenance and layout required for counting and deallocation. An implementation may choose an equivalent representation but cannot recover ownership from a borrowed payload pointer.

The initial handle API has the following signatures, expressed as inputs and results. Here S and T are registered view types; S is inferred from the source handle. `CRc<S>` implements `Clone`, and its associated `clone` function has the same behavior, so `guard.as_ref().map(CRc::clone)` infers S from the guarded handle.

| Call | Input and result |
| --- | --- |
| `CRc::clone(&x)` | `&CRc<S>` → `CRc<S>` |
| `CRc::upcast::<T>(&x)` | `&CRc<S>` → `CRc<T>`; T must be statically guaranteed by S. |
| `CRc::cast::<T>(&x)` | `&CRc<S>` → `Option<CRc<T>>` |
| `CRc::downgrade(&x)` | `&CRc<S>` → `CWeak<S>` |
| `CRc::ptr_eq(&x, &y)` | `&CRc<S>`, `&CRc<T>` → `bool` |

Clone acquires another strong share. Upcast uses the source view's fixed guarantee set. Cast checks T's nominal identity in the actual complete-object view set. Successful conversion acquires a strong share and the selected descriptor; failure acquires none and leaves the source unchanged. Pointer equality compares complete-allocation identity across views.

`Deref<Target = V>` borrows the complete object through V for no longer than the owner borrow. In object syntax, `(self as V)` is statically permitted when the method's fixed guarantee set includes V. The built-in `core::oop::cast_ref::<T>(source)` takes `&'a S` and returns `Option<&'a T>` using compiler-managed metadata; its calling convention takes only T as an explicit type argument and infers the registered source view S. The result cannot outlive that source borrow. Borrowed conversion creates no ownership share; ordinary references, manual trait implementations, and user descriptors cannot manufacture a view or owner.

`CWeak<V>` contains one weak share and a view identity. Downgrade acquires it from a live owner. `CWeak::clone` acquires another weak share; `CWeak::upgrade` returns `Some(CRc<V>)` only while the strong count is nonzero. Dropping a weak handle releases its own share. After payload destruction, control storage remains until the final weak share is released. Upgrade never dereferences a dead view. The initial API excludes raw-pointer constructors, `into_inner`/`try_unwrap`, recovery of ownership from `&self`, and user-defined view metadata.

### Destruction and Rust interoperability

Dropping the final strong owner first makes weak upgrades fail, then drops fields in reverse initialization order: final-class fields in reverse declaration order, its mixins in reverse order, then recursively the base state. Each field is scheduled exactly once. Field types may have ordinary Rust `Drop`; unwinding uses Rust cleanup rules, including termination on a second panic during cleanup. Payload and control storage are released when their ownership counts permit. Reference cycles can retain objects, as with `Rc`.

This subset has no object-level `Drop` implementation or virtual destruction hook. Extra cleanup can be supplied by ordinary Rust values stored in fields. The hidden complete object is not exposed by value, published fields cannot be moved out through object APIs, and cloning copies an ownership share rather than state.

`CRc<V>` and `CWeak<V>` are always `!Send` and `!Sync`. Object views are also `!Send` and `!Sync`, so a borrowed view cannot bypass the thread boundary. Views are `!Unpin`; owning handles are `Unpin`, and moving a handle does not move its allocation. No structural pin projection or self-referential constructor is provided. Ordinary `Pin` wrappers remain governed by their API bounds and grant no additional object operations. Initial views and handles are `!UnwindSafe` and `!RefUnwindSafe`; ordinary explicit assertion facilities remain available. These rules hold independently of storage optimizations.

Ordinary Rust values, containers, closures, and handles can be fields if they satisfy the selected contract and `'static`. Method bodies follow normal visibility and lifetime rules. Compiler-controlled view implementations prevent manual impls from claiming object capabilities or replacing slots; additional ordinary trait behavior belongs on user wrapper types. No stable layout or FFI ABI is promised, and safe FFI code cannot forge object views.

### Compiler realization and cross-crate use

Resolve declaration identities, signatures, and relationship kinds, then build each concrete composition. Check final-host requirements, operation completeness, and call selection before generating the hidden complete-object representation. Internal implementation artifacts may be generic even though source declarations in this subset are not.

One checked representation drives storage and initialization/drop glue, descriptors and function slots, and method bodies with checked field projections and ordinary/earlier calls. A reusable body is instantiated for the actual complete-host type and the earlier prefix before its own layer. The compiler fills each table entry from the selected body; a macro cannot supply a conflicting initializer or a separate target answer. Implementation-internal unsafe operations have explicit validity, lifetime, aliasing, ownership, and cleanup obligations.

Lowering feeds the generated operations through ordinary Rust type, borrow, and MIR processing as appropriate. Built-in view and ownership operations preserve their specified contracts throughout. Inlining or dispatch optimization must preserve behavior; neither a table layout nor a performance gain is prescribed. The implementation must justify that its lowering satisfies the object contracts.

Compiler-produced cross-crate metadata carries nominal relationships, signatures, and reusable definitions needed by downstream classes under ordinary dependency/compiler compatibility rules. Re-exports retain identity. Visible classes and mixins may be extended or applied downstream when required items are accessible; privacy still applies. Changing a base, mixin order, or public operation contract is an API change that can affect downstream behavior. Diagnostics identify the responsible source relationship or body. Rustdoc and IDEs use the same representation to explain fields, interfaces, composition, and earlier targets.

### Compatibility and validation

Existing library `#[class]` expansions retain their implementation. Migration explicitly selects `#[core::oop::class]`, changes owner imports, and addresses the supported subset. The implementations have distinct types and no implicit ownership conversion; adapters use public operations to transfer application data or wrap an existing owner.

| Aspect | Compiler-supported contract and migration consequence |
| --- | --- |
| Object behavior | Retains per-object mixin state, final-host `on`, explicit overrides, and same-receiver `self`/`super` cooperation. |
| Ownership | Opaque `CRc`/`CWeak`; code relying on an `Rc` alias or owner recovery from `&self` must change. |
| Construction | Base, mixins, own fields; no access before publication. Other initialization orders or constructor-body hooks require restructuring. |
| Fields | The four contracts above apply; non-Copy mutable owner fields use guard-based access, with guards released before callbacks. |
| Methods and types | Shared receivers and non-generic, `'static` objects form the initial subset. Broader forms need subsequent design. |
| Destruction and threads | Field drop without virtual destruction; single-threaded handles and views. Cleanup hooks or threaded ownership require adaptation. |

A prototype would establish compilation cost, code size, runtime cost, and migration effort. Its first dispatch tests should run the guide's two command types through the common command view, assert that each returns the result of its own `execute()`, and check that the count is shared across views of one command and independent across instances. Further tests must cover ordered overrides, first implementations of abstract operations, invalid earlier targets, final-host requirements, ambiguous names, cross-crate aliases and privacy, and failed conversions preserving their source. Ownership tests must exercise weak upgrades during destruction, initializer failure at every field boundary, reentrant borrowing, leaked guards, cycles, and panicking field destructors. Negative tests must reject forged views, manual view impls, partial-object escape, bad signatures, and unsupported generic or receiver forms.

Generated code and runtime support require a safety argument covering valid references, initialization, ownership, projection, and destruction. Tests and interpreters such as Miri check executed cases and complement that argument. Existing oop_rs behavior provides comparison cases where the contracts agree. The prototype needs separate expectations for intentional migration differences.

## Drawbacks
[drawbacks]: #drawbacks

Rust would gain a substantial object facility alongside traits, composition, and macro libraries. Name lookup, composition, construction, conversion, dispatch, and compiler-controlled view implementations increase teaching and maintenance costs.

The compiler and allocation library would jointly maintain lowering and handle contracts, including safety obligations currently carried by a library. Centralizing generation makes the contract explicit but still requires correct implementation and review. Instantiated bodies, descriptors, and checking may increase build time and code size.

Shared reference counting and field policies constrain applications. Cycles can leak and borrow conflicts can panic. The subset requires changes to some existing oop_rs programs. Representative implementations and migrations must establish these costs and benefits.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

In the command library, a third-party extension such as `CountedRun` can name the operation it augments and declare the state each command must carry. The compiler checks the combination at the class that uses the extension. That check determines where the counter lives, which `run()` body the extension may call, and how the base body's `self.execute()` reaches the final implementation. Using the attribute-based form keeps these choices visible to readers familiar with oop_rs and makes migration explicit.

**Continue with the oop_rs macro and runtime.** oop_rs can express the counted commands today. The macro expands declarations into Rust types and methods, and Rust type-checks that expansion. The crate can improve its checks, diagnostics, and implementation on its own release schedule. The proposed built-in gives Rust responsibility for the composition rule itself, including the meaning of a downstream class that applies a mixin from another crate. That commitment would be worthwhile if enough authors rely on the same rule and benefit from compiler checks at the declaration that combines the layers.

**Check declaration metadata for tools.** A smaller channel can check relationships and improve navigation while preserving existing expansions. It does not make the checked model determine their actual storage, table initialization, or dispatch. It is an alternative when tooling is the main goal.

**Use traits and explicit composition.** A Rust trait can give `run()` a default body that performs preparation and calls `self.execute()`. Each command implements `execute()`. To add counting, the hosts can store a counter and expose it through a trait method, or they can pass a shared helper the state and the work to perform. For a small command library this keeps the control flow explicit and uses existing Rust.

A generic `Counted<C>` wrapper can instead own a counter and delegate `run()` to its inner `C`. For the two commands in the guide, this gives the same counts and return values. If a call from inside the inner command must see an override on the outer wrapper, forwarding `inner.run()` alone still uses the inner receiver. The wrapper can implement the hook or pass an explicit callback or receiver to the common implementation. The proposed composition rule specifies that receiver and the order of earlier bodies when several authors supply layers. A useful prototype comparison would add or reorder a layer on both command types and compare the forwarding code and diagnostics in each version.

**Choose different native syntax or a larger ownership model.** Dedicated keywords, generics, stack objects, mutable receivers, or threaded owners would cover additional programs while increasing semantic work. The attribute-based subset establishes one executable contract; extensions should preserve its identity and ownership rules or explicitly replace them.

**Special-case the existing crate in rustc.** This couples language behavior to private layouts and library versions. A specified built-in instead has an explicit compatibility boundary, with oop_rs remaining an experience source and a separately usable library.

## Prior art
[prior-art]: #prior-art

oop_rs provides the attribute-based declaration style, per-object mixin state, final-host requirements, explicit overrides, and earlier calls on the same receiver. The command example uses those relationships to explain the proposed compiler feature. The construction and owning-handle contracts described here are choices for the compiler-supported subset.

RustUI provides an example from an existing codebase. Its `RenderAnimatedOpacityMixin` supplies animation state and behavior to both box and sliver rendering objects. Each object keeps its own state and the mixin can continue the host's earlier behavior.

Rust's [procedural macros](https://doc.rust-lang.org/reference/procedural-macros.html), [trait rules](https://doc.rust-lang.org/reference/items/implementations.html), and [visibility](https://doc.rust-lang.org/reference/visibility-and-privacy.html) provide the existing context. [Rc](https://doc.rust-lang.org/std/rc/struct.Rc.html), [RefCell](https://doc.rust-lang.org/std/cell/struct.RefCell.html), and [destructor rules](https://doc.rust-lang.org/reference/destructors.html) inform ownership, borrowing, and cleanup. The [rustc lowering guide](https://rustc-dev-guide.rust-lang.org/hir/lowering.html) describes the compiler transformation setting for a prototype.

[Classes and Mixins](https://cs.brown.edu/people/sk/Publications/Papers/Published/fkf-classes-mixins/) studies reusable object behavior. [Delegation RFC 3530](https://github.com/rust-lang/rfcs/pull/3530) discusses forwarding and generated implementation wiring. Both offer comparison points for this combination.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

1. In which programming tasks do authors repeatedly combine per-object state and same-receiver overrides? How much does a shared compiler contract help them compared with oop_rs or explicit Rust composition?
2. Should the specified semantics use the proposed public names and module placement, or different names?
3. Which concrete representation and lowering best implements the specified contracts, and at what cost?
4. How much existing oop_rs code can migrate to the initial subset, and which excluded features should be designed next?
5. Which diagnostics and migration aids best explain construction order, owning handles, and accessor changes?

The initial execution rules above are the proposed contract. These questions concern adoption, implementation choices within it, and subsequent scope.

## Future possibilities
[future-possibilities]: #future-possibilities

Source-level generics and borrowed state need substitution, lifetime, and variance rules. Repeated mixins, richer interfaces, and explicit mixin-on requirements need corresponding identities and selection rules. `Arc`, unique mutable access, and pinned construction need their own aliasing, concurrency, and lifecycle contracts.

Experience with object lowering may inform more general typed declaration extensions. Such a generalization should preserve the executable relationship among declarations, storage, and method selection established here.
