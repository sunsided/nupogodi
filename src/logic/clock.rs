//! The 12-hour clock with ДП/ПП (AM/PM) and the alarm.

use super::Cue;

const DAY: f64 = 24.0 * 3600.0;
/// The alarm rings for one minute, beeping once a second.
pub const RING_TIME: f32 = 60.0;

#[derive(Clone, Debug)]
pub struct Clock {
    /// Seconds since midnight.
    seconds: f64,
    /// Alarm time, in minutes since midnight.
    pub alarm: u16,
    pub alarm_on: bool,
    /// Remaining ring time while the alarm goes off.
    ringing: Option<f32>,
    last_beep: u32,
}

/// A time of day as the LCD shows it.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Shown {
    /// 1..=12
    pub hour: u8,
    pub minute: u8,
    pub pm: bool,
}

impl Shown {
    pub fn from_minutes(m: u16) -> Self {
        let h = (m / 60) % 24;
        Self {
            hour: match h % 12 {
                0 => 12,
                x => x as u8,
            },
            minute: (m % 60) as u8,
            pm: h >= 12,
        }
    }
}

impl Clock {
    pub fn new(seconds_of_day: u32) -> Self {
        Self {
            seconds: f64::from(seconds_of_day) % DAY,
            alarm: 7 * 60,
            alarm_on: false,
            ringing: None,
            last_beep: 0,
        }
    }

    pub fn minutes(&self) -> u16 {
        (self.seconds / 60.0) as u16
    }

    pub fn now(&self) -> Shown {
        Shown::from_minutes(self.minutes())
    }

    pub fn alarm_time(&self) -> Shown {
        Shown::from_minutes(self.alarm)
    }

    /// The colon blinks with the seconds.
    pub fn colon(&self) -> bool {
        self.seconds.fract() < 0.5
    }

    pub fn is_ringing(&self) -> bool {
        self.ringing.is_some()
    }

    /// Seconds into the ring, for the hare's bell animation.
    pub fn ring_phase(&self) -> f32 {
        self.ringing.map_or(0.0, |r| RING_TIME - r)
    }

    pub fn stop_ringing(&mut self) {
        self.ringing = None;
    }

    pub fn update(&mut self, dt: f32, cues: &mut Vec<Cue>) {
        let before = self.minutes();
        self.seconds = (self.seconds + f64::from(dt)) % DAY;
        let after = self.minutes();
        if before != after && after == self.alarm && self.alarm_on {
            self.ringing = Some(RING_TIME);
            self.last_beep = u32::MAX;
        }
        if let Some(left) = self.ringing.as_mut() {
            *left -= dt;
            let second = (RING_TIME - *left) as u32;
            if *left <= 0.0 {
                self.ringing = None;
            } else if second != self.last_beep {
                self.last_beep = second;
                cues.push(Cue::Alarm);
            }
        }
    }

    pub fn set_hour(&mut self) {
        let m = self.minutes();
        self.seconds = f64::from(next_hour(m)) * 60.0;
    }

    pub fn set_minute(&mut self) {
        let m = self.minutes();
        self.seconds = f64::from(next_minute(m)) * 60.0;
    }

    pub fn toggle_pm(&mut self) {
        let m = self.minutes();
        self.seconds = f64::from((m + 12 * 60) % (24 * 60)) * 60.0;
    }

    pub fn alarm_hour(&mut self) {
        self.alarm = next_hour(self.alarm);
    }

    pub fn alarm_minute(&mut self) {
        self.alarm = next_minute(self.alarm);
    }

    pub fn alarm_toggle_pm(&mut self) {
        self.alarm = (self.alarm + 12 * 60) % (24 * 60);
    }
}

/// Next hour within the same half of the day (11 → 12 → 1 keeps AM/PM).
fn next_hour(m: u16) -> u16 {
    let h = m / 60;
    let half = h / 12 * 12;
    (half + (h % 12 + 1) % 12) * 60 + m % 60
}

fn next_minute(m: u16) -> u16 {
    m / 60 * 60 + (m % 60 + 1) % 60
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twelve_hour_display() {
        assert_eq!(
            Shown::from_minutes(0),
            Shown {
                hour: 12,
                minute: 0,
                pm: false
            }
        );
        assert_eq!(Shown::from_minutes(13 * 60 + 5).hour, 1);
        assert!(Shown::from_minutes(12 * 60).pm);
        assert_eq!(Shown::from_minutes(12 * 60).hour, 12);
    }

    #[test]
    fn wraps_at_midnight() {
        let mut c = Clock::new(24 * 3600 - 1);
        c.update(2.0, &mut Vec::new());
        assert_eq!(c.minutes(), 0);
    }

    #[test]
    fn hour_button_keeps_half_of_day() {
        assert_eq!(next_hour(11 * 60 + 30), 30);
        assert_eq!(next_hour(23 * 60), 12 * 60);
        assert_eq!(next_hour(12 * 60 + 1), 13 * 60 + 1);
        assert_eq!(next_minute(59), 0);
    }

    #[test]
    fn alarm_rings_for_a_minute() {
        let mut c = Clock::new(7 * 3600 - 1);
        c.alarm = 7 * 60;
        c.alarm_on = true;
        let mut cues = Vec::new();
        c.update(1.5, &mut cues);
        assert!(c.is_ringing());
        for _ in 0..120 {
            c.update(0.5, &mut cues);
        }
        assert!(!c.is_ringing());
        let beeps = cues.iter().filter(|&&q| q == Cue::Alarm).count();
        assert!((59..=61).contains(&beeps), "{beeps}");
    }

    #[test]
    fn alarm_off_stays_silent() {
        let mut c = Clock::new(7 * 3600 - 1);
        c.alarm = 7 * 60;
        let mut cues = Vec::new();
        c.update(2.0, &mut cues);
        assert!(!c.is_ringing());
    }
}
