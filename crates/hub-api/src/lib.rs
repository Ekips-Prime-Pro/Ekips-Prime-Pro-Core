//! Hardware abstraction for Ekips Prime Pro.
//!
//! The VM never touches real hardware. It talks to a [`Hardware`]
//! implementation, which is either the simulator (on a PC) or the
//! firmware (on the SPIKE Prime hub). Because both sides implement the
//! same trait, a program that works in the simulator works on the hub.
//!
//! **Status:** draft. The trait will change while the VM is written.
//!
//! Conventions used throughout this crate:
//! - Lengths are in millimetres (`f32`).
//! - Angles are in degrees (`f32`). Positive turn angles turn the robot
//!   clockwise (to the right) when seen from above.
//! - Speeds are in percent of the maximum speed, from -100.0 to 100.0.
//! - Time is in milliseconds.

#![no_std]

/// One of the six LPF2 ports of the SPIKE Prime hub.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Port {
    /// Port A
    A,
    /// Port B
    B,
    /// Port C
    C,
    /// Port D
    D,
    /// Port E
    E,
    /// Port F
    F,
}

impl Port {
    /// All ports in order A to F.
    pub const ALL: [Port; 6] = [Port::A, Port::B, Port::C, Port::D, Port::E, Port::F];

    /// Parses a port letter (case-insensitive).
    pub fn from_char(c: char) -> Option<Port> {
        match c.to_ascii_uppercase() {
            'A' => Some(Port::A),
            'B' => Some(Port::B),
            'C' => Some(Port::C),
            'D' => Some(Port::D),
            'E' => Some(Port::E),
            'F' => Some(Port::F),
            _ => None,
        }
    }

    /// Returns the upper-case port letter.
    pub fn as_char(self) -> char {
        match self {
            Port::A => 'A',
            Port::B => 'B',
            Port::C => 'C',
            Port::D => 'D',
            Port::E => 'E',
            Port::F => 'F',
        }
    }

    /// Zero-based index (A = 0, F = 5).
    pub fn index(self) -> usize {
        self as usize
    }
}

/// Colors the SPIKE Prime color sensor can detect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    /// No color detected (nothing in front of the sensor)
    None,
    /// Black
    Black,
    /// Violet
    Violet,
    /// Blue
    Blue,
    /// Azure (light blue)
    Azure,
    /// Green
    Green,
    /// Yellow
    Yellow,
    /// Red
    Red,
    /// White
    White,
}

impl Color {
    /// All colors, in the order of their bytecode encoding.
    pub const ALL: [Color; 9] = [
        Color::None,
        Color::Black,
        Color::Violet,
        Color::Blue,
        Color::Azure,
        Color::Green,
        Color::Yellow,
        Color::Red,
        Color::White,
    ];

    /// Looks up a color by its lower-case name in the language.
    pub fn from_name(name: &str) -> Option<Color> {
        Some(match name {
            "none" => Color::None,
            "black" => Color::Black,
            "violet" => Color::Violet,
            "blue" => Color::Blue,
            "azure" => Color::Azure,
            "green" => Color::Green,
            "yellow" => Color::Yellow,
            "red" => Color::Red,
            "white" => Color::White,
            _ => return None,
        })
    }

    /// The lower-case name used in the language.
    pub fn name(self) -> &'static str {
        match self {
            Color::None => "none",
            Color::Black => "black",
            Color::Violet => "violet",
            Color::Blue => "blue",
            Color::Azure => "azure",
            Color::Green => "green",
            Color::Yellow => "yellow",
            Color::Red => "red",
            Color::White => "white",
        }
    }
}

/// Kind of device that can be plugged into a port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceKind {
    /// Medium or large angular motor
    Motor,
    /// Color sensor
    ColorSensor,
    /// Ultrasonic distance sensor
    DistanceSensor,
    /// Force (touch) sensor
    ForceSensor,
}

/// Geometry of a two-wheeled drive base.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DriveBaseConfig {
    /// Port of the left wheel motor
    pub left: Port,
    /// Port of the right wheel motor
    pub right: Port,
    /// Wheel diameter in mm
    pub wheel_diameter_mm: f32,
    /// Distance between the wheel contact points in mm
    pub axle_track_mm: f32,
}

/// Errors a hardware call can report back to the VM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareError {
    /// No device is plugged into the port.
    NoDevice(Port),
    /// A different device than expected is plugged into the port.
    WrongDevice {
        /// The port in question
        port: Port,
        /// The device the program expected
        expected: DeviceKind,
    },
    /// A drive command was used before the drive base was configured.
    NoDriveBase,
}

/// Interface between the VM and the robot (real or simulated).
///
/// Commands that take time (driving, rotating a motor) only *start* the
/// movement and return immediately. The VM polls the matching `*_done`
/// method and lets other tasks run in the meantime. This keeps the VM
/// free of async code and lets it run on the hub and in the simulator.
pub trait Hardware {
    /// Milliseconds since the program started.
    fn now_ms(&self) -> u64;

    // --- Ports and devices ---

    /// Checks that the expected device is plugged into `port`.
    fn check_device(&self, port: Port, kind: DeviceKind) -> Result<(), HardwareError>;

    // --- Single motors ---

    /// Starts rotating a motor by `degrees` at `speed_pct`.
    fn motor_run_angle(
        &mut self,
        port: Port,
        degrees: f32,
        speed_pct: f32,
    ) -> Result<(), HardwareError>;
    /// Starts rotating a motor continuously at `speed_pct`.
    fn motor_run(&mut self, port: Port, speed_pct: f32) -> Result<(), HardwareError>;
    /// Stops a motor and holds its position.
    fn motor_stop(&mut self, port: Port) -> Result<(), HardwareError>;
    /// Returns `true` once the last `motor_run_angle` has finished.
    fn motor_done(&self, port: Port) -> bool;
    /// Current motor angle in degrees since the last reset.
    fn motor_angle(&self, port: Port) -> Result<f32, HardwareError>;
    /// Current motor speed in percent.
    fn motor_speed(&self, port: Port) -> Result<f32, HardwareError>;
    /// Sets the motor angle counter to zero.
    fn motor_reset_angle(&mut self, port: Port) -> Result<(), HardwareError>;

    // --- Drive base ---

    /// Configures the two-wheeled drive base.
    fn drive_configure(&mut self, config: DriveBaseConfig) -> Result<(), HardwareError>;
    /// Starts driving straight for `distance_mm` (negative = backwards).
    fn drive_straight(&mut self, distance_mm: f32, speed_pct: f32) -> Result<(), HardwareError>;
    /// Starts turning on the spot by `degrees` (positive = clockwise).
    fn drive_turn(&mut self, degrees: f32, speed_pct: f32) -> Result<(), HardwareError>;
    /// Returns `true` once the last drive command has finished.
    fn drive_done(&self) -> bool;
    /// Stops the drive base.
    fn drive_stop(&mut self) -> Result<(), HardwareError>;

    // --- Built-in IMU ---

    /// Heading in degrees since the last reset (clockwise positive).
    fn heading(&self) -> f32;
    /// Sets the heading to zero.
    fn reset_heading(&mut self);

    // --- Sensors ---

    /// Color seen by a color sensor.
    fn color(&self, port: Port) -> Result<Color, HardwareError>;
    /// Reflected light of a color sensor in percent.
    fn reflection(&self, port: Port) -> Result<f32, HardwareError>;
    /// Distance in mm, or `None` if nothing is in range.
    fn distance_mm(&self, port: Port) -> Result<Option<f32>, HardwareError>;
    /// Whether a force sensor is pressed.
    fn pressed(&self, port: Port) -> Result<bool, HardwareError>;
    /// Force on a force sensor in newtons.
    fn force_n(&self, port: Port) -> Result<f32, HardwareError>;

    // --- Hub ---

    /// Shows a short text or number on the 5x5 light matrix.
    fn show(&mut self, text: &str);
    /// Plays a beep with `frequency_hz` for `duration_ms`.
    fn beep(&mut self, frequency_hz: u16, duration_ms: u16);
    /// Battery charge in percent.
    fn battery_pct(&self) -> f32;
    /// Sends a line of text to the computer (USB console / simulator log).
    fn print(&mut self, text: &str);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_round_trip() {
        for port in Port::ALL {
            assert_eq!(Port::from_char(port.as_char()), Some(port));
            assert_eq!(
                Port::from_char(port.as_char().to_ascii_lowercase()),
                Some(port)
            );
        }
        assert_eq!(Port::from_char('G'), None);
        assert_eq!(Port::F.index(), 5);
    }

    #[test]
    fn color_round_trip() {
        for color in Color::ALL {
            assert_eq!(Color::from_name(color.name()), Some(color));
        }
        assert_eq!(Color::from_name("pink"), None);
    }
}
