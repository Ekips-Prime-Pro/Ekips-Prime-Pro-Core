//! Promises: results of things that take time.

use alloc::rc::Rc;
use alloc::vec::Vec;
use core::cell::RefCell;

use ekips_hub_api::Port;

use crate::value::{Closure, Value};

/// What a pending Promise is waiting for.
#[derive(Debug, Clone)]
pub enum Waiting {
    /// A task (a running `async` function); resolved when it returns
    Task,
    /// A motor movement started by `rotate`; done when the motor arrives or
    /// gets a new command (`generation` changed)
    Motor {
        /// The motor's port
        port: Port,
        /// Command counter of that motor when the movement started
        generation: u32,
    },
    /// A drive base movement started by `drive` or `turn`
    Drive {
        /// Command counter of the drive base when the movement started
        generation: u32,
    },
    /// A point in time (`wait`, `hub.beep`), in ms since start
    Until(u64),
    /// `waitUntil(condition)`: done when the function returns a truthy value
    Condition(Rc<Closure>),
    /// `Promise.all`: done when all are done; result is the list of results
    All(Vec<Rc<Promise>>),
    /// `Promise.race`: done when the first is done; result is its result
    Race(Vec<Rc<Promise>>),
}

#[derive(Debug)]
enum State {
    Pending(Waiting),
    Done(Value),
}

/// A Promise: pending until the thing it waits for is done.
#[derive(Debug)]
pub struct Promise {
    state: RefCell<State>,
}

impl Promise {
    /// A pending Promise.
    pub fn pending(waiting: Waiting) -> Rc<Promise> {
        Rc::new(Promise {
            state: RefCell::new(State::Pending(waiting)),
        })
    }

    /// A Promise that is already done.
    pub fn resolved(value: Value) -> Rc<Promise> {
        Rc::new(Promise {
            state: RefCell::new(State::Done(value)),
        })
    }

    /// The result, if done.
    pub fn result(&self) -> Option<Value> {
        match &*self.state.borrow() {
            State::Done(v) => Some(v.clone()),
            State::Pending(_) => None,
        }
    }

    /// What it waits for, if pending.
    pub fn waiting(&self) -> Option<Waiting> {
        match &*self.state.borrow() {
            State::Pending(w) => Some(w.clone()),
            State::Done(_) => None,
        }
    }

    /// Marks the Promise as done (does nothing if it already is).
    pub fn resolve(&self, value: Value) {
        let mut state = self.state.borrow_mut();
        if matches!(*state, State::Pending(_)) {
            *state = State::Done(value);
        }
    }
}
