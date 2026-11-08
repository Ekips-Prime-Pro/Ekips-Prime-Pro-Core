//! The interpreter and the cooperative scheduler.
//!
//! Every `async` function call runs in its own **task** with its own
//! stack. Tasks never run in parallel: [`Vm::tick`] lets each ready task
//! run until it has to wait (`await` on a pending Promise) or until its
//! share of the instruction budget is used up, then the next task runs.
//! Waiting tasks are woken when the Promise they wait for is done.

use alloc::rc::Rc;
use alloc::vec;
use alloc::vec::Vec;
use core::cell::RefCell;

use ekips_bytecode::{Capture, Constant, DecodeError, Dim, Native, Op, Program};
use ekips_hub_api::{Hardware, Port};

use crate::error::{ErrorKind, RuntimeError};
use crate::promise::{Promise, Waiting};
use crate::value::{Closure, Device, Value};

/// Instructions each task may run per tick (its time slice).
///
/// Every task gets its own slice, so a task in an endless loop can not
/// starve the others.
pub const DEFAULT_BUDGET: u32 = 1_000;
/// Instructions the condition of a `waitUntil` may take.
const CONDITION_BUDGET: u32 = 10_000;
/// Maximum number of nested function calls in one task.
const MAX_FRAMES: usize = 200;
/// Maximum number of `async` calls that start each other right away.
const MAX_SPAWN_DEPTH: u8 = 32;
/// Default speed for commands with an optional speed.
const DEFAULT_SPEED: f32 = 50.0;

/// State of the program after a tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Still running; call [`Vm::tick`] again
    Running,
    /// `main` has finished
    Finished,
}

#[derive(Debug)]
struct Frame {
    closure: Rc<Closure>,
    /// Offset of the next instruction, relative to the function start
    pc: usize,
    /// Stack index of local slot 0
    base: usize,
    /// The callee value sits below the locals (sync calls only)
    has_callee: bool,
}

#[derive(Debug)]
struct Task {
    stack: Vec<Value>,
    frames: Vec<Frame>,
    promise: Rc<Promise>,
    waiting: Option<Rc<Promise>>,
    line: u16,
}

enum Exit {
    Finished(Value),
    Suspended,
    OutOfBudget,
}

/// The virtual machine.
pub struct Vm {
    program: Program,
    constants: Vec<Value>,
    globals: Vec<Value>,
    tasks: Vec<Task>,
    root: Option<Rc<Promise>>,
    budget: u32,
    motor_generation: [u32; 6],
    drive_generation: u32,
    drive: Option<(Port, Port)>,
    timer_start: u64,
    rng: u32,
}

fn bad() -> ErrorKind {
    ErrorKind::BadProgram
}

impl Vm {
    /// Loads an encoded program.
    pub fn load(bytes: &[u8]) -> Result<Vm, DecodeError> {
        Ok(Vm::new(Program::decode(bytes)?))
    }

    /// Creates a VM for a decoded program.
    pub fn new(program: Program) -> Vm {
        let constants = program
            .constants
            .iter()
            .map(|c| match c {
                Constant::Num(v, d) => Value::Num(*v, *d),
                Constant::Str(s) => Value::Str(s.as_str().into()),
                Constant::Port(p) => Value::Port(*p),
                Constant::Color(c) => Value::Color(*c),
                Constant::MethodSet(_) => Value::Undefined,
            })
            .collect();
        Vm {
            globals: vec![Value::Undefined; usize::from(program.globals)],
            program,
            constants,
            tasks: Vec::new(),
            root: None,
            budget: DEFAULT_BUDGET,
            motor_generation: [0; 6],
            drive_generation: 0,
            drive: None,
            timer_start: 0,
            rng: 0x2545_F491,
        }
    }

    /// Sets the number of instructions each task may run per [`Vm::tick`].
    pub fn set_budget(&mut self, budget: u32) {
        self.budget = budget.max(1);
    }

    /// `true` once `main` has finished.
    pub fn is_finished(&self) -> bool {
        self.root.as_ref().is_some_and(|p| p.result().is_some())
    }

    /// Number of tasks that are still running or waiting.
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Runs the program a little further.
    ///
    /// Call this in a loop (on the hub: every millisecond or so) until it
    /// returns [`Status::Finished`] or an error.
    pub fn tick<H: Hardware>(&mut self, hw: &mut H) -> Result<Status, RuntimeError> {
        if self.root.is_none() {
            self.start(hw).map_err(|k| RuntimeError::new(k, 0))?;
        }
        if self.is_finished() {
            return Ok(Status::Finished);
        }
        let tasks = core::mem::take(&mut self.tasks);
        let mut remaining = Vec::with_capacity(tasks.len());
        let mut iter = tasks.into_iter();
        while let Some(mut task) = iter.next() {
            let mut budget = self.budget;
            let result = self.resume(&mut task, hw, &mut budget);
            match result {
                Ok(true) => remaining.push(task),
                Ok(false) => {}
                Err(e) => {
                    // Keep the other tasks so the state stays inspectable.
                    remaining.push(task);
                    remaining.extend(iter);
                    remaining.append(&mut self.tasks);
                    self.tasks = remaining;
                    return Err(e);
                }
            }
        }
        // Tasks started during this tick come after the existing ones.
        remaining.append(&mut self.tasks);
        self.tasks = remaining;
        Ok(if self.is_finished() {
            Status::Finished
        } else {
            Status::Running
        })
    }

    /// Creates the root task (function 0).
    fn start<H: Hardware>(&mut self, hw: &mut H) -> Result<(), ErrorKind> {
        self.timer_start = hw.now_ms();
        self.rng ^= hw.now_ms() as u32;
        let closure = Rc::new(Closure {
            function: 0,
            upvals: Vec::new(),
        });
        let task = self.new_task(closure, Vec::new())?;
        self.root = Some(task.promise.clone());
        self.tasks.push(task);
        Ok(())
    }

    fn new_task(&self, closure: Rc<Closure>, args: Vec<Value>) -> Result<Task, ErrorKind> {
        let mut stack = Vec::with_capacity(16);
        self.push_frame_locals(&closure, args, &mut stack)?;
        Ok(Task {
            stack,
            frames: vec![Frame {
                closure,
                pc: 0,
                base: 0,
                has_callee: false,
            }],
            promise: Promise::pending(Waiting::Task),
            waiting: None,
            line: 0,
        })
    }

    /// Pushes arguments (cut or padded to the parameter count) and the
    /// other local slots.
    fn push_frame_locals(
        &self,
        closure: &Closure,
        mut args: Vec<Value>,
        stack: &mut Vec<Value>,
    ) -> Result<(), ErrorKind> {
        let f = self
            .program
            .functions
            .get(usize::from(closure.function))
            .ok_or_else(bad)?;
        args.resize(usize::from(f.params), Value::Undefined);
        stack.extend(args);
        for _ in f.params..f.locals {
            stack.push(Value::Undefined);
        }
        Ok(())
    }

    /// Wakes and runs one task. Returns `false` when it has finished.
    fn resume<H: Hardware>(
        &mut self,
        task: &mut Task,
        hw: &mut H,
        budget: &mut u32,
    ) -> Result<bool, RuntimeError> {
        if let Some(p) = task.waiting.clone() {
            match self
                .poll(&p, hw, 0)
                .map_err(|k| RuntimeError::new(k, task.line))?
            {
                Some(v) => {
                    task.waiting = None;
                    task.stack.push(v);
                }
                None => return Ok(true),
            }
        }
        if *budget == 0 {
            return Ok(true);
        }
        match self.run(task, hw, budget, 0)? {
            Exit::Finished(v) => {
                task.promise.resolve(v);
                Ok(false)
            }
            Exit::Suspended | Exit::OutOfBudget => Ok(true),
        }
    }

    // ------------------------------------------------------------------
    // Promises
    // ------------------------------------------------------------------

    /// Checks whether a Promise is done; returns its result if so.
    fn poll<H: Hardware>(
        &mut self,
        promise: &Rc<Promise>,
        hw: &mut H,
        depth: u8,
    ) -> Result<Option<Value>, ErrorKind> {
        let Some(waiting) = promise.waiting() else {
            return Ok(promise.result());
        };
        let done = match waiting {
            Waiting::Task => None,
            Waiting::Motor { port, generation } => {
                (self.motor_generation[port.index()] != generation || hw.motor_done(port))
                    .then_some(Value::Undefined)
            }
            Waiting::Drive { generation } => {
                (self.drive_generation != generation || hw.drive_done()).then_some(Value::Undefined)
            }
            Waiting::Until(t) => (hw.now_ms() >= t).then_some(Value::Undefined),
            Waiting::Condition(closure) => {
                let v = self.call_sync(closure, hw, depth)?;
                v.truthy().then_some(Value::Undefined)
            }
            Waiting::All(list) => {
                let mut results = Vec::with_capacity(list.len());
                let mut all = true;
                for p in &list {
                    match self.poll(p, hw, depth)? {
                        Some(v) => results.push(v),
                        None => all = false,
                    }
                }
                all.then(|| Value::List(Rc::new(RefCell::new(results))))
            }
            Waiting::Race(list) => {
                let mut first = None;
                for p in &list {
                    if let Some(v) = self.poll(p, hw, depth)? {
                        first = Some(v);
                        break;
                    }
                }
                first
            }
        };
        if let Some(v) = &done {
            promise.resolve(v.clone());
        }
        Ok(done)
    }

    /// Runs a (non-async) function to the end, e.g. a `waitUntil` condition.
    fn call_sync<H: Hardware>(
        &mut self,
        closure: Rc<Closure>,
        hw: &mut H,
        depth: u8,
    ) -> Result<Value, ErrorKind> {
        let f = self
            .program
            .functions
            .get(usize::from(closure.function))
            .ok_or_else(bad)?;
        if f.is_async {
            return Err(ErrorKind::ConditionWaits);
        }
        let mut task = self.new_task(closure, Vec::new())?;
        let mut budget = CONDITION_BUDGET;
        match self.run(&mut task, hw, &mut budget, depth + 1) {
            Ok(Exit::Finished(v)) => Ok(v),
            Ok(Exit::Suspended) => Err(ErrorKind::ConditionWaits),
            Ok(Exit::OutOfBudget) => Err(ErrorKind::ConditionTooLong),
            Err(e) => Err(e.kind),
        }
    }

    // ------------------------------------------------------------------
    // Interpreter
    // ------------------------------------------------------------------

    fn code_byte(&self, frame: &Frame, offset: usize) -> Result<u8, ErrorKind> {
        let f = self
            .program
            .functions
            .get(usize::from(frame.closure.function))
            .ok_or_else(bad)?;
        if offset >= f.len as usize {
            return Err(bad());
        }
        self.program
            .code
            .get(f.offset as usize + offset)
            .copied()
            .ok_or_else(bad)
    }

    fn read_u8(&self, task: &mut Task) -> Result<u8, ErrorKind> {
        let frame = task.frames.last_mut().ok_or_else(bad)?;
        let b = self.code_byte(frame, frame.pc)?;
        frame.pc += 1;
        Ok(b)
    }

    fn read_u16(&self, task: &mut Task) -> Result<u16, ErrorKind> {
        let lo = self.read_u8(task)?;
        let hi = self.read_u8(task)?;
        Ok(u16::from_le_bytes([lo, hi]))
    }

    /// Runs a task until it finishes, waits or runs out of budget.
    fn run<H: Hardware>(
        &mut self,
        task: &mut Task,
        hw: &mut H,
        budget: &mut u32,
        depth: u8,
    ) -> Result<Exit, RuntimeError> {
        loop {
            if *budget == 0 {
                return Ok(Exit::OutOfBudget);
            }
            *budget -= 1;
            match self.step(task, hw, budget, depth) {
                Ok(None) => {}
                Ok(Some(exit)) => return Ok(exit),
                Err(Fail::Kind(kind)) => return Err(RuntimeError::new(kind, task.line)),
                Err(Fail::Error(e)) => return Err(e),
            }
        }
    }

    /// Executes one instruction.
    fn step<H: Hardware>(
        &mut self,
        task: &mut Task,
        hw: &mut H,
        budget: &mut u32,
        depth: u8,
    ) -> Result<Option<Exit>, Fail> {
        let op = Op::from_u8(self.read_u8(task)?).ok_or_else(bad)?;
        let base = task.frames.last().ok_or_else(bad)?.base;
        match op {
            Op::Nop => {}
            Op::Const => {
                let i = usize::from(self.read_u16(task)?);
                let v = self.constants.get(i).cloned().ok_or_else(bad)?;
                task.stack.push(v);
            }
            Op::Undefined => task.stack.push(Value::Undefined),
            Op::Null => task.stack.push(Value::Null),
            Op::True => task.stack.push(Value::Bool(true)),
            Op::False => task.stack.push(Value::Bool(false)),
            Op::Pop => {
                pop(task)?;
            }
            Op::Dup => {
                let v = task.stack.last().cloned().ok_or_else(bad)?;
                task.stack.push(v);
            }
            Op::Dup2 => {
                let n = task.stack.len();
                if n < 2 {
                    return Err(bad().into());
                }
                let (a, b) = (task.stack[n - 2].clone(), task.stack[n - 1].clone());
                task.stack.push(a);
                task.stack.push(b);
            }
            Op::LoadLocal => {
                let slot = base + usize::from(self.read_u8(task)?);
                let v = task.stack.get(slot).cloned().ok_or_else(bad)?;
                task.stack.push(v);
            }
            Op::StoreLocal => {
                let slot = base + usize::from(self.read_u8(task)?);
                let v = pop(task)?;
                *task.stack.get_mut(slot).ok_or_else(bad)? = v;
            }
            Op::LoadGlobal => {
                let i = usize::from(self.read_u16(task)?);
                let v = self.globals.get(i).cloned().ok_or_else(bad)?;
                task.stack.push(v);
            }
            Op::StoreGlobal => {
                let i = usize::from(self.read_u16(task)?);
                let v = pop(task)?;
                *self.globals.get_mut(i).ok_or_else(bad)? = v;
            }
            Op::NewCell => {
                let slot = base + usize::from(self.read_u8(task)?);
                let v = pop(task)?;
                *task.stack.get_mut(slot).ok_or_else(bad)? = Value::Cell(Rc::new(RefCell::new(v)));
            }
            Op::LoadCell => {
                let slot = base + usize::from(self.read_u8(task)?);
                let Some(Value::Cell(c)) = task.stack.get(slot) else {
                    return Err(bad().into());
                };
                let v = c.borrow().clone();
                task.stack.push(v);
            }
            Op::StoreCell => {
                let slot = base + usize::from(self.read_u8(task)?);
                let v = pop(task)?;
                let Some(Value::Cell(c)) = task.stack.get(slot) else {
                    return Err(bad().into());
                };
                *c.borrow_mut() = v;
            }
            Op::LoadUpval => {
                let i = usize::from(self.read_u8(task)?);
                let frame = task.frames.last().ok_or_else(bad)?;
                let v = frame
                    .closure
                    .upvals
                    .get(i)
                    .ok_or_else(bad)?
                    .borrow()
                    .clone();
                task.stack.push(v);
            }
            Op::StoreUpval => {
                let i = usize::from(self.read_u8(task)?);
                let v = pop(task)?;
                let frame = task.frames.last().ok_or_else(bad)?;
                *frame.closure.upvals.get(i).ok_or_else(bad)?.borrow_mut() = v;
            }
            Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Rem => {
                let b = pop(task)?;
                let a = pop(task)?;
                task.stack.push(arithmetic(op, &a, &b)?);
            }
            Op::Neg => match pop(task)? {
                Value::Num(v, d) => task.stack.push(Value::Num(-v, d)),
                _ => return Err(ErrorKind::BadOperands.into()),
            },
            Op::ToNumber => match pop(task)? {
                v @ Value::Num(..) => task.stack.push(v),
                Value::Bool(b) => task.stack.push(Value::num(if b { 1.0 } else { 0.0 })),
                _ => return Err(ErrorKind::BadOperands.into()),
            },
            Op::Not => {
                let v = pop(task)?;
                task.stack.push(Value::Bool(!v.truthy()));
            }
            Op::Eq | Op::NotEq => {
                let b = pop(task)?;
                let a = pop(task)?;
                let eq = a.strict_eq(&b);
                task.stack
                    .push(Value::Bool(if op == Op::Eq { eq } else { !eq }));
            }
            Op::Lt | Op::Le | Op::Gt | Op::Ge => {
                let b = pop(task)?;
                let a = pop(task)?;
                task.stack.push(Value::Bool(compare(op, &a, &b)?));
            }
            Op::Jump => {
                let target = usize::from(self.read_u16(task)?);
                task.frames.last_mut().ok_or_else(bad)?.pc = target;
            }
            Op::JumpIfFalse => {
                let target = usize::from(self.read_u16(task)?);
                if !pop(task)?.truthy() {
                    task.frames.last_mut().ok_or_else(bad)?.pc = target;
                }
            }
            Op::JumpIfTrueKeep | Op::JumpIfFalseKeep => {
                let target = usize::from(self.read_u16(task)?);
                let truthy = task.stack.last().ok_or_else(bad)?.truthy();
                if truthy == (op == Op::JumpIfTrueKeep) {
                    task.frames.last_mut().ok_or_else(bad)?.pc = target;
                } else {
                    pop(task)?;
                }
            }
            Op::Closure => {
                let index = self.read_u16(task)?;
                let f = self
                    .program
                    .functions
                    .get(usize::from(index))
                    .ok_or_else(bad)?;
                let frame = task.frames.last().ok_or_else(bad)?;
                let mut upvals = Vec::with_capacity(f.captures.len());
                for c in &f.captures {
                    let cell = match *c {
                        Capture::Local(slot) => match task.stack.get(base + usize::from(slot)) {
                            Some(Value::Cell(cell)) => cell.clone(),
                            _ => return Err(bad().into()),
                        },
                        Capture::Upval(i) => frame
                            .closure
                            .upvals
                            .get(usize::from(i))
                            .ok_or_else(bad)?
                            .clone(),
                    };
                    upvals.push(cell);
                }
                task.stack.push(Value::Closure(Rc::new(Closure {
                    function: index,
                    upvals,
                })));
            }
            Op::Call => {
                let argc = usize::from(self.read_u8(task)?);
                let callee_at = task.stack.len().checked_sub(argc + 1).ok_or_else(bad)?;
                let Value::Closure(closure) = task.stack[callee_at].clone() else {
                    return Err(ErrorKind::NotCallable.into());
                };
                let f = self
                    .program
                    .functions
                    .get(usize::from(closure.function))
                    .ok_or_else(bad)?;
                let args = task.stack.split_off(callee_at + 1);
                if f.is_async {
                    task.stack.pop(); // the callee
                    let promise = self.spawn(closure, args, hw, budget, depth)?;
                    task.stack.push(Value::Promise(promise));
                } else {
                    if task.frames.len() >= MAX_FRAMES {
                        return Err(ErrorKind::TooDeep.into());
                    }
                    let base = task.stack.len();
                    self.push_frame_locals(&closure, args, &mut task.stack)?;
                    task.frames.push(Frame {
                        closure,
                        pc: 0,
                        base,
                        has_callee: true,
                    });
                }
            }
            Op::CallNative => {
                let id = self.read_u16(task)?;
                let argc = usize::from(self.read_u8(task)?);
                let native = Native::from_u16(id).ok_or_else(bad)?;
                let at = task.stack.len().checked_sub(argc).ok_or_else(bad)?;
                let args = task.stack.split_off(at);
                let v = self.native(native, &args, hw)?;
                task.stack.push(v);
            }
            Op::CallMethod => {
                let set = usize::from(self.read_u16(task)?);
                let argc = usize::from(self.read_u8(task)?);
                let Some(Constant::MethodSet(natives)) = self.program.constants.get(set) else {
                    return Err(bad().into());
                };
                let at = task.stack.len().checked_sub(argc + 1).ok_or_else(bad)?;
                let receiver = receiver_kind(&task.stack[at]);
                let native = natives
                    .iter()
                    .copied()
                    .find(|n| n.receiver().is_some() && n.receiver() == receiver)
                    .ok_or(ErrorKind::NoSuchMember)?;
                let args = task.stack.split_off(at);
                let v = self.native(native, &args, hw)?;
                task.stack.push(v);
            }
            Op::Return => {
                let v = pop(task)?;
                let frame = task.frames.pop().ok_or_else(bad)?;
                task.stack.truncate(frame.base);
                if frame.has_callee {
                    task.stack.pop();
                }
                if task.frames.is_empty() {
                    return Ok(Some(Exit::Finished(v)));
                }
                task.stack.push(v);
            }
            Op::MakeList => {
                let n = usize::from(self.read_u16(task)?);
                let at = task.stack.len().checked_sub(n).ok_or_else(bad)?;
                let items = task.stack.split_off(at);
                task.stack.push(Value::List(Rc::new(RefCell::new(items))));
            }
            Op::GetIndex => {
                let index = pop(task)?;
                let object = pop(task)?;
                task.stack.push(get_index(&object, &index)?);
            }
            Op::SetIndex => {
                let value = pop(task)?;
                let index = pop(task)?;
                let object = pop(task)?;
                set_index(&object, &index, value.clone())?;
                task.stack.push(value);
            }
            Op::Await => {
                let v = pop(task)?;
                let Value::Promise(p) = v else {
                    task.stack.push(v);
                    return Ok(None);
                };
                match self.poll(&p, hw, depth)? {
                    Some(result) => task.stack.push(result),
                    None => {
                        task.waiting = Some(p);
                        return Ok(Some(Exit::Suspended));
                    }
                }
            }
            Op::Line => task.line = self.read_u16(task)?,
        }
        Ok(None)
    }

    /// Starts an `async` function in a new task and runs it right away
    /// until it has to wait (like JavaScript).
    fn spawn<H: Hardware>(
        &mut self,
        closure: Rc<Closure>,
        args: Vec<Value>,
        hw: &mut H,
        budget: &mut u32,
        depth: u8,
    ) -> Result<Rc<Promise>, Fail> {
        if depth >= MAX_SPAWN_DEPTH {
            return Err(ErrorKind::TooDeep.into());
        }
        let mut task = self.new_task(closure, args)?;
        let promise = task.promise.clone();
        // Give the new task some budget even if the caller used up its share.
        let mut own = (*budget).max(1);
        let exit = self.run(&mut task, hw, &mut own, depth + 1)?;
        *budget = (*budget).min(own);
        match exit {
            Exit::Finished(v) => promise.resolve(v),
            Exit::Suspended | Exit::OutOfBudget => self.tasks.push(task),
        }
        Ok(promise)
    }

    // ------------------------------------------------------------------
    // Built-in functions
    // ------------------------------------------------------------------

    fn native<H: Hardware>(
        &mut self,
        native: Native,
        args: &[Value],
        hw: &mut H,
    ) -> Result<Value, ErrorKind> {
        use Native as N;
        use ekips_hub_api::DeviceKind as K;
        let hw_err = ErrorKind::Hardware;
        let v = match native {
            N::MotorNew => {
                let port = port_arg(args, 0)?;
                hw.check_device(port, K::Motor).map_err(hw_err)?;
                Value::Device(Device::Motor(port))
            }
            N::ColorSensorNew => {
                let port = port_arg(args, 0)?;
                hw.check_device(port, K::ColorSensor).map_err(hw_err)?;
                Value::Device(Device::ColorSensor(port))
            }
            N::DistanceSensorNew => {
                let port = port_arg(args, 0)?;
                hw.check_device(port, K::DistanceSensor).map_err(hw_err)?;
                Value::Device(Device::DistanceSensor(port))
            }
            N::ForceSensorNew => {
                let port = port_arg(args, 0)?;
                hw.check_device(port, K::ForceSensor).map_err(hw_err)?;
                Value::Device(Device::ForceSensor(port))
            }
            N::DriveBaseNew => {
                let left = motor_arg(args, 0)?;
                let right = motor_arg(args, 1)?;
                let config = ekips_hub_api::DriveBaseConfig {
                    left,
                    right,
                    wheel_diameter_mm: num_arg(args, 2)?.ok_or(ErrorKind::BadArgument)?,
                    axle_track_mm: num_arg(args, 3)?.ok_or(ErrorKind::BadArgument)?,
                };
                hw.drive_configure(config).map_err(hw_err)?;
                self.drive = Some((left, right));
                Value::Device(Device::DriveBase)
            }

            N::MotorRotate => {
                let port = motor_arg(args, 0)?;
                let angle = num_arg(args, 1)?.ok_or(ErrorKind::BadArgument)?;
                let speed = num_arg(args, 2)?.unwrap_or(DEFAULT_SPEED);
                hw.motor_run_angle(port, angle, speed).map_err(hw_err)?;
                let generation = self.next_motor_generation(port);
                Value::Promise(Promise::pending(Waiting::Motor { port, generation }))
            }
            N::MotorRun => {
                let port = motor_arg(args, 0)?;
                let speed = num_arg(args, 1)?.ok_or(ErrorKind::BadArgument)?;
                hw.motor_run(port, speed).map_err(hw_err)?;
                self.next_motor_generation(port);
                Value::Undefined
            }
            N::MotorStop => {
                let port = motor_arg(args, 0)?;
                hw.motor_stop(port).map_err(hw_err)?;
                self.next_motor_generation(port);
                Value::Undefined
            }
            N::MotorAngle => Value::Num(
                hw.motor_angle(motor_arg(args, 0)?).map_err(hw_err)?,
                Dim::Angle,
            ),
            N::MotorSpeed => Value::Num(
                hw.motor_speed(motor_arg(args, 0)?).map_err(hw_err)?,
                Dim::Percent,
            ),
            N::MotorResetAngle => {
                hw.motor_reset_angle(motor_arg(args, 0)?).map_err(hw_err)?;
                Value::Undefined
            }

            N::ColorSensorColor => Value::Color(
                hw.color(sensor_arg(args, 0, Receiver::Color)?)
                    .map_err(hw_err)?,
            ),
            N::ColorSensorSees => {
                let port = sensor_arg(args, 0, Receiver::Color)?;
                let Some(Value::Color(wanted)) = args.get(1) else {
                    return Err(ErrorKind::BadArgument);
                };
                Value::Bool(hw.color(port).map_err(hw_err)? == *wanted)
            }
            N::ColorSensorReflection => Value::Num(
                hw.reflection(sensor_arg(args, 0, Receiver::Color)?)
                    .map_err(hw_err)?,
                Dim::Percent,
            ),
            N::DistanceSensorDistance => {
                let d = hw
                    .distance_mm(sensor_arg(args, 0, Receiver::Distance)?)
                    .map_err(hw_err)?;
                Value::Num(d.unwrap_or(2000.0), Dim::Length)
            }
            N::ForceSensorPressed => Value::Bool(
                hw.pressed(sensor_arg(args, 0, Receiver::Force)?)
                    .map_err(hw_err)?,
            ),
            N::ForceSensorForce => Value::num(
                hw.force_n(sensor_arg(args, 0, Receiver::Force)?)
                    .map_err(hw_err)?,
            ),

            N::DriveBaseDrive | N::DriveBaseTurn => {
                drive_base_arg(args)?;
                let amount = num_arg(args, 1)?.ok_or(ErrorKind::BadArgument)?;
                let speed = num_arg(args, 2)?.unwrap_or(DEFAULT_SPEED);
                if native == N::DriveBaseDrive {
                    hw.drive_straight(amount, speed).map_err(hw_err)?;
                } else {
                    hw.drive_turn(amount, speed).map_err(hw_err)?;
                }
                self.drive_generation = self.drive_generation.wrapping_add(1);
                Value::Promise(Promise::pending(Waiting::Drive {
                    generation: self.drive_generation,
                }))
            }
            N::DriveBaseRun => {
                drive_base_arg(args)?;
                let speed = num_arg(args, 1)?.ok_or(ErrorKind::BadArgument)?;
                let (left, right) = self.drive.ok_or(ErrorKind::Hardware(
                    ekips_hub_api::HardwareError::NoDriveBase,
                ))?;
                hw.motor_run(left, speed).map_err(hw_err)?;
                hw.motor_run(right, speed).map_err(hw_err)?;
                self.drive_generation = self.drive_generation.wrapping_add(1);
                Value::Undefined
            }
            N::DriveBaseStop => {
                drive_base_arg(args)?;
                hw.drive_stop().map_err(hw_err)?;
                if let Some((left, right)) = self.drive {
                    hw.motor_stop(left).map_err(hw_err)?;
                    hw.motor_stop(right).map_err(hw_err)?;
                }
                self.drive_generation = self.drive_generation.wrapping_add(1);
                Value::Undefined
            }

            N::HubShow => {
                let text = args.first().map(Value::to_text).unwrap_or_default();
                hw.show(&text);
                Value::Undefined
            }
            N::HubBeep => {
                let frequency = num_arg(args, 0)?.unwrap_or(500.0);
                let duration = num_arg(args, 1)?.unwrap_or(200.0).max(0.0);
                hw.beep(
                    frequency.clamp(0.0, 20_000.0) as u16,
                    duration.min(60_000.0) as u16,
                );
                Value::Promise(Promise::pending(Waiting::Until(
                    hw.now_ms() + duration as u64,
                )))
            }
            N::HubHeading => Value::Num(hw.heading(), Dim::Angle),
            N::HubResetHeading => {
                hw.reset_heading();
                Value::Undefined
            }
            N::HubTimer => Value::Num(
                hw.now_ms().saturating_sub(self.timer_start) as f32,
                Dim::Time,
            ),
            N::HubResetTimer => {
                self.timer_start = hw.now_ms();
                Value::Undefined
            }
            N::HubBattery => Value::Num(hw.battery_pct(), Dim::Percent),

            N::MathAbs | N::MathRound | N::MathFloor | N::MathCeil | N::MathSqrt => {
                let Some(Value::Num(x, dim)) = args.first() else {
                    return Err(ErrorKind::BadArgument);
                };
                let r = match native {
                    N::MathAbs => libm::fabsf(*x),
                    N::MathRound => libm::roundf(*x),
                    N::MathFloor => libm::floorf(*x),
                    N::MathCeil => libm::ceilf(*x),
                    _ => libm::sqrtf(*x),
                };
                Value::Num(r, *dim)
            }
            N::MathMin | N::MathMax => {
                let mut best: Option<(f32, Dim)> = None;
                for a in args {
                    let Value::Num(x, d) = a else {
                        return Err(ErrorKind::BadArgument);
                    };
                    best = Some(match best {
                        None => (*x, *d),
                        Some((b, bd)) => {
                            let better = if native == N::MathMin { *x < b } else { *x > b };
                            if better { (*x, bd) } else { (b, bd) }
                        }
                    });
                }
                let (x, d) = best.ok_or(ErrorKind::BadArgument)?;
                Value::Num(x, d)
            }
            N::MathRandom => {
                // xorshift32
                let mut x = self.rng;
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                self.rng = x;
                Value::num((x >> 8) as f32 / (1u32 << 24) as f32)
            }

            N::PromiseAll | N::PromiseRace => {
                let Some(Value::List(items)) = args.first() else {
                    return Err(ErrorKind::BadArgument);
                };
                let promises: Vec<Rc<Promise>> = items
                    .borrow()
                    .iter()
                    .map(|v| match v {
                        Value::Promise(p) => p.clone(),
                        other => Promise::resolved(other.clone()),
                    })
                    .collect();
                let waiting = if native == N::PromiseAll {
                    Waiting::All(promises)
                } else {
                    Waiting::Race(promises)
                };
                Value::Promise(Promise::pending(waiting))
            }
            N::Wait => {
                let ms = num_arg(args, 0)?.ok_or(ErrorKind::BadArgument)?.max(0.0);
                Value::Promise(Promise::pending(Waiting::Until(hw.now_ms() + ms as u64)))
            }
            N::WaitUntil => {
                let Some(Value::Closure(c)) = args.first() else {
                    return Err(ErrorKind::BadArgument);
                };
                Value::Promise(Promise::pending(Waiting::Condition(c.clone())))
            }
            N::ConsoleLog => {
                let mut line = alloc::string::String::new();
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        line.push(' ');
                    }
                    line.push_str(&a.to_text());
                }
                hw.print(&line);
                Value::Undefined
            }
            N::Length => match args.first() {
                Some(Value::List(items)) => Value::num(items.borrow().len() as f32),
                Some(Value::Str(s)) => Value::num(s.chars().count() as f32),
                _ => return Err(ErrorKind::NoSuchMember),
            },
        };
        Ok(v)
    }

    fn next_motor_generation(&mut self, port: Port) -> u32 {
        let g = &mut self.motor_generation[port.index()];
        *g = g.wrapping_add(1);
        *g
    }
}

/// An error inside [`Vm::step`]: either a plain kind (the line is added by
/// the caller) or a complete error from a nested task.
enum Fail {
    Kind(ErrorKind),
    Error(RuntimeError),
}

impl From<ErrorKind> for Fail {
    fn from(k: ErrorKind) -> Self {
        Fail::Kind(k)
    }
}

impl From<RuntimeError> for Fail {
    fn from(e: RuntimeError) -> Self {
        Fail::Error(e)
    }
}

fn pop(task: &mut Task) -> Result<Value, ErrorKind> {
    task.stack.pop().ok_or_else(bad)
}

/// Kind of number of a result: the kind that is not a plain number wins.
fn result_dim(a: Dim, b: Dim) -> Dim {
    if a == Dim::Number { b } else { a }
}

fn arithmetic(op: Op, a: &Value, b: &Value) -> Result<Value, ErrorKind> {
    if op == Op::Add && (matches!(a, Value::Str(_)) || matches!(b, Value::Str(_))) {
        let mut s = a.to_text();
        s.push_str(&b.to_text());
        return Ok(Value::Str(s.as_str().into()));
    }
    let (Value::Num(x, da), Value::Num(y, db)) = (a, b) else {
        return Err(ErrorKind::BadOperands);
    };
    let (x, y, da, db) = (*x, *y, *da, *db);
    Ok(match op {
        Op::Add => Value::Num(x + y, result_dim(da, db)),
        Op::Sub => Value::Num(x - y, result_dim(da, db)),
        Op::Mul => Value::Num(x * y, result_dim(da, db)),
        Op::Div => {
            let dim = if db == Dim::Number {
                da
            } else if da == db {
                Dim::Number
            } else {
                da
            };
            Value::Num(x / y, dim)
        }
        _ => Value::Num(libm::fmodf(x, y), da),
    })
}

fn compare(op: Op, a: &Value, b: &Value) -> Result<bool, ErrorKind> {
    let ord = match (a, b) {
        (Value::Num(x, _), Value::Num(y, _)) => x.partial_cmp(y),
        (Value::Str(x), Value::Str(y)) => Some(x.cmp(y)),
        _ => return Err(ErrorKind::BadOperands),
    };
    let Some(ord) = ord else { return Ok(false) };
    Ok(match op {
        Op::Lt => ord.is_lt(),
        Op::Le => ord.is_le(),
        Op::Gt => ord.is_gt(),
        _ => ord.is_ge(),
    })
}

fn index_of(index: &Value) -> Result<Option<usize>, ErrorKind> {
    match index {
        Value::Num(i, _) if *i >= 0.0 && libm::floorf(*i) == *i => Ok(Some(*i as usize)),
        Value::Num(..) => Ok(None),
        _ => Err(ErrorKind::BadOperands),
    }
}

fn get_index(object: &Value, index: &Value) -> Result<Value, ErrorKind> {
    let i = index_of(index)?;
    match object {
        Value::List(items) => Ok(i
            .and_then(|i| items.borrow().get(i).cloned())
            .unwrap_or(Value::Undefined)),
        Value::Str(s) => Ok(i
            .and_then(|i| s.chars().nth(i))
            .map(|c| {
                let mut buf = [0u8; 4];
                Value::Str((&*c.encode_utf8(&mut buf)).into())
            })
            .unwrap_or(Value::Undefined)),
        _ => Err(ErrorKind::NotAList),
    }
}

fn set_index(object: &Value, index: &Value, value: Value) -> Result<(), ErrorKind> {
    let Value::List(items) = object else {
        return Err(ErrorKind::NotAList);
    };
    let i = index_of(index)?.ok_or(ErrorKind::IndexOutOfRange)?;
    let mut items = items.borrow_mut();
    if i < items.len() {
        items[i] = value;
    } else if i == items.len() {
        items.push(value);
    } else {
        return Err(ErrorKind::IndexOutOfRange);
    }
    Ok(())
}

fn receiver_kind(v: &Value) -> Option<ekips_bytecode::Receiver> {
    use ekips_bytecode::Receiver as R;
    Some(match v {
        Value::Device(Device::Motor(_)) => R::Motor,
        Value::Device(Device::ColorSensor(_)) => R::ColorSensor,
        Value::Device(Device::DistanceSensor(_)) => R::DistanceSensor,
        Value::Device(Device::ForceSensor(_)) => R::ForceSensor,
        Value::Device(Device::DriveBase) => R::DriveBase,
        Value::List(_) | Value::Str(_) => R::ListOrText,
        _ => return None,
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Receiver {
    Color,
    Distance,
    Force,
}

/// A number argument; `None` if missing or `undefined`.
fn num_arg(args: &[Value], i: usize) -> Result<Option<f32>, ErrorKind> {
    match args.get(i) {
        None | Some(Value::Undefined) => Ok(None),
        Some(Value::Num(v, _)) => Ok(Some(*v)),
        Some(_) => Err(ErrorKind::BadArgument),
    }
}

fn port_arg(args: &[Value], i: usize) -> Result<Port, ErrorKind> {
    match args.get(i) {
        Some(Value::Port(p)) => Ok(*p),
        _ => Err(ErrorKind::BadArgument),
    }
}

fn motor_arg(args: &[Value], i: usize) -> Result<Port, ErrorKind> {
    match args.get(i) {
        Some(Value::Device(Device::Motor(p))) => Ok(*p),
        _ => Err(ErrorKind::BadArgument),
    }
}

fn sensor_arg(args: &[Value], i: usize, kind: Receiver) -> Result<Port, ErrorKind> {
    match (args.get(i), kind) {
        (Some(Value::Device(Device::ColorSensor(p))), Receiver::Color)
        | (Some(Value::Device(Device::DistanceSensor(p))), Receiver::Distance)
        | (Some(Value::Device(Device::ForceSensor(p))), Receiver::Force) => Ok(*p),
        _ => Err(ErrorKind::BadArgument),
    }
}

fn drive_base_arg(args: &[Value]) -> Result<(), ErrorKind> {
    match args.first() {
        Some(Value::Device(Device::DriveBase)) => Ok(()),
        _ => Err(ErrorKind::BadArgument),
    }
}
