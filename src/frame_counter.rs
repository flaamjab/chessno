use std::time::Instant;

pub struct FrameCounter {
    average_frame_time: f64,
    frame_count: usize,
    start_time: Instant,
}

impl FrameCounter {
    pub fn new() -> Self {
        Self {
            average_frame_time: 0.0,
            frame_count: 0,
            start_time: Instant::now(),
        }
    }

    pub fn end_frame(&mut self) {
        self.average_frame_time = self.start_time.elapsed().as_secs_f64() / self.frame_count as f64;
        self.frame_count += 1;
    }

    pub fn secs_per_frame(&self) -> f64 {
        self.average_frame_time
    }

    pub fn framerate(&self) -> f64 {
        1.0 / self.average_frame_time
    }
}
