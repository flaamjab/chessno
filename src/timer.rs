use std::time::Instant;

pub struct Timer {
    set_time: Instant,
}

impl Timer {
    pub fn new() -> Self {
        Timer {
            set_time: Instant::now(),
        }
    }

    pub fn reset(&mut self) {
        self.set_time = Instant::now();
    }

    pub fn elapsed(&self) -> f32 {
        self.set_time.elapsed().as_secs_f32()
    }
}
