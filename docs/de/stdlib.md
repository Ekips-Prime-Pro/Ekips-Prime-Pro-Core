# Referenz der Standardbibliothek

<!-- Erzeugt aus crates/compiler/src/stdlib.rs mit `cargo run -p ekips-compiler --example gen-docs`. Nicht von Hand ändern. -->

Alles, was in Ekips Prime Pro eingebaut ist. Befehle, die Zeit brauchen, geben ein **Promise** zurück; mit `await` wartest du, bis sie fertig sind.

## Geräte

### Motor

Ein mittlerer oder großer Motor, z. B. für einen Arm: `new Motor(Port.C)`.

```ts
const x = new Motor(port);
```

| Mitglied | Beschreibung | Typ / Parameter |
|---|---|---|
| `motor.rotate(angle, speed?)` | Dreht den Motor um einen Winkel. Gibt ein Promise zurück, das fertig ist, wenn der Motor angekommen ist. | `angle`: ein Winkel<br>`speed`: ein Prozentwert<br>→ ein Promise |
| `motor.run(speed)` | Beginnt zu drehen, bis `stop()` aufgerufen wird. | `speed`: ein Prozentwert<br>→ nichts (`undefined`) |
| `motor.stop()` | Hält den Motor an und hält seine Position. | → nichts (`undefined`) |
| `motor.angle` | Winkel seit dem letzten `resetAngle()`. | ein Winkel |
| `motor.speed` | Aktuelle Geschwindigkeit. | ein Prozentwert |
| `motor.resetAngle()` | Setzt `angle` auf `0deg`. | → nichts (`undefined`) |

### ColorSensor

Der Farbsensor: `new ColorSensor(Port.D)`.

```ts
const x = new ColorSensor(port);
```

| Mitglied | Beschreibung | Typ / Parameter |
|---|---|---|
| `sensor.color` | Die Farbe, die gerade gesehen wird, z. B. `Color.Red`. | eine Farbe wie `Color.Red` |
| `sensor.sees(color)` | `true`, wenn der Sensor diese Farbe gerade sieht. | `color`: eine Farbe wie `Color.Red`<br>→ ein Ja/Nein-Wert (`true` oder `false`) |
| `sensor.reflection` | Reflektiertes Licht: `0%` = dunkel, `100%` = hell. | ein Prozentwert |

### DistanceSensor

Der Ultraschall-Abstandssensor: `new DistanceSensor(Port.E)`.

```ts
const x = new DistanceSensor(port);
```

| Mitglied | Beschreibung | Typ / Parameter |
|---|---|---|
| `sensor.distance` | Abstand zum nächsten Gegenstand; `200cm`, wenn nichts in Reichweite ist. | eine Länge |

### ForceSensor

Der Kraft- bzw. Tastsensor: `new ForceSensor(Port.F)`.

```ts
const x = new ForceSensor(port);
```

| Mitglied | Beschreibung | Typ / Parameter |
|---|---|---|
| `sensor.pressed` | `true`, solange er gedrückt ist. | ein Ja/Nein-Wert (`true` oder `false`) |
| `sensor.force` | Kraft in Newton (eine Zahl ohne Einheit). | eine Zahl ohne Einheit |

### DriveBase

Zwei Räder, die den Roboter fahren: `new DriveBase(left, right, 56mm, 112mm)`.

```ts
const x = new DriveBase(left, right, wheelDiameter, axleTrack);
```

| Mitglied | Beschreibung | Typ / Parameter |
|---|---|---|
| `robot.drive(distance, speed?)` | Fährt geradeaus. Gibt ein Promise zurück, das fertig ist, wenn der Roboter angekommen ist. | `distance`: eine Länge<br>`speed`: ein Prozentwert<br>→ ein Promise |
| `robot.turn(angle, speed?)` | Dreht auf der Stelle. Gibt ein Promise zurück, das fertig ist, wenn die Drehung beendet ist. | `angle`: ein Winkel<br>`speed`: ein Prozentwert<br>→ ein Promise |
| `robot.run(speed)` | Fährt geradeaus los, bis `stop()` aufgerufen wird. | `speed`: ein Prozentwert<br>→ nichts (`undefined`) |
| `robot.stop()` | Hält beide Räder an. | → nichts (`undefined`) |

## Eingebaute Objekte

### hub

Der SPIKE-Prime-Hub selbst: Lichtmatrix, Lautsprecher, Gyro, Timer, Akku.

| Mitglied | Beschreibung | Typ / Parameter |
|---|---|---|
| `hub.show(value)` | Zeigt eine Zahl oder einen kurzen Text auf der Lichtmatrix. | `value`: beliebiger Wert<br>→ nichts (`undefined`) |
| `hub.beep(frequency?, duration?)` | Piept. Gibt ein Promise zurück, das fertig ist, wenn der Ton vorbei ist. | `frequency`: eine Zahl ohne Einheit<br>`duration`: eine Zeit<br>→ ein Promise |
| `hub.heading` | Gyro-Richtung seit `resetHeading()`; rechts = positiv. | ein Winkel |
| `hub.resetHeading()` | Setzt `heading` auf `0deg`. | → nichts (`undefined`) |
| `hub.timer` | Zeit seit dem Start oder seit `resetTimer()`. | eine Zeit |
| `hub.resetTimer()` | Setzt `timer` auf `0s`. | → nichts (`undefined`) |
| `hub.battery` | Akkuladung. | ein Prozentwert |

### Port

Die sechs Anschlüsse des Hubs: `Port.A` … `Port.F`.

| Mitglied | Beschreibung | Typ / Parameter |
|---|---|---|
| `Port.A` | Anschluss A | ein Anschluss wie `Port.A` |
| `Port.B` | Anschluss B | ein Anschluss wie `Port.A` |
| `Port.C` | Anschluss C | ein Anschluss wie `Port.A` |
| `Port.D` | Anschluss D | ein Anschluss wie `Port.A` |
| `Port.E` | Anschluss E | ein Anschluss wie `Port.A` |
| `Port.F` | Anschluss F | ein Anschluss wie `Port.A` |

### Color

Farben, die der Farbsensor sehen kann.

| Mitglied | Beschreibung | Typ / Parameter |
|---|---|---|
| `Color.None` | Keine Farbe (nichts vor dem Sensor) | eine Farbe wie `Color.Red` |
| `Color.Black` | Schwarz | eine Farbe wie `Color.Red` |
| `Color.Violet` | Violett | eine Farbe wie `Color.Red` |
| `Color.Blue` | Blau | eine Farbe wie `Color.Red` |
| `Color.Azure` | Hellblau | eine Farbe wie `Color.Red` |
| `Color.Green` | Grün | eine Farbe wie `Color.Red` |
| `Color.Yellow` | Gelb | eine Farbe wie `Color.Red` |
| `Color.Red` | Rot | eine Farbe wie `Color.Red` |
| `Color.White` | Weiß | eine Farbe wie `Color.Red` |

### Math

Mathematik.

| Mitglied | Beschreibung | Typ / Parameter |
|---|---|---|
| `Math.abs(x)` | Betrag ohne Vorzeichen: `Math.abs(-3cm)` ist `3cm`. | `x`: eine Zahl (beliebige Einheit)<br>→ gleiche Art wie die Eingabe |
| `Math.min(values, values, …)` | Der kleinste der Werte. | `values`: eine Zahl (beliebige Einheit)<br>`values`: eine Zahl (beliebige Einheit)<br>→ gleiche Art wie die Eingabe |
| `Math.max(values, values, …)` | Der größte der Werte. | `values`: eine Zahl (beliebige Einheit)<br>`values`: eine Zahl (beliebige Einheit)<br>→ gleiche Art wie die Eingabe |
| `Math.round(x)` | Rundet auf die nächste ganze Zahl. | `x`: eine Zahl (beliebige Einheit)<br>→ gleiche Art wie die Eingabe |
| `Math.floor(x)` | Rundet ab. | `x`: eine Zahl (beliebige Einheit)<br>→ gleiche Art wie die Eingabe |
| `Math.ceil(x)` | Rundet auf. | `x`: eine Zahl (beliebige Einheit)<br>→ gleiche Art wie die Eingabe |
| `Math.sqrt(x)` | Quadratwurzel. | `x`: eine Zahl ohne Einheit<br>→ eine Zahl ohne Einheit |
| `Math.random()` | Eine Zufallszahl von 0 (einschließlich) bis 1 (ausschließlich). | → eine Zahl ohne Einheit |
| `Math.PI` | Die Zahl π (3,14159…). | eine Zahl ohne Einheit |

### Promise

Promises kombinieren, um mehrere Dinge gleichzeitig zu tun.

| Mitglied | Beschreibung | Typ / Parameter |
|---|---|---|
| `Promise.all(promises)` | Fertig, wenn **alle** angegebenen Promises fertig sind. | `promises`: eine Liste<br>→ ein Promise |
| `Promise.race(promises)` | Fertig, wenn das **erste** der angegebenen Promises fertig ist. | `promises`: eine Liste<br>→ ein Promise |

### console

Ausgabe auf dem Computer.

| Mitglied | Beschreibung | Typ / Parameter |
|---|---|---|
| `console.log(values?, …)` | Gibt die Werte auf dem Computer aus (USB-Konsole / Simulator). | `values`: beliebiger Wert<br>→ nichts (`undefined`) |

## Globale Funktionen

| Funktion | Beschreibung | Typ / Parameter |
|---|---|---|
| `wait(time)` | Ein Promise, das nach der angegebenen Zeit fertig ist: `await wait(2s)`. | `time`: eine Zeit<br>→ ein Promise |
| `waitUntil(condition)` | Ein Promise, das fertig ist, sobald die Bedingung `true` ist. | `condition`: eine Funktion<br>→ ein Promise |

## Listen und Texte

| Mitglied | Beschreibung | Typ / Parameter |
|---|---|---|
| `list.length` | Anzahl der Elemente einer Liste oder Zeichen eines Texts. | eine Zahl ohne Einheit |
