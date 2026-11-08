//! A simulated hub for tests and examples.
//!
//! [`MockHub`] implements [`Hardware`] with a simulated clock: nothing
//! happens until [`MockHub::advance`] moves time forward. Movements take
//! as long as they would on a robot (100% speed = 1000 degrees per second
//! at the wheel or motor). Every hardware call is recorded in
//! [`MockHub::log`] as readable text. Sensor values can be set directly or
//! scheduled with [`MockHub::script`].

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use ekips_hub_api::{Color, DeviceKind, DriveBaseConfig, Hardware, HardwareError, Port};

/// One recorded hardware call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// Simulated time in ms
    pub time_ms: u64,
    /// What happened, e.g. `drive: straight 20cm at 50%`
    pub text: String,
}

/// A sensor change scheduled with [`MockHub::script`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Change {
    /// The color sensor at the port sees this color
    Color(Port, Color),
    /// The color sensor at the port measures this reflection (%)
    Reflection(Port, f32),
    /// The distance sensor at the port measures this distance (mm, `None` = nothing in range)
    Distance(Port, Option<f32>),
    /// The force sensor at the port is pressed or released
    Pressed(Port, bool),
}

#[derive(Debug, Clone, Copy, Default)]
struct Motor {
    angle: f32,
    speed: f32,
    /// Movement from `rotate`: (start time, start angle, degrees, duration)
    moving: Option<(u64, f32, f32, u64)>,
    running: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct Sensor {
    color: Option<Color>,
    reflection: f32,
    distance: Option<f32>,
    pressed: bool,
}

/// A simulated SPIKE Prime hub.
#[derive(Debug, Clone)]
pub struct MockHub {
    now: u64,
    devices: [Option<DeviceKind>; 6],
    accept_any_device: bool,
    motors: [Motor; 6],
    sensors: [Sensor; 6],
    drive: Option<DriveBaseConfig>,
    drive_until: u64,
    /// (end time, heading change) of a running turn
    turn: Option<(u64, f32)>,
    heading: f32,
    battery: f32,
    script: Vec<(u64, Change)>,
    /// Everything that happened, in order
    pub log: Vec<Event>,
}

impl Default for MockHub {
    fn default() -> Self {
        MockHub::new()
    }
}

/// Duration of a movement of `degrees` at `speed_pct` (100% = 1 degree per ms).
fn duration_ms(degrees: f32, speed_pct: f32) -> u64 {
    let speed = libm::fabsf(speed_pct);
    if speed < 0.01 {
        return u64::MAX / 4;
    }
    libm::ceilf(libm::fabsf(degrees) * 100.0 / speed) as u64
}

fn cm(mm: f32) -> String {
    format!("{}cm", mm / 10.0)
}

impl MockHub {
    /// A hub where every device the program asks for is plugged in.
    pub fn new() -> MockHub {
        MockHub {
            now: 0,
            devices: [None; 6],
            accept_any_device: true,
            motors: [Motor::default(); 6],
            sensors: [Sensor::default(); 6],
            drive: None,
            drive_until: 0,
            turn: None,
            heading: 0.0,
            battery: 100.0,
            script: Vec::new(),
            log: Vec::new(),
        }
    }

    /// A hub with exactly these devices plugged in.
    pub fn with_devices(devices: &[(Port, DeviceKind)]) -> MockHub {
        let mut hub = MockHub::new();
        hub.accept_any_device = false;
        for &(port, kind) in devices {
            hub.devices[port.index()] = Some(kind);
        }
        hub
    }

    /// Current simulated time in ms.
    pub fn now(&self) -> u64 {
        self.now
    }

    /// Moves simulated time forward.
    pub fn advance(&mut self, ms: u64) {
        let target = self.now + ms;
        // Apply scripted changes in order.
        self.script.sort_by_key(|(t, _)| *t);
        while let Some(&(t, change)) = self.script.first() {
            if t > target {
                break;
            }
            self.script.remove(0);
            self.now = self.now.max(t);
            self.apply(change);
        }
        self.now = target;
        for m in &mut self.motors {
            if let Some((start, from, degrees, duration)) = m.moving {
                if self.now >= start + duration {
                    m.angle = from + degrees;
                    m.speed = 0.0;
                    m.moving = None;
                } else {
                    let done = (self.now - start) as f32 / duration as f32;
                    m.angle = from + degrees * done;
                }
            }
        }
        if let Some((end, change)) = self.turn {
            if self.now >= end {
                self.heading += change;
                self.turn = None;
            }
        }
    }

    /// Schedules a sensor change at `time_ms`.
    pub fn script(&mut self, time_ms: u64, change: Change) {
        self.script.push((time_ms, change));
    }

    /// Changes a sensor value right now.
    pub fn set(&mut self, change: Change) {
        self.apply(change);
    }

    /// Sets the battery charge in percent.
    pub fn set_battery(&mut self, pct: f32) {
        self.battery = pct;
    }

    /// The log as lines `[  1.230s] text`.
    pub fn log_lines(&self) -> Vec<String> {
        self.log
            .iter()
            .map(|e| {
                format!(
                    "[{:3}.{:03}s] {}",
                    e.time_ms / 1000,
                    e.time_ms % 1000,
                    e.text
                )
            })
            .collect()
    }

    fn apply(&mut self, change: Change) {
        match change {
            Change::Color(p, c) => self.sensors[p.index()].color = Some(c),
            Change::Reflection(p, r) => self.sensors[p.index()].reflection = r,
            Change::Distance(p, d) => self.sensors[p.index()].distance = d,
            Change::Pressed(p, b) => self.sensors[p.index()].pressed = b,
        }
    }

    fn record(&mut self, text: String) {
        self.log.push(Event {
            time_ms: self.now,
            text,
        });
    }

    fn device(&self, port: Port, kind: DeviceKind) -> Result<(), HardwareError> {
        if self.accept_any_device {
            return Ok(());
        }
        match self.devices[port.index()] {
            None => Err(HardwareError::NoDevice(port)),
            Some(k) if k == kind => Ok(()),
            Some(_) => Err(HardwareError::WrongDevice {
                port,
                expected: kind,
            }),
        }
    }

    fn drive_config(&self) -> Result<DriveBaseConfig, HardwareError> {
        self.drive.ok_or(HardwareError::NoDriveBase)
    }
}

impl Hardware for MockHub {
    fn now_ms(&self) -> u64 {
        self.now
    }

    fn check_device(&self, port: Port, kind: DeviceKind) -> Result<(), HardwareError> {
        self.device(port, kind)
    }

    fn motor_run_angle(
        &mut self,
        port: Port,
        degrees: f32,
        speed_pct: f32,
    ) -> Result<(), HardwareError> {
        self.device(port, DeviceKind::Motor)?;
        let m = &mut self.motors[port.index()];
        m.moving = Some((self.now, m.angle, degrees, duration_ms(degrees, speed_pct)));
        m.running = false;
        m.speed = speed_pct;
        self.record(format!(
            "motor {}: rotate {degrees}deg at {speed_pct}%",
            port.as_char()
        ));
        Ok(())
    }

    fn motor_run(&mut self, port: Port, speed_pct: f32) -> Result<(), HardwareError> {
        self.device(port, DeviceKind::Motor)?;
        let m = &mut self.motors[port.index()];
        m.moving = None;
        m.running = true;
        m.speed = speed_pct;
        self.record(format!("motor {}: run at {speed_pct}%", port.as_char()));
        Ok(())
    }

    fn motor_stop(&mut self, port: Port) -> Result<(), HardwareError> {
        self.device(port, DeviceKind::Motor)?;
        let m = &mut self.motors[port.index()];
        m.moving = None;
        m.running = false;
        m.speed = 0.0;
        self.record(format!("motor {}: stop", port.as_char()));
        Ok(())
    }

    fn motor_done(&self, port: Port) -> bool {
        let m = &self.motors[port.index()];
        m.moving.is_none()
    }

    fn motor_angle(&self, port: Port) -> Result<f32, HardwareError> {
        self.device(port, DeviceKind::Motor)?;
        Ok(self.motors[port.index()].angle)
    }

    fn motor_speed(&self, port: Port) -> Result<f32, HardwareError> {
        self.device(port, DeviceKind::Motor)?;
        Ok(self.motors[port.index()].speed)
    }

    fn motor_reset_angle(&mut self, port: Port) -> Result<(), HardwareError> {
        self.device(port, DeviceKind::Motor)?;
        self.motors[port.index()].angle = 0.0;
        self.record(format!("motor {}: reset angle", port.as_char()));
        Ok(())
    }

    fn drive_configure(&mut self, config: DriveBaseConfig) -> Result<(), HardwareError> {
        self.device(config.left, DeviceKind::Motor)?;
        self.device(config.right, DeviceKind::Motor)?;
        self.drive = Some(config);
        self.record(format!(
            "drive: configure left {} right {} wheel {} axle {}",
            config.left.as_char(),
            config.right.as_char(),
            cm(config.wheel_diameter_mm),
            cm(config.axle_track_mm)
        ));
        Ok(())
    }

    fn drive_straight(&mut self, distance_mm: f32, speed_pct: f32) -> Result<(), HardwareError> {
        let c = self.drive_config()?;
        let wheel_degrees = distance_mm / (core::f32::consts::PI * c.wheel_diameter_mm) * 360.0;
        self.drive_until = self.now + duration_ms(wheel_degrees, speed_pct);
        self.turn = None;
        self.record(format!(
            "drive: straight {} at {speed_pct}%",
            cm(distance_mm)
        ));
        Ok(())
    }

    fn drive_turn(&mut self, degrees: f32, speed_pct: f32) -> Result<(), HardwareError> {
        let c = self.drive_config()?;
        let wheel_degrees = c.axle_track_mm * degrees / c.wheel_diameter_mm;
        let duration = duration_ms(wheel_degrees, speed_pct);
        self.drive_until = self.now + duration;
        self.turn = Some((self.drive_until, degrees));
        self.record(format!("drive: turn {degrees}deg at {speed_pct}%"));
        Ok(())
    }

    fn drive_done(&self) -> bool {
        self.now >= self.drive_until
    }

    fn drive_stop(&mut self) -> Result<(), HardwareError> {
        self.drive_config()?;
        self.drive_until = self.now;
        self.turn = None;
        self.record("drive: stop".into());
        Ok(())
    }

    fn heading(&self) -> f32 {
        self.heading
    }

    fn reset_heading(&mut self) {
        self.heading = 0.0;
        self.record("hub: reset heading".into());
    }

    fn color(&self, port: Port) -> Result<Color, HardwareError> {
        self.device(port, DeviceKind::ColorSensor)?;
        Ok(self.sensors[port.index()].color.unwrap_or(Color::None))
    }

    fn reflection(&self, port: Port) -> Result<f32, HardwareError> {
        self.device(port, DeviceKind::ColorSensor)?;
        Ok(self.sensors[port.index()].reflection)
    }

    fn distance_mm(&self, port: Port) -> Result<Option<f32>, HardwareError> {
        self.device(port, DeviceKind::DistanceSensor)?;
        Ok(self.sensors[port.index()].distance)
    }

    fn pressed(&self, port: Port) -> Result<bool, HardwareError> {
        self.device(port, DeviceKind::ForceSensor)?;
        Ok(self.sensors[port.index()].pressed)
    }

    fn force_n(&self, port: Port) -> Result<f32, HardwareError> {
        self.device(port, DeviceKind::ForceSensor)?;
        Ok(if self.sensors[port.index()].pressed {
            5.0
        } else {
            0.0
        })
    }

    fn show(&mut self, text: &str) {
        self.record(format!("hub: show \"{text}\""));
    }

    fn beep(&mut self, frequency_hz: u16, duration_ms: u16) {
        self.record(format!("hub: beep {frequency_hz} Hz for {duration_ms}ms"));
    }

    fn battery_pct(&self) -> f32 {
        self.battery
    }

    fn print(&mut self, text: &str) {
        self.record(format!("console: {text}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movements_take_time() {
        let mut hub = MockHub::new();
        hub.motor_run_angle(Port::C, 90.0, 50.0).unwrap();
        assert!(!hub.motor_done(Port::C));
        hub.advance(179);
        assert!(!hub.motor_done(Port::C));
        hub.advance(1);
        assert!(hub.motor_done(Port::C));
        assert_eq!(hub.motor_angle(Port::C).unwrap(), 90.0);
    }

    #[test]
    fn devices_are_checked() {
        let hub = MockHub::with_devices(&[(Port::A, DeviceKind::Motor)]);
        assert!(hub.check_device(Port::A, DeviceKind::Motor).is_ok());
        assert_eq!(
            hub.check_device(Port::B, DeviceKind::Motor),
            Err(HardwareError::NoDevice(Port::B))
        );
        assert!(matches!(
            hub.check_device(Port::A, DeviceKind::ColorSensor),
            Err(HardwareError::WrongDevice { .. })
        ));
    }

    #[test]
    fn scripted_sensor_changes() {
        let mut hub = MockHub::new();
        hub.script(100, Change::Color(Port::D, Color::Black));
        assert_eq!(hub.color(Port::D).unwrap(), Color::None);
        hub.advance(150);
        assert_eq!(hub.color(Port::D).unwrap(), Color::Black);
    }
}
