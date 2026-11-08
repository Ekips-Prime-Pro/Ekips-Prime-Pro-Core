//! Ids of the built-in (native) functions the VM implements.
//!
//! These ids are part of the bytecode ABI: the compiler emits them in
//! `CallNative` instructions and the VM dispatches on them. Names, types and
//! documentation of the functions live in the compiler's standard library
//! table; this enum only fixes the numbers.

macro_rules! natives {
    ($($(#[$doc:meta])* $name:ident = $id:literal,)*) => {
        /// A built-in function, identified by a stable 16-bit id.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(u16)]
        pub enum Native {
            $($(#[$doc])* $name = $id,)*
        }

        impl Native {
            /// All natives in id order.
            pub const ALL: &'static [Native] = &[$(Native::$name,)*];

            /// Decodes a native from its id.
            pub fn from_u16(id: u16) -> Option<Native> {
                match id {
                    $($id => Some(Native::$name),)*
                    _ => None,
                }
            }

            /// The Rust name of the native, for debugging output.
            pub fn name(self) -> &'static str {
                match self {
                    $(Native::$name => stringify!($name),)*
                }
            }
        }
    };
}

natives! {
    /// `new Motor(port)`
    MotorNew = 0,
    /// `new ColorSensor(port)`
    ColorSensorNew = 1,
    /// `new DistanceSensor(port)`
    DistanceSensorNew = 2,
    /// `new ForceSensor(port)`
    ForceSensorNew = 3,
    /// `new DriveBase(left, right, wheelDiameter, axleTrack)`
    DriveBaseNew = 4,

    /// `motor.rotate(angle, speed?)`
    MotorRotate = 16,
    /// `motor.run(speed)`
    MotorRun = 17,
    /// `motor.stop()`
    MotorStop = 18,
    /// `motor.angle`
    MotorAngle = 19,
    /// `motor.speed`
    MotorSpeed = 20,
    /// `motor.resetAngle()`
    MotorResetAngle = 21,

    /// `sensor.color`
    ColorSensorColor = 32,
    /// `sensor.sees(color)`
    ColorSensorSees = 33,
    /// `sensor.reflection`
    ColorSensorReflection = 34,
    /// `sensor.distance`
    DistanceSensorDistance = 35,
    /// `sensor.pressed`
    ForceSensorPressed = 36,
    /// `sensor.force`
    ForceSensorForce = 37,

    /// `robot.drive(distance, speed?)`
    DriveBaseDrive = 48,
    /// `robot.turn(angle, speed?)`
    DriveBaseTurn = 49,
    /// `robot.run(speed)`
    DriveBaseRun = 50,
    /// `robot.stop()`
    DriveBaseStop = 51,

    /// `hub.show(value)`
    HubShow = 64,
    /// `hub.beep(frequency?, duration?)`
    HubBeep = 65,
    /// `hub.heading`
    HubHeading = 66,
    /// `hub.resetHeading()`
    HubResetHeading = 67,
    /// `hub.timer`
    HubTimer = 68,
    /// `hub.resetTimer()`
    HubResetTimer = 69,
    /// `hub.battery`
    HubBattery = 70,

    /// `Math.abs(x)`
    MathAbs = 80,
    /// `Math.min(a, b, …)`
    MathMin = 81,
    /// `Math.max(a, b, …)`
    MathMax = 82,
    /// `Math.round(x)`
    MathRound = 83,
    /// `Math.floor(x)`
    MathFloor = 84,
    /// `Math.ceil(x)`
    MathCeil = 85,
    /// `Math.sqrt(x)`
    MathSqrt = 86,
    /// `Math.random()`
    MathRandom = 87,

    /// `Promise.all(list)`
    PromiseAll = 96,
    /// `Promise.race(list)`
    PromiseRace = 97,
    /// `wait(time)`
    Wait = 98,
    /// `waitUntil(condition)`
    WaitUntil = 99,
    /// `console.log(a, b, …)`
    ConsoleLog = 100,

    /// `list.length` / `text.length`
    Length = 112,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip() {
        for &native in Native::ALL {
            assert_eq!(Native::from_u16(native as u16), Some(native));
        }
        assert_eq!(Native::from_u16(9999), None);
        assert_eq!(Native::DriveBaseDrive.name(), "DriveBaseDrive");
    }
}
