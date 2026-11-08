# Standard library reference

<!-- Generated from crates/compiler/src/stdlib.rs by `cargo run -p ekips-compiler --example gen-docs`. Do not edit. -->

Everything that is built into Ekips Prime Pro. Commands that take time return a **Promise**; use `await` to wait until they are done.

## Devices

### Motor

A medium or large motor, e.g. for an arm: `new Motor(Port.C)`.

```ts
const x = new Motor(port);
```

| Member | Description | Type / parameters |
|---|---|---|
| `motor.rotate(angle, speed?)` | Rotates the motor by an angle. Returns a Promise that is done when the motor arrives. | `angle`: an angle<br>`speed`: a percentage<br>→ a Promise |
| `motor.run(speed)` | Starts rotating until `stop()` is called. | `speed`: a percentage<br>→ nothing (`undefined`) |
| `motor.stop()` | Stops the motor and holds its position. | → nothing (`undefined`) |
| `motor.angle` | Angle since the last `resetAngle()`. | an angle |
| `motor.speed` | Current speed. | a percentage |
| `motor.resetAngle()` | Sets `angle` to `0deg`. | → nothing (`undefined`) |

### ColorSensor

The color sensor: `new ColorSensor(Port.D)`.

```ts
const x = new ColorSensor(port);
```

| Member | Description | Type / parameters |
|---|---|---|
| `sensor.color` | The color seen right now, e.g. `Color.Red`. | a color like `Color.Red` |
| `sensor.sees(color)` | `true` if the sensor sees this color right now. | `color`: a color like `Color.Red`<br>→ a yes/no value (`true` or `false`) |
| `sensor.reflection` | Reflected light: `0%` = dark, `100%` = bright. | a percentage |

### DistanceSensor

The ultrasonic distance sensor: `new DistanceSensor(Port.E)`.

```ts
const x = new DistanceSensor(port);
```

| Member | Description | Type / parameters |
|---|---|---|
| `sensor.distance` | Distance to the nearest object; `200cm` if nothing is in range. | a length |

### ForceSensor

The force (touch) sensor: `new ForceSensor(Port.F)`.

```ts
const x = new ForceSensor(port);
```

| Member | Description | Type / parameters |
|---|---|---|
| `sensor.pressed` | `true` while pressed. | a yes/no value (`true` or `false`) |
| `sensor.force` | Force in newtons (a plain number). | a plain number |

### DriveBase

Two wheels that drive the robot: `new DriveBase(left, right, 56mm, 112mm)`.

```ts
const x = new DriveBase(left, right, wheelDiameter, axleTrack);
```

| Member | Description | Type / parameters |
|---|---|---|
| `robot.drive(distance, speed?)` | Drives straight. Returns a Promise that is done when the robot arrives. | `distance`: a length<br>`speed`: a percentage<br>→ a Promise |
| `robot.turn(angle, speed?)` | Turns on the spot. Returns a Promise that is done when the turn is finished. | `angle`: an angle<br>`speed`: a percentage<br>→ a Promise |
| `robot.run(speed)` | Starts driving straight until `stop()` is called. | `speed`: a percentage<br>→ nothing (`undefined`) |
| `robot.stop()` | Stops both wheels. | → nothing (`undefined`) |

## Built-in objects

### hub

The SPIKE Prime hub itself: light matrix, speaker, gyro, timer, battery.

| Member | Description | Type / parameters |
|---|---|---|
| `hub.show(value)` | Shows a number or short text on the light matrix. | `value`: any value<br>→ nothing (`undefined`) |
| `hub.beep(frequency?, duration?)` | Beeps. Returns a Promise that is done when the beep is over. | `frequency`: a plain number<br>`duration`: a time<br>→ a Promise |
| `hub.heading` | Gyro heading since `resetHeading()`; right = positive. | an angle |
| `hub.resetHeading()` | Sets `heading` to `0deg`. | → nothing (`undefined`) |
| `hub.timer` | Time since the start or since `resetTimer()`. | a time |
| `hub.resetTimer()` | Sets `timer` to `0s`. | → nothing (`undefined`) |
| `hub.battery` | Battery charge. | a percentage |

### Port

The six ports of the hub: `Port.A` … `Port.F`.

| Member | Description | Type / parameters |
|---|---|---|
| `Port.A` | Port A | a port like `Port.A` |
| `Port.B` | Port B | a port like `Port.A` |
| `Port.C` | Port C | a port like `Port.A` |
| `Port.D` | Port D | a port like `Port.A` |
| `Port.E` | Port E | a port like `Port.A` |
| `Port.F` | Port F | a port like `Port.A` |

### Color

Colors the color sensor can see.

| Member | Description | Type / parameters |
|---|---|---|
| `Color.None` | No color (nothing in front of the sensor) | a color like `Color.Red` |
| `Color.Black` | Black | a color like `Color.Red` |
| `Color.Violet` | Violet | a color like `Color.Red` |
| `Color.Blue` | Blue | a color like `Color.Red` |
| `Color.Azure` | Light blue | a color like `Color.Red` |
| `Color.Green` | Green | a color like `Color.Red` |
| `Color.Yellow` | Yellow | a color like `Color.Red` |
| `Color.Red` | Red | a color like `Color.Red` |
| `Color.White` | White | a color like `Color.Red` |

### Math

Mathematics.

| Member | Description | Type / parameters |
|---|---|---|
| `Math.abs(x)` | Value without sign: `Math.abs(-3cm)` is `3cm`. | `x`: a number (any unit)<br>→ same kind as the input |
| `Math.min(values, values, …)` | The smallest of the values. | `values`: a number (any unit)<br>`values`: a number (any unit)<br>→ same kind as the input |
| `Math.max(values, values, …)` | The largest of the values. | `values`: a number (any unit)<br>`values`: a number (any unit)<br>→ same kind as the input |
| `Math.round(x)` | Rounds to the nearest whole number. | `x`: a number (any unit)<br>→ same kind as the input |
| `Math.floor(x)` | Rounds down. | `x`: a number (any unit)<br>→ same kind as the input |
| `Math.ceil(x)` | Rounds up. | `x`: a number (any unit)<br>→ same kind as the input |
| `Math.sqrt(x)` | Square root. | `x`: a plain number<br>→ a plain number |
| `Math.random()` | A random number from 0 (inclusive) to 1 (exclusive). | → a plain number |
| `Math.PI` | The number π (3.14159…). | a plain number |

### Promise

Combine Promises to do several things at the same time.

| Member | Description | Type / parameters |
|---|---|---|
| `Promise.all(promises)` | Done when **all** given Promises are done. | `promises`: a list<br>→ a Promise |
| `Promise.race(promises)` | Done when the **first** of the given Promises is done. | `promises`: a list<br>→ a Promise |

### console

Output on the computer.

| Member | Description | Type / parameters |
|---|---|---|
| `console.log(values?, …)` | Prints the values on the computer (USB console / simulator). | `values`: any value<br>→ nothing (`undefined`) |

## Global functions

| Function | Description | Type / parameters |
|---|---|---|
| `wait(time)` | A Promise that is done after the given time: `await wait(2s)`. | `time`: a time<br>→ a Promise |
| `waitUntil(condition)` | A Promise that is done as soon as the condition is `true`. | `condition`: a function<br>→ a Promise |

## Lists and texts

| Member | Description | Type / parameters |
|---|---|---|
| `list.length` | Number of items in a list or characters in a text. | a plain number |
