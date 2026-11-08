# Bytecode-Format und Befehlssatz

Der Compiler erzeugt pro Programm eine Binärdatei. Der Hub (oder der
Simulator) lädt sie in die VM (`ekips-vm`). Das Format ist in
`crates/bytecode` (`no_std`) festgelegt, das beide Seiten benutzen.

## Aufbau der Datei

Alle Ganzzahlen sind Little-Endian.

| Teil | Inhalt |
|---|---|
| Kopf (16 Bytes) | Magic `EKPP`, Formatversion (u16, derzeit `1`), Flags (u16, `0`), Code-Länge (u32), Daten-Länge (u32) |
| Code-Abschnitt | die Befehle aller Funktionen, nacheinander |
| Daten-Abschnitt | Anzahl globaler Variablen, Funktionstabelle, Konstanten |

Die Firmware prüft zuerst Magic und Version. Ein Programm für eine andere
Formatversion wird mit einer klaren Meldung abgelehnt, statt abzustürzen.

### Daten-Abschnitt

```text
u16 globals                    Anzahl globaler Variablen
u16 function_count
function_count × Funktion:
    str  name                  u16 Länge + UTF-8
    u32  offset, u32 length    Position im Code-Abschnitt
    u8   params, u8 locals     locals enthält die Parameter
    u8   is_async
    u8   capture_count
    capture_count × (u8 kind, u8 index)   kind 0 = lokaler Slot, 1 = Upvalue
u16 constant_count
constant_count × Konstante:
    0: f32 Wert, u8 Art        Zahl (Art: 0 Number, 1 Length, 2 Angle, 3 Time, 4 Percent)
    1: str                     Text
    2: u8 port                 0 = A … 5 = F
    3: u8 color                Reihenfolge von `Color::ALL`
    4: u8 n, n × u16 native    Methodenmenge (für `CallMethod`)
```

**Funktion 0** ist der Einstiegspunkt. Sie führt den Code auf oberster Ebene
(Aufbau) aus und danach `await main()`. Das Programm ist fertig, wenn
Funktion 0 zurückkehrt.

## Werte und Einheiten

Zahlen sind `f32` (die FPU des Hubs rechnet mit einfacher Genauigkeit) und
tragen ihre *Art*. Einheiten rechnet der Compiler in eine Grundeinheit pro
Art um:

| Art | Grundeinheit | `20cm` wird gespeichert als |
|---|---|---|
| Length (Länge) | mm | `200` |
| Angle (Winkel) | Grad | `1.5rot` → `540` |
| Time (Zeit) | ms | `2s` → `2000` |
| Percent (Prozent) | % | `50%` → `50` |

Die Art wird nur zum Ausgeben benutzt (`console.log(20cm)` gibt `20cm` aus)
und für die Art eines Ergebnisses (`2 * 90deg` ist ein Winkel). Alle
Einheiten-*Prüfungen* passieren im Compiler.

## Befehle

Die VM ist eine **Stapelmaschine** (stack machine). Jeder Befehl ist ein
Opcode-Byte plus Operanden. Sprungziele sind Abstände relativ zum Anfang der
aktuellen Funktion (u16, eine Funktion darf also höchstens 64 KiB Code
haben).

| Gruppe | Befehle |
|---|---|
| Konstanten | `Const idx`, `Undefined`, `Null`, `True`, `False` |
| Stapel | `Pop`, `Dup`, `Dup2` |
| Variablen | `LoadLocal n`, `StoreLocal n`, `LoadGlobal n`, `StoreGlobal n` |
| Eingefangene Variablen | `NewCell n`, `LoadCell n`, `StoreCell n`, `LoadUpval n`, `StoreUpval n` |
| Rechnen | `Add`, `Sub`, `Mul`, `Div`, `Rem`, `Neg`, `ToNumber` |
| Logik / Vergleich | `Not`, `Eq`, `NotEq`, `Lt`, `Le`, `Gt`, `Ge` |
| Sprünge | `Jump t`, `JumpIfFalse t`, `JumpIfTrueKeep t` (für `\|\|`), `JumpIfFalseKeep t` (für `&&`) |
| Funktionen | `Closure fn`, `Call argc`, `CallNative id argc`, `CallMethod set argc`, `Return` |
| Listen | `MakeList n`, `GetIndex`, `SetIndex` |
| Async | `Await` |
| Debug | `Line n` (Beginn der Quellzeile `n`, für Laufzeitfehler) |

Die genaue Kodierung jedes Opcodes steht in
`crates/bytecode/src/opcode.rs`. `Program::disassemble()` gibt eine lesbare
Liste aus.

### Eingebaute Funktionen

`CallNative id argc` führt eine eingebaute Funktion aus. Die IDs sind das
Enum `Native` in `crates/bytecode/src/native.rs` (Teil der ABI). Methoden
bekommen ihr Objekt als erstes Argument (`robot.drive(20cm)` →
`robot, 200mm`). Fehlende optionale Argumente werden einfach nicht
übergeben.

Kennt der Compiler den Typ eines Werts nicht (ein Parameter `motor` in
`function lift(motor) { motor.rotate(90deg) }`), erzeugt er
`CallMethod set argc`. Die Konstante `set` listet alle Natives mit diesem
Namen auf (`MotorRotate`); die VM wählt zur Laufzeit die, deren Empfängerart
(`Native::receiver()`) zum Wert passt.

### Variablen

- **Globale**: `const`/`let`/`function` direkt auf oberster Ebene.
- **Lokale**: alles andere, in nummerierten Slots des Funktionsrahmens
  (Parameter zuerst).
- **Eingefangene Variablen**: Eine lokale Variable, die eine innere Funktion
  benutzt (Pfeilfunktion, verschachtelte Funktion), liegt in einer *Zelle*
  auf dem Heap. Die deklarierende Funktion erzeugt die Zelle mit `NewCell`,
  wenn der Block betreten wird, und benutzt `LoadCell`/`StoreCell`; die
  Closure speichert einen Verweis auf die Zelle und benutzt
  `LoadUpval`/`StoreUpval`. Weil die Zelle bei jedem Betreten des Blocks neu
  erzeugt wird, bekommt jede Schleifenrunde ihre eigene Variable (wie `let`
  in JavaScript).

### Async-Funktionen und Promises

`Call` auf eine `async`-Funktion führt sie nicht in der aktuellen Aufgabe
(Task) aus. Es erzeugt eine **neue Task**, führt sie sofort bis zu ihrem
ersten `await` aus, das warten muss (wie JavaScript), und legt ein
**Promise** für ihr Ergebnis auf den Stapel. `Await` auf ein noch nicht
fertiges Promise hält die aktuelle Task an; der Scheduler setzt sie fort,
sobald das Promise fertig ist. Details: [vm.md](vm.md).
