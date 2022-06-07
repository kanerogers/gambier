use std::time::{Duration, Instant};

pub struct Timer {
    fps_timer: Duration,
    last_frame_time: Instant,
    frames: usize,
    delta_time: Duration,
    total_time: Duration,
}

impl Timer {
    pub fn tick(&mut self) {
        self.delta_time = Instant::now().duration_since(self.last_frame_time);
        self.total_time += self.delta_time;

        // Advance FPS timer
        if self.fps_timer.as_secs_f32() >= 1.0 {
            self.fps_timer = Default::default();
            self.print();
            self.frames = 0;
            self.last_frame_time = Instant::now();
        } else {
            self.frames += 1;
            self.fps_timer += self.delta_time;
            self.last_frame_time = Instant::now();
        }
    }

    fn print(&self) {
        println!("FPS {}", self.frames);
    }

    pub fn delta(&self) -> f32 {
        self.delta_time.as_secs_f32()
    }
    pub fn time(&self) -> f32 {
        self.total_time.as_secs_f32()
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self {
            fps_timer: Default::default(),
            last_frame_time: Instant::now(),
            frames: Default::default(),
            delta_time: Default::default(),
            total_time: Default::default(),
        }
    }
}
