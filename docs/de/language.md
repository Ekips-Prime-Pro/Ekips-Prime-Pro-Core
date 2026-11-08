# Ekips Prime Pro Sprachreferenz (v0.1, Entwurf)

Ekips Prime Pro ist eine kleine Programmiersprache für LEGO® SPIKE™ Prime
Roboter. Ihre Syntax ist eine Teilmenge von **TypeScript ohne
Typangaben**, plus **Einheiten an Zahlen** (`20cm`, `90deg`, `2s`, `50%`).
Sie ist für Schülerinnen und Schüler der Klassen 7–10 gedacht und für
KI-Assistenten, die ihnen helfen.

Diese Datei ist die einzige verbindliche Beschreibung der Sprache. Sie ist
auch dafür gedacht, einem KI-Assistenten als Kontext gegeben zu werden.

> **Umsetzungsstand:** Der Parser kann die gesamte Syntax auf dieser Seite.
> Die semantische Analyse prüft Namen, Gültigkeitsbereiche und die
> Benutzung der Standardbibliothek (siehe [compiler.md](compiler.md),
> vollständige Liste in [stdlib.md](stdlib.md)) und Einheiten.
> Codeerzeugung und die VM sind noch nicht geschrieben.

## 1. Ein erstes Programm

```ts
// Aufbau: was ist wo eingesteckt
const leftWheel = new Motor(Port.A);
const rightWheel = new Motor(Port.B);
const robot = new DriveBase(leftWheel, rightWheel, 56mm, 112mm);

async function square(side) {
  for (let i = 0; i < 4; i++) {
    await robot.drive(side);
    await robot.turn(90deg);
  }
}

// Hier startet das Programm
async function main() {
  await square(20cm);
  await hub.beep();
}
```

## 2. Grundlagen

- Dateien enden auf `.ekips`.
- Das Programm startet in **`async function main()`**. Jedes Programm
  braucht sie.
- Code außerhalb von Funktionen läuft einmal vor `main`. Benutze ihn für den
  Aufbau (`const arm = new Motor(Port.C);`) und für Konstanten.
- Kommentare: `// bis zum Ende der Zeile` und `/* über mehrere Zeilen */`.
- Semikolons am Zeilenende sind freiwillig. Zwei Befehle in derselben Zeile
  brauchen ein `;` dazwischen.
- Bei Namen zählt Groß- und Kleinschreibung (`speed` und `Speed` sind
  verschieden).
- Es gibt **keine Typen**. Schreibe nicht `: number` oder `: void`; der
  Compiler erklärt das, wenn du es doch tust.

## 3. Einheiten

Eine Einheit wird direkt hinter eine Zahl geschrieben, ohne Leerzeichen.

| Art | Einheiten | Beispiel |
|---|---|---|
| Länge | `mm`, `cm` | `20cm`, `150mm` |
| Winkel | `deg` (oder `°`), `rot` (ganze Umdrehungen) | `90deg`, `90°`, `1.5rot` |
| Zeit | `s`, `ms` | `2s`, `300ms` |
| Geschwindigkeit / Kraft | `%` | `50%` |

Werte behalten beim Rechnen ihre Einheit: `20cm + 5mm` ist `20.5cm`,
`2 * 90deg` ist `180deg`. Verschiedene Arten zu mischen (`20cm + 3s`) ist
ein Fehler, ebenso wie einem Befehl die falsche Art zu geben
(`robot.drive(90deg)`). Eine einfache `0` passt zu jeder Art
(`hub.heading > 0`). Alle Regeln: [compiler.md](compiler.md#einheitenprüfung).

`%` hinter einer Zahl ist ein Prozentwert. Zwischen zwei Werten ist es der
Rest-Operator: `10 % 3` ist `1`.

## 4. Standardbibliothek (geplante API)

### Aufbau

```ts
const arm    = new Motor(Port.C);
const eye    = new ColorSensor(Port.D);
const sonar  = new DistanceSensor(Port.E);
const bumper = new ForceSensor(Port.F);
const robot  = new DriveBase(leftMotor, rightMotor, wheelDiameter, axleTrack);
```

`Port.A` … `Port.F` sind die sechs Anschlüsse des Hubs. `wheelDiameter` ist
der Durchmesser der Räder, `axleTrack` der Abstand zwischen den beiden
Rädern (beides Längen, z. B. `56mm` und `112mm`).

Der Gyrosensor ist im Hub eingebaut (`hub.heading`).

### Befehle, die Zeit brauchen, geben ein Promise zurück

Befehle, die etwas bewegen, brauchen Zeit. Sie geben ein **Promise**
zurück: Die Bewegung startet sofort, und `await` wartet, bis sie fertig ist.

```ts
await robot.drive(20cm);          // fahren, dann weitermachen
const lifting = arm.rotate(90deg); // Arm anheben, sofort weitermachen
await robot.drive(20cm);          // fahren, während der Arm sich hebt
await lifting;                    // sicherstellen, dass der Arm fertig ist
```

> **`await` vergessen** ist der häufigste Fehler: Der Roboter beginnt dann
> den nächsten Befehl, während der vorherige noch läuft. Der Compiler warnt
> bei Befehlen, deren Promise weder mit `await` abgewartet, noch
> gespeichert, noch an `Promise.all`/`Promise.race` weitergegeben wird.

### DriveBase

| Mitglied | Beschreibung |
|---|---|
| `drive(distance, speed?)` | Geradeaus fahren. Negativ = rückwärts. Gibt ein Promise zurück. |
| `turn(angle, speed?)` | Auf der Stelle drehen. Positiv = rechts (im Uhrzeigersinn). Gibt ein Promise zurück. |
| `run(speed)` | Geradeaus losfahren, bis `stop()` kommt. |
| `stop()` | Beide Räder anhalten. |

### Motor

| Mitglied | Beschreibung |
|---|---|
| `rotate(angle, speed?)` | Um einen Winkel drehen (`deg` oder `rot`). Gibt ein Promise zurück. |
| `run(speed)` | Losdrehen, bis `stop()` kommt. Negativ = andere Richtung. |
| `stop()` | Anhalten und die Position halten. |
| `angle` | Winkel seit dem letzten `resetAngle()` (in `deg`). |
| `speed` | Aktuelle Geschwindigkeit (in `%`). |
| `resetAngle()` | `angle` auf `0deg` setzen. |

### Sensoren

| Mitglied | Beschreibung |
|---|---|
| `ColorSensor.color` | Farbe, die gerade gesehen wird, z. B. `Color.Red` |
| `ColorSensor.sees(color)` | `true`, wenn er diese Farbe sieht |
| `ColorSensor.reflection` | Reflektiertes Licht in `%` |
| `DistanceSensor.distance` | Abstand als Länge; `200cm`, wenn nichts in Reichweite ist |
| `ForceSensor.pressed` | `true`, solange er gedrückt ist |
| `ForceSensor.force` | Kraft in Newton (einfache Zahl) |

Farben: `Color.None`, `Color.Black`, `Color.Violet`, `Color.Blue`,
`Color.Azure`, `Color.Green`, `Color.Yellow`, `Color.Red`, `Color.White`.

### Hub

| Mitglied | Beschreibung |
|---|---|
| `hub.show(value)` | Eine Zahl oder einen kurzen Text auf der Lichtmatrix zeigen |
| `hub.beep(frequency?, duration?)` | Piepen. Gibt ein Promise zurück. |
| `hub.heading` | Gyro-Richtung seit `resetHeading()` (in `deg`, rechts = positiv) |
| `hub.resetHeading()` | Die Richtung auf `0deg` setzen |
| `hub.timer` | Zeit seit dem Start oder `resetTimer()` (in `s`) |
| `hub.resetTimer()` | Den Timer auf `0s` setzen |
| `hub.battery` | Akkuladung in `%` |

### Globale Funktionen

| Funktion | Beschreibung |
|---|---|
| `wait(time)` | Promise, das nach `time` fertig ist |
| `waitUntil(() => condition)` | Promise, das fertig ist, sobald die Bedingung `true` ist |
| `Promise.all([a, b, …])` | Fertig, wenn **alle** angegebenen Promises fertig sind |
| `Promise.race([a, b, …])` | Fertig, wenn das **erste** fertig ist |
| `console.log(a, b, …)` | Auf dem Computer ausgeben (USB-Konsole / Simulator) |
| `Math.abs`, `Math.min`, `Math.max`, `Math.round`, `Math.floor`, `Math.ceil`, `Math.sqrt`, `Math.random`, `Math.PI` | Mathematik |

### Mehrere Dinge gleichzeitig tun

```ts
// Beides gleichzeitig, weiter wenn beides fertig ist
await Promise.all([robot.drive(80cm), arm.rotate(180deg)]);

// Bis zu einer schwarzen Linie fahren, aber höchstens 1 m
await Promise.race([
  robot.drive(100cm),
  waitUntil(() => eye.sees(Color.Black)),
]);
robot.stop();

// Eigene async-Funktionen laufen genauso parallel
async function wave() {
  await arm.rotate(90deg);
  await arm.rotate(-90deg);
}
await Promise.all([robot.drive(30cm), wave()]);
```

## 5. Befehle (Anweisungen)

```ts
const SPEED = 60%;          // kann später nicht geändert werden
let count = 0;              // kann später geändert werden
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

break;     // die Schleife verlassen
continue;  // nächste Runde der Schleife
return x;  // die Funktion verlassen (mit einem Wert)
```

## 6. Funktionen

```ts
function double(x) {
  return x * 2;
}

async function square(side) {   // async: darf innen await benutzen
  for (let i = 0; i < 4; i++) {
    await robot.drive(side);
    await robot.turn(90deg);
  }
}

// Pfeilfunktionen
const isDark = () => eye.reflection < 20%;
const half = x => x / 2;
const go = async () => { await robot.drive(10cm); };
```

`await` funktioniert nur innerhalb von `async`-Funktionen (und
`async`-Pfeilfunktionen).

## 7. Werte und Operatoren

| Art | Beispiele |
|---|---|
| Zahl | `42`, `2.5`, `20cm`, `90deg`, `50%` |
| Text | `"hallo"`, `'hallo'` (Escapes: `\n`, `\t`, `\"`, `\'`, `\\`) |
| Ja/Nein | `true`, `false` |
| Nichts | `null`, `undefined` |
| Liste | `[1, 2, 3]`, `list[0]`, `list.length` |

| Operatoren (schwächste Bindung zuerst) | |
|---|---|
| `=` `+=` `-=` `*=` `/=` `%=` | Zuweisung |
| `? :` | Bedingter Wert: `x > 0 ? x : -x` |
| `\|\|` | oder |
| `&&` | und |
| `==` `===` `!=` `!==` | Gleich / ungleich (`==` und `===` bedeuten dasselbe) |
| `<` `<=` `>` `>=` | Vergleich |
| `+` `-` | Addieren, subtrahieren (`+` verbindet auch Texte) |
| `*` `/` `%` | Multiplizieren, dividieren, Rest |
| `!` `-` `+` `await` `++` `--` | Präfix |
| `++` `--` | Postfix |
| `.` `[]` `()` `new` | Mitglied, Index, Aufruf, Erzeugen |

## 8. Unterschiede zu TypeScript

- Keine Typangaben, Interfaces, `type`, Generics oder `enum`-Deklarationen.
- Keine Klassen, Objekt-Literale (`{ a: 1 }`), `import`/`export`,
  Template-Strings, `var`, `switch`, `try`/`catch` (noch nicht).
- `==` und `===` vergleichen beide streng; es gibt keine Typumwandlung.
- Ein `(` oder `[` am **Anfang einer Zeile** beginnt immer einen neuen
  Befehl. (In JavaScript würde `let a = b` gefolgt von `(c)` in der nächsten
  Zeile `b(c)` aufrufen.)
- Einheiten an Zahlen (`20cm`) gibt es in TypeScript nicht.
- Jedes Programm braucht `async function main()`.

## 9. Reservierte Wörter

```
async await break const continue else false for function if let new
null of return true undefined while
```

Schlüsselwörter dürfen trotzdem als Eigenschaftsnamen benutzt werden
(`Promise.race`, `x.new`).

## 10. Grammatik (EBNF)

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
postfix      = call_member [ "++" | "--" ] ;      (* kein Zeilenumbruch vor ++/-- *)
call_member  = ( "new" primary { "." name } [ args ] | primary )
               { "." name | "[" expr "]" | args } ; (* kein Zeilenumbruch vor "[" oder "(" *)
args         = "(" [ expr { "," expr } [ "," ] ] ")" ;
primary      = number | string | "true" | "false" | "null" | "undefined"
             | name | "(" expr ")" | "[" [ expr { "," expr } [ "," ] ] "]" ;

number       = digit { digit } [ "." digit { digit } ] [ unit ] ;
unit         = "mm" | "cm" | "deg" | "°" | "rot" | "s" | "ms" | "%" ;
```
