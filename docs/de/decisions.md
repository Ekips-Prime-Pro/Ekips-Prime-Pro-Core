# Architekturentscheidungen

Kurze Aufzeichnungen von Entscheidungen, die das Projekt prägen. Neueste
zuletzt. Jeder Eintrag sagt, was entschieden wurde und warum.

## D1 – Die Dokumentation liegt in `docs/`, auf Englisch und Deutsch

- **Entscheidung:** Das Wiki besteht aus Markdown-Dateien in `docs/en` und
  `docs/de` mit gleichem Aufbau. `docs/README.md` verlinkt beide.
- **Warum:** Die Schülerinnen und Schüler im Team lesen Deutsch, Mitwirkende
  und KI-Assistenten meist Englisch. Weil die Doku neben dem Code liegt, wird
  sie zusammen mit dem Compiler, den sie beschreibt, versioniert und getaggt.
