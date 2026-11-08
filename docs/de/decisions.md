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
