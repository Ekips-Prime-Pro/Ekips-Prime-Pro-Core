# Architekturentscheidungen

Kurze Aufzeichnungen von Entscheidungen, die das Projekt prägen. Neueste
zuletzt. Jeder Eintrag sagt, was entschieden wurde und warum.

## D1 – Die Dokumentation liegt in `docs/`, auf Englisch und Deutsch

- **Entscheidung:** Das Wiki besteht aus Markdown-Dateien in `docs/en` und
  `docs/de` mit gleichem Aufbau. `docs/README.md` verlinkt beide.
- **Warum:** Die Schülerinnen und Schüler im Team lesen Deutsch, Mitwirkende
  und KI-Assistenten meist Englisch. Weil die Doku neben dem Code liegt, wird
  sie zusammen mit dem Compiler, den sie beschreibt, versioniert und getaggt.

## D2 – Die Standardbibliothek ist eine Datentabelle

- **Entscheidung:** Alles Eingebaute ist in `GLOBALS` in
  `crates/compiler/src/stdlib.rs` beschrieben, mit Parameterarten, Doku in
  beiden Sprachen und Aliasen (deutsche Wörter). Analyse, Einheitenprüfung,
  Codeerzeugung, die erzeugte Referenz (`docs/*/stdlib.md`) und der Language
  Server lesen diese Tabelle.
- **Warum:** Ein Ort, um einen Befehl hinzuzufügen. Doku, Fehlermeldungen und
  Autovervollständigung können nicht auseinanderlaufen. Ein Test schlägt
  fehl, wenn die erzeugte Referenz veraltet ist.

## D3 – Syntaxfehler verdecken semantische Fehler

- **Entscheidung:** Meldet der Parser einen Fehler, meldet `check` keine
  semantischen Probleme (die Analyse läuft für Editoren trotzdem).
- **Warum:** Eine kaputte Zeile enthält oft eine Deklaration. „Unbekannter
  Name“ bei jeder späteren Benutzung würde den einen echten Fehler
  verstecken.

## D4 – Die IDs der Native-Funktionen liegen in `ekips-bytecode`

- **Entscheidung:** Das Enum `Native` (feste 16-Bit-IDs der eingebauten
  Funktionen) gehört zum `no_std`-Bytecode-Crate. Die Stdlib-Tabelle
  verweist auf diese IDs; ein Test prüft, dass jede ID genau einmal benutzt
  wird.
- **Warum:** Die IDs sind die ABI zwischen Compiler und VM. Die VM darf nicht
  vom Compiler abhängen (der `std` benutzt und die ganze Doku enthält), aber
  beide müssen sich bei den Nummern einig sein.

## D5 – Gültigkeitsbereiche und Hoisting wie in JavaScript

- **Entscheidung:** `let`/`const` gelten im Block, Funktionen werden an den
  Anfang ihres Blocks hochgezogen, und Funktionsrümpfe dürfen Variablen
  benutzen, die später in einem äußeren Block erstellt werden. Eine Variable
  vor ihrer Zeile in *derselben* Funktion (oder im Code auf oberster Ebene)
  zu benutzen, ist ein Fehler.
- **Warum:** Wer später JavaScript/TypeScript lernt, soll nichts umlernen
  müssen. Der Fehler „benutzt, bevor erstellt“ ersetzt den Laufzeitfehler,
  den JavaScript geben würde.

## D6 – Einheiten werden beim Übersetzen geprüft, nach Art

- **Entscheidung:** Der Compiler leitet für jede Zahl eine Art ab (`Length`,
  `Angle`, `Time`, `Percent`, `Number`) und lehnt das Mischen von Arten ab.
  Einheiten werden beim Übersetzen in eine Grundeinheit pro Art umgerechnet
  (mm, Grad, ms, Prozent), `20cm + 5mm` ist also einfach `200 + 5`. Produkte
  und Quotienten zweier Einheiten (`cm²`, `cm/s`) gibt es nicht. Eine
  einfache `0` passt zu jeder Art; Werte unbekannter Art sind nie Fehler.
- **Warum:** Die meisten Roboterfehler in der FLL sind falsche Zahlen (90
  fahren statt 90 drehen). Sie zu finden, bevor sich der Roboter bewegt, spart
  viel Zeit. Ohne zusammengesetzte Einheiten bleiben die Regeln einfach genug
  für Klasse 7.

## D7 – Stapelbasierte Bytecode-VM, `no_std` + `alloc`

- **Entscheidung:** Programme werden in einen kompakten Bytecode für eine
  Stapelmaschine übersetzt und von einem Interpreter in `ekips-vm`
  ausgeführt. Die VM benutzt `alloc` (`Vec`, `Rc`) und braucht in der
  Firmware einen globalen Allokator. Zahlen sind `f32` mit ihrer Art;
  Einheiten werden beim Übersetzen umgerechnet.
- **Warum:** Eine Bytecode-Datei ist klein für die Übertragung per USB und
  kann vor dem Start geprüft werden (Version). Eine Stapelmaschine ist am
  einfachsten zu erzeugen und zu interpretieren. `alloc` ermöglicht Listen,
  Closures und viele gleichzeitige Tasks ohne feste Grenzen; der Hub hat
  320 KiB RAM, genug für FLL-Programme. `f32` passt zur FPU des Hubs.

## D8 – Kooperativer Scheduler mit einer Zeitscheibe pro Task

- **Entscheidung:** Jede laufende `async`-Funktion ist eine Task.
  `Vm::tick` gibt jeder bereiten Task ihre eigene Zeitscheibe an Befehlen;
  wartende Tasks werden geweckt, indem ihr Promise abgefragt wird
  (`motor_done`, `drive_done`, Zeit, Bedingung). Der Aufruf einer
  `async`-Funktion führt sie sofort bis zum ersten Warten aus.
- **Warum:** Keine Threads, keine Interrupts und kein Rust-`async` nötig,
  daher läuft es gleich auf dem Hub und auf dem PC. Das Abfragen passt zum
  `Hardware`-Trait, dessen Bewegungsbefehle eine Bewegung nur *starten*.
  Eine Zeitscheibe pro Task heißt, dass eine Endlosschleife in einer Task
  den Roboter nicht einfrieren kann. Dass `async`-Funktionen sofort starten,
  entspricht JavaScript, so passt das Bild, das Schülerinnen und Schüler aus
  dem Web kennen.

## D9 – Promises von Bewegungen sind fertig, wenn das Gerät einen neuen Befehl bekommt

- **Entscheidung:** Ein Promise von `rotate`/`drive`/`turn` ist fertig, wenn
  die Bewegung beendet ist **oder** wenn derselbe Motor bzw. dieselbe
  DriveBase einen anderen Befehl bekommt (zum Beispiel `robot.stop()` nach
  einem `Promise.race`).
- **Warum:** Sonst würde eine angehaltene Bewegung nie fertig, und jede Task,
  die darauf wartet, würde hängen bleiben.

## D10 – Die Mock-Hardware liegt im VM-Crate

- **Entscheidung:** `ekips_vm::MockHub` (simulierte Uhr, realistische
  Bewegungszeiten, geplante Sensorwerte, Text-Protokoll) ist Teil von
  `ekips-vm`.
- **Warum:** Jeder Test, das Beispiel und der Simulator im Tools-Repository
  brauchen denselben einfachen, deterministischen Hub. Er ist ebenfalls
  `no_std` und schränkt das Crate deshalb nicht ein.
