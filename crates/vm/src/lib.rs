//! Virtual machine for Ekips Prime Pro bytecode.
//!
//! `no_std` (with `alloc`), so the same VM runs in the SPIKE Prime firmware
//! and in the simulator. It never touches hardware directly: every motor,
//! sensor and hub call goes through the [`ekips_hub_api::Hardware`] trait.
//!
//! `async`/`await` is implemented with a **cooperative scheduler**: every
//! running `async` function is a task, and [`Vm::tick`] runs the tasks one
//! after another until each has to wait. See `docs/en/vm.md`.
//!
//! ```
//! use ekips_vm::{MockHub, Status, Vm};
//!
//! let source = "async function main() { await wait(1s); console.log(\"done\"); }";
//! let bytes = ekips_compiler::compile(source).bytecode.unwrap();
//! let mut vm = Vm::load(&bytes).unwrap();
//! let mut hub = MockHub::new();
//! while vm.tick(&mut hub).unwrap() == Status::Running {
//!     hub.advance(10);
//! }
//! assert_eq!(hub.log_lines(), ["[  1.000s] console: done"]);
//! ```

#![no_std]

extern crate alloc;

mod error;
pub mod mock;
mod promise;
mod value;
mod vm;

pub use error::{ErrorKind, Lang, RuntimeError};
pub use mock::MockHub;
pub use promise::{Promise, Waiting};
pub use value::{Closure, Device, Value};
pub use vm::{DEFAULT_BUDGET, Status, Vm};
