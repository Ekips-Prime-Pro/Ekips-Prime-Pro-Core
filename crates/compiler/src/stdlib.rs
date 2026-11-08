//! The standard library, described as **one data table**.
//!
//! Everything the compiler knows about built-in names lives in
//! [`GLOBALS`]: which names exist, their parameters with expected units,
//! return types, the native function the VM runs, documentation in
//! English and German, and words students might write instead
//! (`fahre` → `drive`). Semantic analysis, unit checking, code generation,
//! the generated reference (`docs/*/stdlib.md`) and the language server all
//! read this table. Add new built-ins here and nowhere else.

use ekips_bytecode::Native;
use ekips_hub_api::{Color, Port};

use crate::diagnostic::Lang;
use crate::types::{Class, Dim, Ty};

/// A text in English and German.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Doc {
    /// English text
    pub en: &'static str,
    /// German text
    pub de: &'static str,
}

impl Doc {
    /// The text in the given language.
    pub fn get(&self, lang: Lang) -> &'static str {
        match lang {
            Lang::En => self.en,
            Lang::De => self.de,
        }
    }
}

const fn doc(en: &'static str, de: &'static str) -> Doc {
    Doc { en, de }
}

/// What a parameter accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expect {
    /// Any value
    Any,
    /// A number of any kind (all such parameters of one call must have the same kind)
    Number,
    /// Exactly this type (numbers: this kind)
    Is(Ty),
}

/// What a function returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Returns {
    /// Always this type
    Is(Ty),
    /// A number of the same kind as the [`Expect::Number`] arguments (`Math.abs(-3cm)` is a length)
    SameKindAsArgs,
}

/// A parameter of a built-in function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Param {
    /// Name used in docs and messages
    pub name: &'static str,
    /// Accepted values
    pub expect: Expect,
    /// May be left out
    pub optional: bool,
    /// Short description
    pub doc: Doc,
}

/// Parameters, return type and VM entry of a built-in function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Signature {
    /// Parameters in order
    pub params: &'static [Param],
    /// The last parameter may be repeated (`console.log(a, b, c)`)
    pub variadic: bool,
    /// Result type
    pub returns: Returns,
    /// Function the VM runs
    pub native: Native,
}

impl Signature {
    /// Smallest number of arguments.
    pub fn min_args(&self) -> usize {
        self.params.iter().filter(|p| !p.optional).count()
    }

    /// Largest number of arguments (`None` = unlimited).
    pub fn max_args(&self) -> Option<usize> {
        if self.variadic {
            None
        } else {
            Some(self.params.len())
        }
    }

    /// The parameter for argument number `i`.
    pub fn param(&self, i: usize) -> Option<&'static Param> {
        match self.params.get(i) {
            Some(p) => Some(p),
            None if self.variadic => self.params.last(),
            None => None,
        }
    }

    /// `true` if the function returns a Promise.
    pub fn returns_promise(&self) -> bool {
        self.returns == Returns::Is(Ty::Promise)
    }
}

/// A fixed value such as `Port.A`, `Color.Red` or `Math.PI`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConstValue {
    /// A port
    Port(Port),
    /// A color
    Color(Color),
    /// A number in the base unit of its kind
    Num(f64, Dim),
}

impl ConstValue {
    /// The type of the value.
    pub fn ty(self) -> Ty {
        match self {
            ConstValue::Port(_) => Ty::Port,
            ConstValue::Color(_) => Ty::Color,
            ConstValue::Num(_, d) => Ty::Num(d),
        }
    }
}

/// What kind of member something is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemberKind {
    /// A command, called with `()`
    Method(Signature),
    /// A value that is read (`motor.angle`); can not be assigned
    Property {
        /// Type of the value
        ty: Ty,
        /// Getter the VM runs
        native: Native,
    },
    /// A fixed value (`Port.A`)
    Constant(ConstValue),
}

/// A member of a class or namespace (`robot.drive`, `Port.A`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Member {
    /// Name after the `.`
    pub name: &'static str,
    /// Method, property or constant
    pub kind: MemberKind,
    /// Description
    pub doc: Doc,
    /// Words students might write instead (German, other languages)
    pub aliases: &'static [&'static str],
}

/// What a global name is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GlobalKind {
    /// A device class, created with `new`
    Class {
        /// Which class
        class: Class,
        /// Constructor parameters (`returns` is the object)
        ctor: Signature,
        /// Members of every instance
        members: &'static [Member],
    },
    /// A group of members that is used directly (`hub.beep()`, `Port.A`)
    Namespace {
        /// The members
        members: &'static [Member],
    },
    /// A global function (`wait(1s)`)
    Function(Signature),
}

/// A built-in global name.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Global {
    /// The name
    pub name: &'static str,
    /// Class, namespace or function
    pub kind: GlobalKind,
    /// Description
    pub doc: Doc,
    /// Words students might write instead
    pub aliases: &'static [&'static str],
}

impl Global {
    /// Members (of instances for a class, of the namespace otherwise).
    pub fn members(&self) -> &'static [Member] {
        match self.kind {
            GlobalKind::Class { members, .. } | GlobalKind::Namespace { members } => members,
            GlobalKind::Function(_) => &[],
        }
    }
}

// ----------------------------------------------------------------------
// Building blocks
// ----------------------------------------------------------------------

const fn param(name: &'static str, expect: Expect, doc: Doc) -> Param {
    Param {
        name,
        expect,
        optional: false,
        doc,
    }
}

const fn optional(name: &'static str, expect: Expect, doc: Doc) -> Param {
    Param {
        name,
        expect,
        optional: true,
        doc,
    }
}

const fn method(
    name: &'static str,
    params: &'static [Param],
    returns: Ty,
    native: Native,
    doc: Doc,
    aliases: &'static [&'static str],
) -> Member {
    Member {
        name,
        kind: MemberKind::Method(Signature {
            params,
            variadic: false,
            returns: Returns::Is(returns),
            native,
        }),
        doc,
        aliases,
    }
}

const fn property(
    name: &'static str,
    ty: Ty,
    native: Native,
    doc: Doc,
    aliases: &'static [&'static str],
) -> Member {
    Member {
        name,
        kind: MemberKind::Property { ty, native },
        doc,
        aliases,
    }
}

const fn constant(name: &'static str, value: ConstValue, doc: Doc) -> Member {
    Member {
        name,
        kind: MemberKind::Constant(value),
        doc,
        aliases: &[],
    }
}

const fn color(
    name: &'static str,
    value: Color,
    doc: Doc,
    aliases: &'static [&'static str],
) -> Member {
    Member {
        name,
        kind: MemberKind::Constant(ConstValue::Color(value)),
        doc,
        aliases,
    }
}

const LENGTH: Expect = Expect::Is(Ty::Num(Dim::Length));
const ANGLE: Expect = Expect::Is(Ty::Num(Dim::Angle));
const TIME: Expect = Expect::Is(Ty::Num(Dim::Time));
const PERCENT: Expect = Expect::Is(Ty::Num(Dim::Percent));
const PLAIN: Expect = Expect::Is(Ty::Num(Dim::Number));

const SPEED_OPT: Param = optional(
    "speed",
    PERCENT,
    doc(
        "Speed from -100% to 100% (default 50%)",
        "Geschwindigkeit von -100% bis 100% (Standard 50%)",
    ),
);
const SPEED: Param = param(
    "speed",
    PERCENT,
    doc(
        "Speed from -100% to 100%; negative = other direction",
        "Geschwindigkeit von -100% bis 100%; negativ = andere Richtung",
    ),
);
const PORT: Param = param(
    "port",
    Expect::Is(Ty::Port),
    doc(
        "Where the device is plugged in, e.g. `Port.A`",
        "Wo das Gerät eingesteckt ist, z. B. `Port.A`",
    ),
);
const STOP_ALIASES: &[&str] = &["halt", "stopp", "stoppe", "anhalten", "halte"];
const RUN_ALIASES: &[&str] = &["start", "laufe", "starte", "los"];

const fn ctor(params: &'static [Param], class: Class, native: Native) -> Signature {
    Signature {
        params,
        variadic: false,
        returns: Returns::Is(Ty::Object(class)),
        native,
    }
}

// ----------------------------------------------------------------------
// Classes
// ----------------------------------------------------------------------

const MOTOR_MEMBERS: &[Member] = &[
    method(
        "rotate",
        &[
            param(
                "angle",
                ANGLE,
                doc(
                    "How far to rotate (`deg` or `rot`); negative = backwards",
                    "Wie weit gedreht wird (`deg` oder `rot`); negativ = rückwärts",
                ),
            ),
            SPEED_OPT,
        ],
        Ty::Promise,
        Native::MotorRotate,
        doc(
            "Rotates the motor by an angle. Returns a Promise that is done when the motor arrives.",
            "Dreht den Motor um einen Winkel. Gibt ein Promise zurück, das fertig ist, wenn der Motor angekommen ist.",
        ),
        &["drehe", "drehen", "dreh", "turn", "rotiere"],
    ),
    method(
        "run",
        &[SPEED],
        Ty::Nothing,
        Native::MotorRun,
        doc(
            "Starts rotating until `stop()` is called.",
            "Beginnt zu drehen, bis `stop()` aufgerufen wird.",
        ),
        RUN_ALIASES,
    ),
    method(
        "stop",
        &[],
        Ty::Nothing,
        Native::MotorStop,
        doc(
            "Stops the motor and holds its position.",
            "Hält den Motor an und hält seine Position.",
        ),
        STOP_ALIASES,
    ),
    property(
        "angle",
        Ty::Num(Dim::Angle),
        Native::MotorAngle,
        doc(
            "Angle since the last `resetAngle()`.",
            "Winkel seit dem letzten `resetAngle()`.",
        ),
        &["winkel", "position"],
    ),
    property(
        "speed",
        Ty::Num(Dim::Percent),
        Native::MotorSpeed,
        doc("Current speed.", "Aktuelle Geschwindigkeit."),
        &["geschwindigkeit", "tempo"],
    ),
    method(
        "resetAngle",
        &[],
        Ty::Nothing,
        Native::MotorResetAngle,
        doc("Sets `angle` to `0deg`.", "Setzt `angle` auf `0deg`."),
        &["reset", "zuruecksetzen"],
    ),
];

const COLOR_SENSOR_MEMBERS: &[Member] = &[
    property(
        "color",
        Ty::Color,
        Native::ColorSensorColor,
        doc(
            "The color seen right now, e.g. `Color.Red`.",
            "Die Farbe, die gerade gesehen wird, z. B. `Color.Red`.",
        ),
        &["farbe", "colour"],
    ),
    method(
        "sees",
        &[param(
            "color",
            Expect::Is(Ty::Color),
            doc("The color to look for", "Die gesuchte Farbe"),
        )],
        Ty::Bool,
        Native::ColorSensorSees,
        doc(
            "`true` if the sensor sees this color right now.",
            "`true`, wenn der Sensor diese Farbe gerade sieht.",
        ),
        &["sieht", "see", "erkennt"],
    ),
    property(
        "reflection",
        Ty::Num(Dim::Percent),
        Native::ColorSensorReflection,
        doc(
            "Reflected light: `0%` = dark, `100%` = bright.",
            "Reflektiertes Licht: `0%` = dunkel, `100%` = hell.",
        ),
        &["reflexion", "helligkeit", "light"],
    ),
];

const DISTANCE_SENSOR_MEMBERS: &[Member] = &[property(
    "distance",
    Ty::Num(Dim::Length),
    Native::DistanceSensorDistance,
    doc(
        "Distance to the nearest object; `200cm` if nothing is in range.",
        "Abstand zum nächsten Gegenstand; `200cm`, wenn nichts in Reichweite ist.",
    ),
    &["abstand", "entfernung"],
)];

const FORCE_SENSOR_MEMBERS: &[Member] = &[
    property(
        "pressed",
        Ty::Bool,
        Native::ForceSensorPressed,
        doc("`true` while pressed.", "`true`, solange er gedrückt ist."),
        &["gedrueckt", "gedrückt", "isPressed"],
    ),
    property(
        "force",
        Ty::Num(Dim::Number),
        Native::ForceSensorForce,
        doc(
            "Force in newtons (a plain number).",
            "Kraft in Newton (eine Zahl ohne Einheit).",
        ),
        &["kraft"],
    ),
];

const DRIVE_BASE_MEMBERS: &[Member] = &[
    method(
        "drive",
        &[
            param(
                "distance",
                LENGTH,
                doc(
                    "How far to drive; negative = backwards",
                    "Wie weit gefahren wird; negativ = rückwärts",
                ),
            ),
            SPEED_OPT,
        ],
        Ty::Promise,
        Native::DriveBaseDrive,
        doc(
            "Drives straight. Returns a Promise that is done when the robot arrives.",
            "Fährt geradeaus. Gibt ein Promise zurück, das fertig ist, wenn der Roboter angekommen ist.",
        ),
        &["fahre", "fahren", "fahr", "straight", "move", "forward"],
    ),
    method(
        "turn",
        &[
            param(
                "angle",
                ANGLE,
                doc(
                    "How far to turn; positive = right (clockwise)",
                    "Wie weit gedreht wird; positiv = rechts (im Uhrzeigersinn)",
                ),
            ),
            SPEED_OPT,
        ],
        Ty::Promise,
        Native::DriveBaseTurn,
        doc(
            "Turns on the spot. Returns a Promise that is done when the turn is finished.",
            "Dreht auf der Stelle. Gibt ein Promise zurück, das fertig ist, wenn die Drehung beendet ist.",
        ),
        &["drehe", "drehen", "dreh", "rotate", "wende"],
    ),
    method(
        "run",
        &[SPEED],
        Ty::Nothing,
        Native::DriveBaseRun,
        doc(
            "Starts driving straight until `stop()` is called.",
            "Fährt geradeaus los, bis `stop()` aufgerufen wird.",
        ),
        RUN_ALIASES,
    ),
    method(
        "stop",
        &[],
        Ty::Nothing,
        Native::DriveBaseStop,
        doc("Stops both wheels.", "Hält beide Räder an."),
        STOP_ALIASES,
    ),
];

// ----------------------------------------------------------------------
// Namespaces
// ----------------------------------------------------------------------

const HUB_MEMBERS: &[Member] = &[
    method(
        "show",
        &[param(
            "value",
            Expect::Any,
            doc("Number or short text", "Zahl oder kurzer Text"),
        )],
        Ty::Nothing,
        Native::HubShow,
        doc(
            "Shows a number or short text on the light matrix.",
            "Zeigt eine Zahl oder einen kurzen Text auf der Lichtmatrix.",
        ),
        &["zeige", "zeigen", "display", "print", "anzeigen"],
    ),
    method(
        "beep",
        &[
            optional(
                "frequency",
                PLAIN,
                doc(
                    "Pitch in hertz (default 500)",
                    "Tonhöhe in Hertz (Standard 500)",
                ),
            ),
            optional(
                "duration",
                TIME,
                doc("How long (default 200ms)", "Wie lange (Standard 200ms)"),
            ),
        ],
        Ty::Promise,
        Native::HubBeep,
        doc(
            "Beeps. Returns a Promise that is done when the beep is over.",
            "Piept. Gibt ein Promise zurück, das fertig ist, wenn der Ton vorbei ist.",
        ),
        &["piep", "piepe", "piepen", "ton", "sound"],
    ),
    property(
        "heading",
        Ty::Num(Dim::Angle),
        Native::HubHeading,
        doc(
            "Gyro heading since `resetHeading()`; right = positive.",
            "Gyro-Richtung seit `resetHeading()`; rechts = positiv.",
        ),
        &["richtung", "gyro", "angle"],
    ),
    method(
        "resetHeading",
        &[],
        Ty::Nothing,
        Native::HubResetHeading,
        doc("Sets `heading` to `0deg`.", "Setzt `heading` auf `0deg`."),
        &[],
    ),
    property(
        "timer",
        Ty::Num(Dim::Time),
        Native::HubTimer,
        doc(
            "Time since the start or since `resetTimer()`.",
            "Zeit seit dem Start oder seit `resetTimer()`.",
        ),
        &["zeit", "time", "stoppuhr"],
    ),
    method(
        "resetTimer",
        &[],
        Ty::Nothing,
        Native::HubResetTimer,
        doc("Sets `timer` to `0s`.", "Setzt `timer` auf `0s`."),
        &[],
    ),
    property(
        "battery",
        Ty::Num(Dim::Percent),
        Native::HubBattery,
        doc("Battery charge.", "Akkuladung."),
        &["akku", "batterie"],
    ),
];

const PORT_MEMBERS: &[Member] = &[
    constant("A", ConstValue::Port(Port::A), doc("Port A", "Anschluss A")),
    constant("B", ConstValue::Port(Port::B), doc("Port B", "Anschluss B")),
    constant("C", ConstValue::Port(Port::C), doc("Port C", "Anschluss C")),
    constant("D", ConstValue::Port(Port::D), doc("Port D", "Anschluss D")),
    constant("E", ConstValue::Port(Port::E), doc("Port E", "Anschluss E")),
    constant("F", ConstValue::Port(Port::F), doc("Port F", "Anschluss F")),
];

const COLOR_MEMBERS: &[Member] = &[
    color(
        "None",
        Color::None,
        doc(
            "No color (nothing in front of the sensor)",
            "Keine Farbe (nichts vor dem Sensor)",
        ),
        &["Keine", "Nichts", "Nothing"],
    ),
    color("Black", Color::Black, doc("Black", "Schwarz"), &["Schwarz"]),
    color(
        "Violet",
        Color::Violet,
        doc("Violet", "Violett"),
        &["Violett", "Lila", "Purple"],
    ),
    color("Blue", Color::Blue, doc("Blue", "Blau"), &["Blau"]),
    color(
        "Azure",
        Color::Azure,
        doc("Light blue", "Hellblau"),
        &["Hellblau", "Cyan", "LightBlue"],
    ),
    color(
        "Green",
        Color::Green,
        doc("Green", "Grün"),
        &["Gruen", "Grün"],
    ),
    color("Yellow", Color::Yellow, doc("Yellow", "Gelb"), &["Gelb"]),
    color("Red", Color::Red, doc("Red", "Rot"), &["Rot"]),
    color(
        "White",
        Color::White,
        doc("White", "Weiß"),
        &["Weiss", "Weiß"],
    ),
];

const X: Param = param(
    "x",
    Expect::Number,
    doc(
        "A number (with or without unit)",
        "Eine Zahl (mit oder ohne Einheit)",
    ),
);
const VALUES: Param = param(
    "values",
    Expect::Number,
    doc("Numbers of the same kind", "Zahlen der gleichen Art"),
);

const fn math_fn(
    name: &'static str,
    params: &'static [Param],
    variadic: bool,
    returns: Returns,
    native: Native,
    doc: Doc,
) -> Member {
    Member {
        name,
        kind: MemberKind::Method(Signature {
            params,
            variadic,
            returns,
            native,
        }),
        doc,
        aliases: &[],
    }
}

const MATH_MEMBERS: &[Member] = &[
    math_fn(
        "abs",
        &[X],
        false,
        Returns::SameKindAsArgs,
        Native::MathAbs,
        doc(
            "Value without sign: `Math.abs(-3cm)` is `3cm`.",
            "Betrag ohne Vorzeichen: `Math.abs(-3cm)` ist `3cm`.",
        ),
    ),
    math_fn(
        "min",
        &[VALUES, VALUES],
        true,
        Returns::SameKindAsArgs,
        Native::MathMin,
        doc("The smallest of the values.", "Der kleinste der Werte."),
    ),
    math_fn(
        "max",
        &[VALUES, VALUES],
        true,
        Returns::SameKindAsArgs,
        Native::MathMax,
        doc("The largest of the values.", "Der größte der Werte."),
    ),
    math_fn(
        "round",
        &[X],
        false,
        Returns::SameKindAsArgs,
        Native::MathRound,
        doc(
            "Rounds to the nearest whole number.",
            "Rundet auf die nächste ganze Zahl.",
        ),
    ),
    math_fn(
        "floor",
        &[X],
        false,
        Returns::SameKindAsArgs,
        Native::MathFloor,
        doc("Rounds down.", "Rundet ab."),
    ),
    math_fn(
        "ceil",
        &[X],
        false,
        Returns::SameKindAsArgs,
        Native::MathCeil,
        doc("Rounds up.", "Rundet auf."),
    ),
    math_fn(
        "sqrt",
        &[param(
            "x",
            PLAIN,
            doc("A plain number", "Eine Zahl ohne Einheit"),
        )],
        false,
        Returns::Is(Ty::Num(Dim::Number)),
        Native::MathSqrt,
        doc("Square root.", "Quadratwurzel."),
    ),
    math_fn(
        "random",
        &[],
        false,
        Returns::Is(Ty::Num(Dim::Number)),
        Native::MathRandom,
        doc(
            "A random number from 0 (inclusive) to 1 (exclusive).",
            "Eine Zufallszahl von 0 (einschließlich) bis 1 (ausschließlich).",
        ),
    ),
    constant(
        "PI",
        ConstValue::Num(std::f64::consts::PI, Dim::Number),
        doc("The number π (3.14159…).", "Die Zahl π (3,14159…)."),
    ),
];

const PROMISES: Param = param(
    "promises",
    Expect::Is(Ty::List),
    doc(
        "A list of Promises, e.g. `[robot.drive(20cm), arm.rotate(90deg)]`",
        "Eine Liste von Promises, z. B. `[robot.drive(20cm), arm.rotate(90deg)]`",
    ),
);

const PROMISE_MEMBERS: &[Member] = &[
    method(
        "all",
        &[PROMISES],
        Ty::Promise,
        Native::PromiseAll,
        doc(
            "Done when **all** given Promises are done.",
            "Fertig, wenn **alle** angegebenen Promises fertig sind.",
        ),
        &["alle"],
    ),
    method(
        "race",
        &[PROMISES],
        Ty::Promise,
        Native::PromiseRace,
        doc(
            "Done when the **first** of the given Promises is done.",
            "Fertig, wenn das **erste** der angegebenen Promises fertig ist.",
        ),
        &["wettrennen", "first", "any"],
    ),
];

const CONSOLE_MEMBERS: &[Member] = &[Member {
    name: "log",
    kind: MemberKind::Method(Signature {
        params: &[optional(
            "values",
            Expect::Any,
            doc("What to print", "Was ausgegeben wird"),
        )],
        variadic: true,
        returns: Returns::Is(Ty::Nothing),
        native: Native::ConsoleLog,
    }),
    doc: doc(
        "Prints the values on the computer (USB console / simulator).",
        "Gibt die Werte auf dem Computer aus (USB-Konsole / Simulator).",
    ),
    aliases: &["print", "ausgabe", "schreibe", "println"],
}];

/// Members of lists and texts.
pub const LIST_MEMBERS: &[Member] = &[property(
    "length",
    Ty::Num(Dim::Number),
    Native::Length,
    doc(
        "Number of items in a list or characters in a text.",
        "Anzahl der Elemente einer Liste oder Zeichen eines Texts.",
    ),
    &["len", "size", "count", "laenge", "länge"],
)];

// ----------------------------------------------------------------------
// The table
// ----------------------------------------------------------------------

/// All built-in global names.
pub static GLOBALS: &[Global] = &[
    Global {
        name: "Motor",
        kind: GlobalKind::Class {
            class: Class::Motor,
            ctor: ctor(&[PORT], Class::Motor, Native::MotorNew),
            members: MOTOR_MEMBERS,
        },
        doc: doc(
            "A medium or large motor, e.g. for an arm: `new Motor(Port.C)`.",
            "Ein mittlerer oder großer Motor, z. B. für einen Arm: `new Motor(Port.C)`.",
        ),
        aliases: &["motor"],
    },
    Global {
        name: "ColorSensor",
        kind: GlobalKind::Class {
            class: Class::ColorSensor,
            ctor: ctor(&[PORT], Class::ColorSensor, Native::ColorSensorNew),
            members: COLOR_SENSOR_MEMBERS,
        },
        doc: doc(
            "The color sensor: `new ColorSensor(Port.D)`.",
            "Der Farbsensor: `new ColorSensor(Port.D)`.",
        ),
        aliases: &["Farbsensor", "colorSensor", "ColourSensor"],
    },
    Global {
        name: "DistanceSensor",
        kind: GlobalKind::Class {
            class: Class::DistanceSensor,
            ctor: ctor(&[PORT], Class::DistanceSensor, Native::DistanceSensorNew),
            members: DISTANCE_SENSOR_MEMBERS,
        },
        doc: doc(
            "The ultrasonic distance sensor: `new DistanceSensor(Port.E)`.",
            "Der Ultraschall-Abstandssensor: `new DistanceSensor(Port.E)`.",
        ),
        aliases: &["Abstandssensor", "UltrasonicSensor", "distanceSensor"],
    },
    Global {
        name: "ForceSensor",
        kind: GlobalKind::Class {
            class: Class::ForceSensor,
            ctor: ctor(&[PORT], Class::ForceSensor, Native::ForceSensorNew),
            members: FORCE_SENSOR_MEMBERS,
        },
        doc: doc(
            "The force (touch) sensor: `new ForceSensor(Port.F)`.",
            "Der Kraft- bzw. Tastsensor: `new ForceSensor(Port.F)`.",
        ),
        aliases: &["Kraftsensor", "TouchSensor", "Tastsensor", "forceSensor"],
    },
    Global {
        name: "DriveBase",
        kind: GlobalKind::Class {
            class: Class::DriveBase,
            ctor: ctor(
                &[
                    param(
                        "left",
                        Expect::Is(Ty::Object(Class::Motor)),
                        doc("Motor of the left wheel", "Motor des linken Rads"),
                    ),
                    param(
                        "right",
                        Expect::Is(Ty::Object(Class::Motor)),
                        doc("Motor of the right wheel", "Motor des rechten Rads"),
                    ),
                    param(
                        "wheelDiameter",
                        LENGTH,
                        doc(
                            "Diameter of the wheels, e.g. `56mm`",
                            "Durchmesser der Räder, z. B. `56mm`",
                        ),
                    ),
                    param(
                        "axleTrack",
                        LENGTH,
                        doc(
                            "Distance between the two wheels, e.g. `112mm`",
                            "Abstand zwischen den beiden Rädern, z. B. `112mm`",
                        ),
                    ),
                ],
                Class::DriveBase,
                Native::DriveBaseNew,
            ),
            members: DRIVE_BASE_MEMBERS,
        },
        doc: doc(
            "Two wheels that drive the robot: `new DriveBase(left, right, 56mm, 112mm)`.",
            "Zwei Räder, die den Roboter fahren: `new DriveBase(left, right, 56mm, 112mm)`.",
        ),
        aliases: &["Fahrwerk", "Fahrbasis", "Robot", "driveBase", "Drivebase"],
    },
    Global {
        name: "hub",
        kind: GlobalKind::Namespace {
            members: HUB_MEMBERS,
        },
        doc: doc(
            "The SPIKE Prime hub itself: light matrix, speaker, gyro, timer, battery.",
            "Der SPIKE-Prime-Hub selbst: Lichtmatrix, Lautsprecher, Gyro, Timer, Akku.",
        ),
        aliases: &["Hub", "brick"],
    },
    Global {
        name: "Port",
        kind: GlobalKind::Namespace {
            members: PORT_MEMBERS,
        },
        doc: doc(
            "The six ports of the hub: `Port.A` … `Port.F`.",
            "Die sechs Anschlüsse des Hubs: `Port.A` … `Port.F`.",
        ),
        aliases: &["port", "Anschluss", "Ports"],
    },
    Global {
        name: "Color",
        kind: GlobalKind::Namespace {
            members: COLOR_MEMBERS,
        },
        doc: doc(
            "Colors the color sensor can see.",
            "Farben, die der Farbsensor sehen kann.",
        ),
        aliases: &["Farbe", "color", "Colour"],
    },
    Global {
        name: "Math",
        kind: GlobalKind::Namespace {
            members: MATH_MEMBERS,
        },
        doc: doc("Mathematics.", "Mathematik."),
        aliases: &["math", "Mathe"],
    },
    Global {
        name: "Promise",
        kind: GlobalKind::Namespace {
            members: PROMISE_MEMBERS,
        },
        doc: doc(
            "Combine Promises to do several things at the same time.",
            "Promises kombinieren, um mehrere Dinge gleichzeitig zu tun.",
        ),
        aliases: &["promise", "Promises"],
    },
    Global {
        name: "console",
        kind: GlobalKind::Namespace {
            members: CONSOLE_MEMBERS,
        },
        doc: doc("Output on the computer.", "Ausgabe auf dem Computer."),
        aliases: &["Console", "konsole"],
    },
    Global {
        name: "wait",
        kind: GlobalKind::Function(Signature {
            params: &[param(
                "time",
                TIME,
                doc("How long, e.g. `2s`", "Wie lange, z. B. `2s`"),
            )],
            variadic: false,
            returns: Returns::Is(Ty::Promise),
            native: Native::Wait,
        }),
        doc: doc(
            "A Promise that is done after the given time: `await wait(2s)`.",
            "Ein Promise, das nach der angegebenen Zeit fertig ist: `await wait(2s)`.",
        ),
        aliases: &["warte", "sleep", "delay", "pause", "warten"],
    },
    Global {
        name: "waitUntil",
        kind: GlobalKind::Function(Signature {
            params: &[param(
                "condition",
                Expect::Is(Ty::Function),
                doc(
                    "A function that returns `true` when it is time, e.g. `() => bumper.pressed`",
                    "Eine Funktion, die `true` liefert, wenn es so weit ist, z. B. `() => bumper.pressed`",
                ),
            )],
            variadic: false,
            returns: Returns::Is(Ty::Promise),
            native: Native::WaitUntil,
        }),
        doc: doc(
            "A Promise that is done as soon as the condition is `true`.",
            "Ein Promise, das fertig ist, sobald die Bedingung `true` ist.",
        ),
        aliases: &["warteBis", "wait_until", "waitFor", "until"],
    },
];

// ----------------------------------------------------------------------
// Lookups
// ----------------------------------------------------------------------

/// Looks up a global by name.
pub fn global(name: &str) -> Option<&'static Global> {
    GLOBALS.iter().find(|g| g.name == name)
}

/// The global that defines `class`.
pub fn class_global(class: Class) -> &'static Global {
    GLOBALS
        .iter()
        .find(|g| matches!(g.kind, GlobalKind::Class { class: c, .. } if c == class))
        .expect("every class is in the table")
}

/// Members of instances of `class`.
pub fn class_members(class: Class) -> &'static [Member] {
    class_global(class).members()
}

/// Looks up a member by name in a member list.
pub fn find_member(members: &'static [Member], name: &str) -> Option<&'static Member> {
    members.iter().find(|m| m.name == name)
}

/// Member lists of everything a value of unknown type could be:
/// every class and lists/texts.
pub fn instance_member_sets() -> impl Iterator<Item = &'static [Member]> {
    Class::ALL
        .into_iter()
        .map(class_members)
        .chain(std::iter::once(LIST_MEMBERS))
}

/// All instance members called `name`, across all classes (for values of unknown type).
pub fn instance_members_named(name: &str) -> Vec<&'static Member> {
    instance_member_sets()
        .filter_map(|set| find_member(set, name))
        .collect()
}

// ----------------------------------------------------------------------
// Reference documentation
// ----------------------------------------------------------------------

fn expect_text(expect: Expect, lang: Lang) -> String {
    match expect {
        Expect::Any => match lang {
            Lang::En => "any value".into(),
            Lang::De => "beliebiger Wert".into(),
        },
        Expect::Number => match lang {
            Lang::En => "a number (any unit)".into(),
            Lang::De => "eine Zahl (beliebige Einheit)".into(),
        },
        Expect::Is(ty) => ty.describe(lang),
    }
}

fn returns_text(returns: Returns, lang: Lang) -> String {
    match returns {
        Returns::Is(ty) => ty.describe(lang),
        Returns::SameKindAsArgs => match lang {
            Lang::En => "same kind as the input".into(),
            Lang::De => "gleiche Art wie die Eingabe".into(),
        },
    }
}

fn signature_text(prefix: &str, sig: &Signature) -> String {
    let params: Vec<String> = sig
        .params
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let dots = if sig.variadic && i + 1 == sig.params.len() {
                ", …"
            } else {
                ""
            };
            let q = if p.optional { "?" } else { "" };
            format!("{}{q}{dots}", p.name)
        })
        .collect();
    format!("{prefix}({})", params.join(", "))
}

fn write_members(out: &mut String, owner: &str, members: &[Member], lang: Lang) {
    use std::fmt::Write as _;
    let (h_member, h_desc, h_type) = match lang {
        Lang::En => ("Member", "Description", "Type / parameters"),
        Lang::De => ("Mitglied", "Beschreibung", "Typ / Parameter"),
    };
    let _ = writeln!(out, "| {h_member} | {h_desc} | {h_type} |\n|---|---|---|");
    for m in members {
        let name = format!("{owner}.{}", m.name);
        let (code, ty) = match m.kind {
            MemberKind::Method(sig) => {
                let params: Vec<String> = sig
                    .params
                    .iter()
                    .map(|p| format!("`{}`: {}", p.name, expect_text(p.expect, lang)))
                    .collect();
                let arrow = format!("→ {}", returns_text(sig.returns, lang));
                let ty = if params.is_empty() {
                    arrow
                } else {
                    format!("{}<br>{arrow}", params.join("<br>"))
                };
                (signature_text(&name, &sig), ty)
            }
            MemberKind::Property { ty, .. } => (name, ty.describe(lang)),
            MemberKind::Constant(v) => (name, v.ty().describe(lang)),
        };
        let _ = writeln!(out, "| `{code}` | {} | {ty} |", m.doc.get(lang));
    }
    out.push('\n');
}

/// Generates the standard library reference as Markdown.
///
/// The files `docs/en/stdlib.md` and `docs/de/stdlib.md` are produced by
/// this function (`cargo run -p ekips-compiler --example gen-docs`), and a
/// test checks that they are up to date.
pub fn reference_markdown(lang: Lang) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    match lang {
        Lang::En => out.push_str(
            "# Standard library reference\n\n\
             <!-- Generated from crates/compiler/src/stdlib.rs by \
             `cargo run -p ekips-compiler --example gen-docs`. Do not edit. -->\n\n\
             Everything that is built into Ekips Prime Pro. Commands that take time \
             return a **Promise**; use `await` to wait until they are done.\n\n",
        ),
        Lang::De => out.push_str(
            "# Referenz der Standardbibliothek\n\n\
             <!-- Erzeugt aus crates/compiler/src/stdlib.rs mit \
             `cargo run -p ekips-compiler --example gen-docs`. Nicht von Hand ändern. -->\n\n\
             Alles, was in Ekips Prime Pro eingebaut ist. Befehle, die Zeit brauchen, \
             geben ein **Promise** zurück; mit `await` wartest du, bis sie fertig sind.\n\n",
        ),
    }
    let (classes, namespaces, functions) = match lang {
        Lang::En => ("Devices", "Built-in objects", "Global functions"),
        Lang::De => ("Geräte", "Eingebaute Objekte", "Globale Funktionen"),
    };

    let _ = writeln!(out, "## {classes}\n");
    for g in GLOBALS {
        if let GlobalKind::Class {
            ctor,
            members,
            class,
        } = g.kind
        {
            let _ = writeln!(out, "### {}\n\n{}\n", g.name, g.doc.get(lang));
            let _ = writeln!(
                out,
                "```ts\nconst x = new {};\n```\n",
                signature_text(g.name, &ctor)
            );
            let var = match class {
                Class::Motor => "motor",
                Class::DriveBase => "robot",
                _ => "sensor",
            };
            write_members(&mut out, var, members, lang);
        }
    }

    let _ = writeln!(out, "## {namespaces}\n");
    for g in GLOBALS {
        if let GlobalKind::Namespace { members } = g.kind {
            let _ = writeln!(out, "### {}\n\n{}\n", g.name, g.doc.get(lang));
            write_members(&mut out, g.name, members, lang);
        }
    }

    let _ = writeln!(out, "## {functions}\n");
    let (h_fn, h_desc, h_type) = match lang {
        Lang::En => ("Function", "Description", "Type / parameters"),
        Lang::De => ("Funktion", "Beschreibung", "Typ / Parameter"),
    };
    let _ = writeln!(out, "| {h_fn} | {h_desc} | {h_type} |\n|---|---|---|");
    for g in GLOBALS {
        if let GlobalKind::Function(sig) = g.kind {
            let params: Vec<String> = sig
                .params
                .iter()
                .map(|p| format!("`{}`: {}", p.name, expect_text(p.expect, lang)))
                .collect();
            let _ = writeln!(
                out,
                "| `{}` | {} | {}<br>→ {} |",
                signature_text(g.name, &sig),
                g.doc.get(lang),
                params.join("<br>"),
                returns_text(sig.returns, lang)
            );
        }
    }
    out.push('\n');

    let lists = match lang {
        Lang::En => "Lists and texts",
        Lang::De => "Listen und Texte",
    };
    let _ = writeln!(out, "## {lists}\n");
    write_members(&mut out, "list", LIST_MEMBERS, lang);
    out.truncate(out.trim_end().len());
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn names_are_unique() {
        let mut seen = HashSet::new();
        for g in GLOBALS {
            assert!(seen.insert(g.name), "duplicate global {}", g.name);
            let mut members = HashSet::new();
            for m in g.members() {
                assert!(
                    members.insert(m.name),
                    "duplicate member {}.{}",
                    g.name,
                    m.name
                );
            }
        }
    }

    #[test]
    fn every_native_is_used_exactly_once() {
        let mut used = Vec::new();
        for g in GLOBALS {
            match g.kind {
                GlobalKind::Class { ctor, .. } => used.push(ctor.native),
                GlobalKind::Function(sig) => used.push(sig.native),
                GlobalKind::Namespace { .. } => {}
            }
        }
        let all_members = GLOBALS.iter().flat_map(|g| g.members()).chain(LIST_MEMBERS);
        for m in all_members {
            match m.kind {
                MemberKind::Method(sig) => used.push(sig.native),
                MemberKind::Property { native, .. } => used.push(native),
                MemberKind::Constant(_) => {}
            }
        }
        for &native in Native::ALL {
            let count = used.iter().filter(|&&n| n == native).count();
            assert_eq!(count, 1, "{native:?} is used {count} times in the table");
        }
    }

    #[test]
    fn native_receivers_match_the_table() {
        use ekips_bytecode::Receiver;
        let receiver = |class: Class| match class {
            Class::Motor => Receiver::Motor,
            Class::ColorSensor => Receiver::ColorSensor,
            Class::DistanceSensor => Receiver::DistanceSensor,
            Class::ForceSensor => Receiver::ForceSensor,
            Class::DriveBase => Receiver::DriveBase,
        };
        let native_of = |m: &Member| match m.kind {
            MemberKind::Method(sig) => Some(sig.native),
            MemberKind::Property { native, .. } => Some(native),
            MemberKind::Constant(_) => None,
        };
        for g in GLOBALS {
            match g.kind {
                GlobalKind::Class {
                    class,
                    ctor,
                    members,
                } => {
                    assert_eq!(ctor.native.receiver(), None);
                    for m in members {
                        let n = native_of(m).expect("instance members have natives");
                        assert_eq!(n.receiver(), Some(receiver(class)), "{n:?}");
                    }
                }
                GlobalKind::Namespace { members } => {
                    for n in members.iter().filter_map(native_of) {
                        assert_eq!(n.receiver(), None, "{n:?}");
                    }
                }
                GlobalKind::Function(sig) => assert_eq!(sig.native.receiver(), None),
            }
        }
        for m in LIST_MEMBERS {
            assert_eq!(native_of(m).unwrap().receiver(), Some(Receiver::ListOrText));
        }
    }

    #[test]
    fn optional_parameters_come_last() {
        let sigs = GLOBALS
            .iter()
            .flat_map(|g| {
                let own = match g.kind {
                    GlobalKind::Class { ctor, .. } => Some(ctor),
                    GlobalKind::Function(sig) => Some(sig),
                    GlobalKind::Namespace { .. } => None,
                };
                let members = g.members().iter().filter_map(|m| match m.kind {
                    MemberKind::Method(sig) => Some(sig),
                    _ => None,
                });
                own.into_iter().chain(members)
            })
            .collect::<Vec<_>>();
        for sig in sigs {
            let first_optional = sig.params.iter().position(|p| p.optional);
            if let Some(i) = first_optional {
                assert!(sig.params[i..].iter().all(|p| p.optional), "{sig:?}");
            }
        }
    }

    #[test]
    fn lookups() {
        assert!(global("hub").is_some());
        assert!(global("Hub").is_none());
        assert!(find_member(class_members(Class::DriveBase), "drive").is_some());
        assert_eq!(instance_members_named("stop").len(), 2);
        assert_eq!(instance_members_named("length").len(), 1);
    }
}
