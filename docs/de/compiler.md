# Compiler

Der Compiler (`crates/compiler`) übersetzt eine `.ekips`-Datei in mehreren
Durchläufen in Bytecode. Jeder Durchlauf meldet *alle* gefundenen Probleme,
auf Englisch und Deutsch, mit einem Hinweis, wie man sie behebt.

| Durchlauf | Modul | Findet |
|---|---|---|
| Lexer | `lexer.rs` | unbekannte Zeichen, unbekannte Einheiten, nicht geschlossene Texte/Kommentare |
| Parser | `parser.rs` | Syntaxfehler, falsch geschriebene Schlüsselwörter, `await` außerhalb von `async`, fehlendes `main` |
| Semantische Analyse | `sema.rs` | unbekannte Namen, Gültigkeitsbereiche, `const`, falsche Benutzung eingebauter Dinge (diese Seite) |

API: `ekips_compiler::parse(source)` führt nur Lexer und Parser aus,
`ekips_compiler::check(source)` zusätzlich die semantische Analyse. Gibt es
Syntaxfehler, meldet `check` nur diese (siehe Entscheidung D3).

## Semantische Analyse

### Gültigkeitsbereiche

Gültigkeitsbereiche funktionieren wie in JavaScript:

- `let` und `const` sind ab ihrer Zeile bis zum Ende ihres Blocks sichtbar
  (`{ … }`, der Rumpf von `if`/`while`/`for`, eine Funktion).
- Eine Variable aus `for (let i = …)` oder `for (const x of …)` ist nur in der
  Schleife sichtbar.
- Funktionsparameter sind im Funktionsrumpf sichtbar.
- `function`-Deklarationen werden *hochgezogen* (hoisting): Man kann sie
  oberhalb der Zeile aufrufen, in der sie stehen.
- Code in einer Funktion läuft erst später, deshalb darf er Variablen
  benutzen, die weiter unten in einem äußeren Block erstellt werden. Code auf
  oberster Ebene und Code in derselben Funktion darf das nicht
  (`UsedBeforeDeclaration`).

### Prüfungen

| Problem | Beispiel | Meldung (Deutsch) |
|---|---|---|
| Unbekannter Name | `await wiat(1s)` | Den Namen `wiat` kenne ich nicht. *Meintest du `wait`?* |
| Unbekanntes Mitglied | `robot.drvie(20cm)`, `robot.fahre(20cm)` | `DriveBase` hat kein `drvie`. *Meintest du `drive`?* |
| Zweimal im selben Block erstellt | `let x = 1; let x = 2;` | `x` gibt es hier schon. (+ Hinweis auf die erste Zeile) |
| Verdeckt etwas Eingebautes | `const hub = 1` | `hub` ist schon ein eingebauter Name. |
| Benutzt, bevor erstellt | `console.log(y); let y = 1;` | `y` wird benutzt, bevor es erstellt wird. |
| Zuweisung an `const` | `const S = 1; S = 2;` (auch `+=`, `++`) | `S` ist eine Konstante (`const`) und kann nicht geändert werden. |
| Zuweisung an Funktion oder Eingebautes | `main = 1`, `hub.heading = 0deg` | … kann nicht geändert werden / kann nur gelesen werden. |
| Befehl ohne `()` | `robot.stop;` | `robot.stop` ist ein Befehl und muss mit Klammern aufgerufen werden. |
| Eingebautes alleine benutzt | `const h = hub` | `hub` kann man nicht alleine benutzen. |
| Fehlendes `new` | `Motor(Port.A)` | Ein `Motor` wird mit `new` erstellt. |
| `new` mit etwas anderem als einem Gerät | `new wait(1s)` | `wait` kann man nicht mit `new` erstellen. |
| Einen Wert aufrufen | `arm.angle()` | `arm.angle` ist kein Befehl und kann nicht aufgerufen werden. |
| Falsche Anzahl Argumente | `robot.drive()` | `robot.drive` braucht 1 bis 2 Argument(e), bekommt aber 0. |
| Falscher Argumenttyp | `new Motor(5)` | Bei `Motor` muss `port` ein Anschluss wie `Port.A` sein, ist aber eine Zahl ohne Einheit. |

Vorschläge („Meintest du …?“) ignorieren Groß- und Kleinschreibung
(`spead` → `SPEED`), kennen deutsche und andere Wörter aus der
Standardbibliothek-Tabelle (`warte` → `wait`, `fahre` → `drive`,
`Color.Rot` → `Color.Red`) und raten bei einzelnen Buchstaben nie.

### Typen

Es gibt keine Typangaben, aber der Compiler leitet für jeden Ausdruck einen
groben Typ ab: Zahl einer Art (`Length`, `Angle`, `Time`, `Percent`,
`Number`), Ja/Nein, Text, Liste, Funktion, Promise, Anschluss, Farbe oder
ein Gerät (`Motor`, …). Was er nicht wissen kann – Parameter,
Listenelemente – ist „unbekannt“ und führt nie zu einem Fehler. Eine
Variable bekommt den Typ ihres Anfangswerts; eine einfache `0` zählt als
unbekannt (Zähler beginnen oft bei `0`).

Bei einem Wert mit unbekanntem Typ wird `.mitglied` über alle Geräte hinweg
nach dem Namen geprüft: `motor.rotate(…)` auf einem Parameter `motor` ist in
Ordnung, `motor.rotat` ist ein Fehler.

### Ergebnisse für spätere Durchläufe

`check` liefert eine `sema::Analysis` mit

- `resolutions`: worauf sich jeder Name bezieht (Deklaration oder Eingebautes),
- `bindings`: jede Deklaration mit Art, Typ, Speicherort (global oder lokal
  in welcher Funktion) und ob eine innere Funktion sie *einfängt* (capture),
- `members`: wie jedes `.mitglied` aufgelöst wurde (Namensraum, bekanntes
  Gerät oder dynamisch).

Codeerzeugung und Language Server benutzen diese Ergebnisse, statt Namen
noch einmal aufzulösen.

## Tabelle der Standardbibliothek

Alles Eingebaute ist in **einer Datentabelle** beschrieben,
`crates/compiler/src/stdlib.rs` (`GLOBALS`): Namen, Parameter mit erwarteter
Art, optionale Parameter, Rückgabetyp, die Native-Funktion der VM,
Dokumentation in beiden Sprachen und Aliase. Die Referenz
[`stdlib.md`](stdlib.md) wird daraus erzeugt:

```sh
cargo run -p ekips-compiler --example gen-docs
```

Ein Test schlägt fehl, wenn die erzeugten Dateien veraltet sind.
