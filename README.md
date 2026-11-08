# Ekips Prime Pro – Core

Language, compiler and shared building blocks for **Ekips Prime Pro**: a
custom firmware and programming language for the LEGO® Education SPIKE™
Prime hub, made for FIRST® LEGO® League teams.

```ts
const robot = new DriveBase(new Motor(Port.A), new Motor(Port.B), 56mm, 112mm);
const eye = new ColorSensor(Port.C);

async function main() {
  await Promise.race([
    robot.drive(100cm, 40%),
    waitUntil(() => eye.sees(Color.Black)),
  ]);
  robot.stop();
  await hub.beep();
}
```

The language looks like TypeScript without types, with units on numbers and
`async`/`await` for doing things at the same time. Error messages are written
for students, in English and German:

```text
Fehler: Das Wort `cosnt` verstehe ich hier nicht.
 --> mission.ekips:1:1
  |
1 | cosnt arm = new Motor(Port.C);
  | ^^^^^
  = Hilfe: Meintest du `const`?
```

## Repositories

| Repository | Contents |
|---|---|
| **Ekips-Prime-Pro-Core** (this one) | Language, compiler, VM, bytecode format, hardware API |
| Ekips-Prime-Pro-Firmware | Embassy firmware for the hub's STM32F413 |
| Ekips-Prime-Pro-Tools | CLI, simulator, language server |
| Ekips-Prime-Pro-VSCode | VS Code extension |

## Crates

| Crate | `no_std` | Status | Purpose |
|---|:---:|---|---|
| [`ekips-hub-api`](crates/hub-api) | ✅ | draft | `Hardware` trait shared by simulator and firmware; ports, colors |
| [`ekips-bytecode`](crates/bytecode) | ✅ | header only | Program container format with version check |
| [`ekips-compiler`](crates/compiler) | – | lexer + parser | Source → syntax tree → (later) bytecode |
| `ekips-vm` | ✅ | planned | Bytecode interpreter with cooperative tasks |
| `ekips-protocol` | ✅ | planned | Messages between computer and hub over USB |

## Try it

```sh
cargo test --workspace
cargo run -p ekips-compiler --example check -- examples/*.ekips
cargo run -p ekips-compiler --example check -- --de --ast examples/square.ekips
```

The documentation is in [`docs/`](docs/README.md) (English and German): language reference in [English](docs/en/language.md) and [German](docs/de/language.md).

## Credits

- [Pybricks](https://github.com/pybricks/pybricks-micropython) – hardware
  details (pin mapping, LUMP device protocol, motor control) and feature
  inspiration. Thank you!
- [spike-rtic](https://github.com/springyworks/spike-rtic) – bare-metal Rust
  firmware for the same hub, used as a reference.

LEGO, SPIKE and FIRST LEGO League are trademarks of their respective owners.
This project is not affiliated with or endorsed by them.

## License

[MIT](LICENSE)
