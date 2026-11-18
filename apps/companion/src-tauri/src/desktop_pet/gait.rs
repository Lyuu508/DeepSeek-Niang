//! The native window moves in physical pixels; elapsed time comes from a monotonic clock.
use std::time::Duration;

const REST_AFTER_INTERACTION: Duration = Duration::from_millis(1600);
const WALK_BOUT: Duration = Duration::from_millis(5500);
const REST_BOUT: Duration = Duration::from_millis(2700);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pose {
    pub walking: bool,
    pub facing_left: bool,
}

pub struct Gait {
    pub pose: Pose,
    remaining: Duration,
    fraction: f64,
}

impl Default for Gait {
    fn default() -> Self {
        Self {
            pose: Pose {
                walking: false,
                facing_left: true,
            },
            remaining: REST_AFTER_INTERACTION,
            fraction: 0.0,
        }
    }
}

impl Gait {
    pub fn rest(&mut self) {
        self.pose.walking = false;
        self.remaining = REST_AFTER_INTERACTION;
        self.fraction = 0.0;
    }

    pub fn step(&mut self, elapsed: Duration, x: i32, left: i32, right: i32, speed: f64) -> i32 {
        let x = x.clamp(left, right);
        if right.saturating_sub(left) < 4 || speed <= 0.0 {
            self.rest();
            return x;
        }
        // An OS resume or a busy event loop must not cause a jump across the screen.
        let elapsed = elapsed.min(Duration::from_millis(150));
        self.remaining = self.remaining.saturating_sub(elapsed);
        if self.remaining.is_zero() {
            self.pose.walking = !self.pose.walking;
            self.remaining = if self.pose.walking {
                WALK_BOUT
            } else {
                REST_BOUT
            };
            self.fraction = 0.0;
        }
        if !self.pose.walking {
            return x;
        }
        if x == left || x == right {
            self.pose.facing_left = x == right;
        }
        let travel = speed * elapsed.as_secs_f64() + self.fraction;
        let pixels = travel.floor() as i32;
        self.fraction = travel.fract();
        let dx = if self.pose.facing_left {
            -pixels
        } else {
            pixels
        };
        x.saturating_add(dx).clamp(left, right)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walks_and_turns_within_a_monitor_left_of_primary() {
        let mut gait = Gait::default();
        let (left, right) = (-1920, -1780);
        let mut x = right;
        let mut turned = false;
        for _ in 0..150 {
            x = gait.step(Duration::from_millis(40), x, left, right, 52.0);
            assert!((left..=right).contains(&x));
            turned |= !gait.pose.facing_left && gait.pose.walking;
        }
        assert!(turned);
        assert_ne!(x, right);
    }

    #[test]
    fn interaction_pauses_and_resumes_from_the_manual_position() {
        let mut gait = Gait::default();
        let tick = Duration::from_millis(40);
        let mut x = 800;
        for _ in 0..60 {
            x = gait.step(tick, x, 0, 1000, 52.0);
        }
        assert!(gait.pose.walking && x < 800);
        gait.rest();
        for _ in 0..39 {
            assert_eq!(gait.step(tick, 400, 0, 1000, 52.0), 400);
            assert!(!gait.pose.walking);
        }
        assert!(gait.step(tick, 400, 0, 1000, 52.0) < 400);
    }

    #[test]
    fn sleeps_between_bouts_and_never_jumps_on_resume() {
        let mut gait = Gait::default();
        let tick = Duration::from_millis(50);
        let mut x = 1000;
        for _ in 0..40 {
            x = gait.step(tick, x, 0, 1200, 50.0);
        }
        assert!(gait.pose.walking);
        let resumed = gait.step(Duration::from_secs(90), x, 0, 1200, 50.0);
        assert!(x - resumed <= 8);
        for _ in 0..110 {
            x = gait.step(tick, x, 0, 1200, 50.0);
            if !gait.pose.walking {
                break;
            }
        }
        assert!(!gait.pose.walking);
        assert_eq!(gait.step(tick, x, 0, 1200, 50.0), x);
        assert_eq!(gait.step(tick, x, 200, 200, 50.0), 200);
        assert!(!gait.pose.walking);
    }
}
