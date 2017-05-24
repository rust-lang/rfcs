- Start Date: 2014-04-14
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Make a way to dereference *T in a simpler way.

# Motivation

In most of the cases, embedded hardware is configured with memory-mapped registers. Often a peripheral would have a moderately big number of registers to the point where tracking each one by its address is hard and prone to errors.

It's common to have structs mapping a given peripheral's ioregs in embedded C world. Unfortunately with rust using such structs require "junk code".

# Detailed design

Consider the following example:

```rust
// this struct maps over a number of ioregs
pub struct UART {
  REG_A:  u32,
  REG_B:  u32,
  _pad_0: u32,
  REG_C:  u32,
}

impl UART {
  // given the nature of regs, we must use volatile access
  pub fn get_REG_A(&self) {
    unsafe { volatile_load(&(self.REG_A)) }
  }

  pub fn set_REG_A(&self, val: u32) {
    unsafe { volatile_store(&mut (self.REG_A), val) }
  }
  ...
}

// instantiate all peripherals over corresponding ioregs
pub static UART0 : *mut UART = 0xdeadbeef as *mut UART;
...
```

Rust requires an "unsafe" block and explicit dereferencing to access the registers:

```rust
let val: u32 = unsafe { (*UART0).REG_A() };
```

Instead of much simpler to read and understand syntax:

```rust
let val: u32 = UART0.REG_A();
```

# Alternatives

It is possible to make a wrapping struct:

```rust
struct UARTWrapper {
  uart: *mut UART,
}

// move all methods from UART to UARTWrapper

static UART0Regs : *mut UART = 0xdeadbeef as *mut UART;
pub static UART0 : UARTWrapper = UARTWrapper { uart: UART0Regs };
```

In this case, accessing methods on `UART0` is done with simple syntax, but the actual
`static UART0` gets into output binary (as u32 0xdeadbeef), which requres developer to make a trade between syntax and binary size.

# Unresolved questions

How to actually simplify the syntax?
