# Ekips Prime Pro Language Reference (v0.1, draft)

Ekips Prime Pro is a small programming language for LEGO® SPIKE™ Prime robots.
Its syntax is a subset of **TypeScript without type annotations**, plus
**units on numbers** (`20cm`, `90deg`, `2s`, `50%`). It is designed for
students in grades 7–10 and for AI assistants that help them.

This file is the single source of truth for the language. It is also meant
to be given to an AI assistant as context.

> **Implementation status:** the parser implements all syntax on this page.
> Semantic analysis checks names, scopes and the use of the standard
> library (see [compiler.md](compiler.md), full list of built-ins in
> [stdlib.md](stdlib.md)) and units. Code generation and the VM are not
> written yet.

## 1. A first program

```ts
// Setup: what is plugged in where
const leftWheel = new Motor(Port.A);
const rightWheel = new Motor(Port.B);
const robot = new DriveBase(leftWheel, rightWheel, 56mm, 112mm);

async function square(side) {
  for (let i = 0; i < 4; i++) {
    await robot.drive(side);
    await robot.turn(90deg);
  }
}

// The program starts here
async function main() {
  await square(20cm);
  await hub.beep();
}
```

## 2. Basics

- Files end in `.ekips`.
- The program starts in **`async function main()`**. Every program needs it.
- Code outside of functions runs once before `main`. Use it for setup
  (`const arm = new Motor(Port.C);`) and constants.
- Comments: `// until the end of the line` and `/* across lines */`.
- Semicolons are optional at the end of a line. Two statements on the same
  line need a `;` between them.
- Names are case-sensitive (`speed` and `Speed` are different).
- There are **no types**. Do not write `: number` or `: void`; the compiler
  explains this if you do.

## 3. Units

A unit is written directly after a number, without a space.

| Kind | Units | Example |
|---|---|---|
| Length | `mm`, `cm` | `20cm`, `150mm` |
| Angle | `deg` (or `°`), `rot` (full rotations) | `90deg`, `90°`, `1.5rot` |
| Time | `s`, `ms` | `2s`, `300ms` |
| Speed / power | `%` | `50%` |

Values keep their unit when you calculate: `20cm + 5mm` is `20.5cm`,
`2 * 90deg` is `180deg`. Mixing kinds (`20cm + 3s`) is an error, and so is
passing the wrong kind to a command (`robot.drive(90deg)`). A plain `0`
fits every kind (`hub.heading > 0`). All rules: [compiler.md](compiler.md#unit-checking).

`%` after a number is a percentage. Between two values it is the remainder
operator: `10 % 3` is `1`.

## 4. Standard library (planned API)

### Setup

```ts
const arm    = new Motor(Port.C);
const eye    = new ColorSensor(Port.D);
const sonar  = new DistanceSensor(Port.E);
const bumper = new ForceSensor(Port.F);
const robot  = new DriveBase(leftMotor, rightMotor, wheelDiameter, axleTrack);
```

`Port.A` … `Port.F` are the six ports of the hub. `wheelDiameter` is the
diameter of the wheels, `axleTrack` the distance between the two wheels
(both lengths, e.g. `56mm` and `112mm`).

The gyro sensor is built into the hub (`hub.heading`).

### Commands that take time return a Promise

Commands that move something take time. They return a **Promise**: the
movement starts immediately, and `await` waits until it is finished.

```ts
await robot.drive(20cm);          // drive, then continue
const lifting = arm.rotate(90deg); // start lifting, continue immediately
await robot.drive(20cm);          // drive while the arm is lifting
await lifting;                    // make sure the arm is done
```

> **Forgetting `await`** is the most common mistake: the robot then starts
> the next command while the previous one is still running. The compiler
> will warn about commands whose Promise is neither awaited nor stored.
> *(planned)*

### DriveBase

| Member | Description |
|---|---|
| `drive(distance, speed?)` | Drive straight. Negative = backwards. Returns a Promise. |
| `turn(angle, speed?)` | Turn on the spot. Positive = right (clockwise). Returns a Promise. |
| `run(speed)` | Start driving straight until `stop()`. |
| `stop()` | Stop both wheels. |

### Motor

| Member | Description |
|---|---|
| `rotate(angle, speed?)` | Rotate by an angle (`deg` or `rot`). Returns a Promise. |
| `run(speed)` | Start rotating until `stop()`. Negative = other direction. |
| `stop()` | Stop and hold the position. |
| `angle` | Angle since the last `resetAngle()` (in `deg`). |
| `speed` | Current speed (in `%`). |
| `resetAngle()` | Set `angle` to `0deg`. |

### Sensors

| Member | Description |
|---|---|
| `ColorSensor.color` | Color seen right now, e.g. `Color.Red` |
| `ColorSensor.sees(color)` | `true` if it sees that color |
| `ColorSensor.reflection` | Reflected light in `%` |
| `DistanceSensor.distance` | Distance as a length; `200cm` if nothing is in range |
| `ForceSensor.pressed` | `true` while pressed |
| `ForceSensor.force` | Force in newtons (plain number) |

Colors: `Color.None`, `Color.Black`, `Color.Violet`, `Color.Blue`,
`Color.Azure`, `Color.Green`, `Color.Yellow`, `Color.Red`, `Color.White`.

### Hub

| Member | Description |
|---|---|
| `hub.show(value)` | Show a number or short text on the light matrix |
| `hub.beep(frequency?, duration?)` | Beep. Returns a Promise. |
| `hub.heading` | Gyro heading since `resetHeading()` (in `deg`, right = positive) |
| `hub.resetHeading()` | Set the heading to `0deg` |
| `hub.timer` | Time since start or `resetTimer()` (in `s`) |
| `hub.resetTimer()` | Set the timer to `0s` |
| `hub.battery` | Battery charge in `%` |

### Global functions

| Function | Description |
|---|---|
| `wait(time)` | Promise that finishes after `time` |
| `waitUntil(() => condition)` | Promise that finishes as soon as the condition is `true` |
| `Promise.all([a, b, …])` | Finishes when **all** given Promises are done |
| `Promise.race([a, b, …])` | Finishes when the **first** one is done |
| `console.log(a, b, …)` | Print to the computer (USB console / simulator) |
| `Math.abs`, `Math.min`, `Math.max`, `Math.round`, `Math.floor`, `Math.ceil`, `Math.sqrt`, `Math.random`, `Math.PI` | Math |

### Doing things at the same time

```ts
// Both at once, continue when both are finished
await Promise.all([robot.drive(80cm), arm.rotate(180deg)]);

// Drive until a black line, but at most 1 m
await Promise.race([
  robot.drive(100cm),
  waitUntil(() => eye.sees(Color.Black)),
]);
robot.stop();

// Your own async functions run in parallel the same way
async function wave() {
  await arm.rotate(90deg);
  await arm.rotate(-90deg);
}
await Promise.all([robot.drive(30cm), wave()]);
```

## 5. Statements

```ts
const SPEED = 60%;          // can not be changed later
let count = 0;              // can be changed later
count = count + 1;
count += 1;
count++;

if (eye.sees(Color.Red)) {
  hub.show("R");
} else if (eye.sees(Color.Green)) {
  hub.show("G");
} else {
  hub.show("?");
}

while (sonar.distance > 10cm) {
  await robot.drive(5cm);
}

for (let i = 0; i < 4; i++) {
  await robot.turn(90deg);
}

for (const mission of [mission1, mission2]) {
  await mission();
}

break;     // leave the loop
continue;  // next round of the loop
return x;  // leave the function (with a value)
```

## 6. Functions

```ts
function double(x) {
  return x * 2;
}

async function square(side) {   // async: may use await inside
  for (let i = 0; i < 4; i++) {
    await robot.drive(side);
    await robot.turn(90deg);
  }
}

// Arrow functions
const isDark = () => eye.reflection < 20%;
const half = x => x / 2;
const go = async () => { await robot.drive(10cm); };
```

`await` only works inside `async` functions (and `async` arrow functions).

## 7. Values and operators

| Kind | Examples |
|---|---|
| Number | `42`, `2.5`, `20cm`, `90deg`, `50%` |
| Text | `"hello"`, `'hello'` (escapes: `\n`, `\t`, `\"`, `\'`, `\\`) |
| Yes/no | `true`, `false` |
| Nothing | `null`, `undefined` |
| List | `[1, 2, 3]`, `list[0]`, `list.length` |

| Operators (weakest binding first) | |
|---|---|
| `=` `+=` `-=` `*=` `/=` `%=` | Assignment |
| `? :` | Conditional value: `x > 0 ? x : -x` |
| `\|\|` | or |
| `&&` | and |
| `==` `===` `!=` `!==` | Equal / not equal (`==` and `===` mean the same) |
| `<` `<=` `>` `>=` | Comparison |
| `+` `-` | Add, subtract (`+` also joins texts) |
| `*` `/` `%` | Multiply, divide, remainder |
| `!` `-` `+` `await` `++` `--` | Prefix |
| `++` `--` | Postfix |
| `.` `[]` `()` `new` | Member, index, call, create |

## 8. Differences from TypeScript

- No type annotations, interfaces, `type`, generics or `enum` declarations.
- No classes, object literals (`{ a: 1 }`), `import`/`export`,
  template strings, `var`, `switch`, `try`/`catch` (yet).
- `==` and `===` both compare strictly; there is no type coercion.
- A `(` or `[` at the **start of a line** always begins a new statement.
  (In JavaScript, `let a = b` followed by `(c)` on the next line would call `b(c)`.)
- Units on numbers (`20cm`) are not TypeScript.
- Every program needs `async function main()`.

## 9. Reserved words

```
async await break const continue else false for function if let new
null of return true undefined while
```

Keywords can still be used as property names (`Promise.race`, `x.new`).

## 10. Grammar (EBNF)

```ebnf
program      = { statement } ;

statement    = block | ";" | var_decl | function | if | while | for
             | "return" [ expr ] end | "break" end | "continue" end
             | expr end ;
end          = ";" | line break | (before "}") | (end of file) ;
block        = "{" { statement } "}" ;
var_decl     = ( "const" | "let" ) name [ "=" expr ] end ;
function     = [ "async" ] "function" name params block ;
params       = "(" [ name { "," name } [ "," ] ] ")" ;
if           = "if" "(" expr ")" statement [ "else" statement ] ;
while        = "while" "(" expr ")" statement ;
for          = "for" "(" [ ( "const" | "let" ) name "=" expr | expr ] ";"
                         [ expr ] ";" [ expr ] ")" statement
             | "for" "(" ( "const" | "let" ) name "of" expr ")" statement ;

expr         = arrow | assignment ;
arrow        = [ "async" ] ( name | params ) "=>" ( block | expr ) ;
assignment   = conditional [ ( "=" | "+=" | "-=" | "*=" | "/=" | "%=" ) expr ] ;
conditional  = or [ "?" expr ":" expr ] ;
or           = and { "||" and } ;
and          = equality { "&&" equality } ;
equality     = relational { ( "==" | "===" | "!=" | "!==" ) relational } ;
relational   = additive { ( "<" | "<=" | ">" | ">=" ) additive } ;
additive     = multiplicative { ( "+" | "-" ) multiplicative } ;
multiplicative = unary { ( "*" | "/" | "%" ) unary } ;
unary        = ( "!" | "-" | "+" | "++" | "--" | "await" ) unary | postfix ;
postfix      = call_member [ "++" | "--" ] ;      (* no line break before ++/-- *)
call_member  = ( "new" primary { "." name } [ args ] | primary )
               { "." name | "[" expr "]" | args } ; (* no line break before "[" or "(" *)
args         = "(" [ expr { "," expr } [ "," ] ] ")" ;
primary      = number | string | "true" | "false" | "null" | "undefined"
             | name | "(" expr ")" | "[" [ expr { "," expr } [ "," ] ] "]" ;

number       = digit { digit } [ "." digit { digit } ] [ unit ] ;
unit         = "mm" | "cm" | "deg" | "°" | "rot" | "s" | "ms" | "%" ;
```
