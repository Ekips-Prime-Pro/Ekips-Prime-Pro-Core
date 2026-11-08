# Virtuelle Maschine (`ekips-vm`)

Die VM führt übersetzte Programme aus. Sie ist `no_std` mit `alloc`,
deshalb läuft derselbe Code in der Hub-Firmware (STM32F413) und im
Simulator auf dem PC. Sie spricht nie direkt mit der Hardware: Jeder Motor-,
Sensor- und Hub-Aufruf geht über das Trait `Hardware` aus `ekips-hub-api`.

## Benutzung

```rust
use ekips_vm::{MockHub, Status, Vm};

let bytes = ekips_compiler::compile(source).bytecode.unwrap();
let mut vm = Vm::load(&bytes)?;        // prüft Kopf und Formatversion
let mut hub = MockHub::new();          // oder die echte Hardware in der Firmware
loop {
    match vm.tick(&mut hub)? {         // führt das Programm ein Stück weiter aus
        Status::Finished => break,
        Status::Running => hub.advance(10), // Firmware: ~1 ms warten
    }
}
```

`tick` blockiert nie. Die Firmware ruft es in ihrer Hauptschleife auf;
zwischen zwei Ticks kann sie Motoren, Sensoren und USB bedienen.

## Tasks und der kooperative Scheduler

- Jede laufende `async`-Funktion ist eine **Task** mit eigenem Stapel.
  Funktion 0 (Aufbau + `await main()`) ist die Wurzel-Task.
- Der Aufruf einer `async`-Funktion erzeugt eine Task und führt sie
  **sofort** aus, bis sie warten muss – wie in JavaScript. Der Aufrufer
  bekommt dann ein Promise.
- `await` auf ein Promise, das noch nicht fertig ist, **hält die Task an**.
- `tick` geht einmal durch alle Tasks. Eine wartende Task wird geweckt, wenn
  ihr Promise fertig ist; eine bereite Task läuft, bis sie warten muss oder
  ihre **Zeitscheibe** verbraucht hat (`DEFAULT_BUDGET` = 1000 Befehle,
  `Vm::set_budget`). Weil jede Task ihre eigene Zeitscheibe hat, kann eine
  Task in einer Endlosschleife (`while (!done) {}`) die anderen nicht
  aufhalten.
- Das Programm ist fertig, wenn die Wurzel-Task zurückkehrt, also wenn
  `main` fertig ist. Bewegungen, die noch laufen, hält die VM nicht an; die
  Firmware stoppt nach dem Programm alle Motoren.

## Promises

| Erzeugt durch | Fertig, wenn |
|---|---|
| `motor.rotate(…)` | der Motor `motor_done` meldet oder einen neuen Befehl bekommt |
| `robot.drive(…)`, `robot.turn(…)` | `drive_done`, oder die DriveBase bekommt einen neuen Befehl (`stop`, `drive`, …) |
| `wait(t)`, `hub.beep(…)` | die Zeit vergangen ist |
| `waitUntil(f)` | `f()` einen wahren Wert liefert (wird einmal pro Tick geprüft, solange darauf gewartet wird) |
| `Promise.all([…])` | alle fertig sind; das Ergebnis ist die Liste der Ergebnisse |
| `Promise.race([…])` | das erste fertig ist; die anderen laufen weiter |
| Aufruf einer `async`-Funktion | die Funktion zurückkehrt; das Ergebnis ist ihr Rückgabewert |

Die Bedingung von `waitUntil` läuft bei jeder Prüfung bis zum Ende; sie
darf kein `await` benutzen und muss in 10 000 Befehlen fertig sein.

## Werte

`undefined`, `null`, Ja/Nein, Zahlen (`f32` mit ihrer Art), Texte, Listen
(geteilte Verweise), Funktionen (Closures mit eingefangenen Zellen),
Promises, Anschlüsse, Farben und Geräte. `console.log` gibt Zahlen in den
Einheiten aus, die man schreibt: `20cm`, `90deg`, `2.5s`, `50%`.

## Laufzeitfehler

Probleme, die der Compiler nicht sehen kann (ein Gerät ist nicht
eingesteckt, endlose Rekursion, Schreiben weit hinter das Ende einer Liste,
…), halten das Programm mit einem `RuntimeError` an: eine Art, die Quellzeile
und eine Meldung auf Englisch und Deutsch (`message(Lang::De)`).

## Mock-Hardware

`MockHub` ist ein simulierter Hub für Tests, Beispiele und den Simulator:

- simulierte Uhr: Nichts bewegt sich bis `advance(ms)`;
- Bewegungen dauern realistisch lange (100% = 1000 Grad pro Sekunde am
  Motor bzw. Rad);
- jeder Aufruf wird in `log` / `log_lines()` aufgezeichnet;
- Sensorwerte kann man setzen (`set`) oder zeitlich planen
  (`script(time, change)`);
- `MockHub::new()` akzeptiert jedes Gerät, `MockHub::with_devices(…)` nur
  die angegebenen (um Fehler wie „nichts eingesteckt“ zu testen).

## Beispiel: Programm ausführen und die Hardwareaufrufe ausgeben

```sh
cargo run -p ekips-vm --example run -- examples/square.ekips
cargo run -p ekips-vm --example run -- --de examples/drive_to_line.ekips --at 1500 color C black
```

```text
[  0.000s] drive: configure left A right B wheel 5.6cm axle 11.2cm
[  0.000s] drive: straight 100cm at 40%
[  1.500s] drive: stop
[  1.500s] motor A: stop
[  1.500s] motor B: stop
[  1.500s] console: Found the line!
Programm nach 1.510s beendet
```

Optionen: `--de` (deutsche Meldungen), `--asm` (Bytecode anzeigen),
`--at ZEIT color|distance|press|release PORT [WERT]` (einen Sensor ändern).
