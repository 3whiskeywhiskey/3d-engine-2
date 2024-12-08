use std::time::{Duration, Instant};
use std::collections::VecDeque;
use openxr as xr;

const FRAME_HISTORY_SIZE: usize = 120;  // 2 seconds at 60fps

#[derive(Debug, Clone, Copy)]
pub struct FrameTiming {
    pub predicted_display_time: xr::Time,
    pub actual_render_start: Instant,
    pub actual_render_end: Option<Instant>,
    pub frame_index: u64,
}

#[derive(Debug)]
pub struct TimingStats {
    pub average_frame_time_ms: f32,
    pub fps: f32,
    pub frame_time_variance_ms: f32,
    pub max_frame_time_ms: f32,
    pub min_frame_time_ms: f32,
    pub dropped_frames: u32,
}

pub struct FrameTimingManager {
    frame_history: VecDeque<FrameTiming>,
    current_frame: Option<FrameTiming>,
    frame_counter: u64,
    last_stats_update: Instant,
    last_stats: TimingStats,
    target_frame_time: Duration,
}

impl FrameTimingManager {
    pub fn new(target_fps: u32) -> Self {
        Self {
            frame_history: VecDeque::with_capacity(FRAME_HISTORY_SIZE),
            current_frame: None,
            frame_counter: 0,
            last_stats_update: Instant::now(),
            last_stats: TimingStats {
                average_frame_time_ms: 0.0,
                fps: 0.0,
                frame_time_variance_ms: 0.0,
                max_frame_time_ms: 0.0,
                min_frame_time_ms: f32::MAX,
                dropped_frames: 0,
            },
            target_frame_time: Duration::from_secs_f32(1.0 / target_fps as f32),
        }
    }

    pub fn begin_frame(&mut self, predicted_display_time: xr::Time) {
        let frame = FrameTiming {
            predicted_display_time,
            actual_render_start: Instant::now(),
            actual_render_end: None,
            frame_index: self.frame_counter,
        };
        self.current_frame = Some(frame);
    }

    pub fn end_frame(&mut self) {
        if let Some(mut frame) = self.current_frame.take() {
            frame.actual_render_end = Some(Instant::now());
            
            // Add to history, removing oldest if at capacity
            if self.frame_history.len() >= FRAME_HISTORY_SIZE {
                self.frame_history.pop_front();
            }
            self.frame_history.push_back(frame);
            self.frame_counter += 1;

            // Update stats if enough time has passed
            if self.last_stats_update.elapsed() >= Duration::from_secs(1) {
                self.update_stats();
            }
        }
    }

    pub fn get_stats(&self) -> &TimingStats {
        &self.last_stats
    }

    pub fn force_stats_update(&mut self) {
        self.update_stats();
    }

    fn update_stats(&mut self) {
        let mut total_frame_time = Duration::ZERO;
        let mut max_frame_time = Duration::ZERO;
        let mut min_frame_time = Duration::MAX;
        let mut frame_times = Vec::new();
        let mut dropped = 0;

        // Calculate basic stats
        for frame in &self.frame_history {
            if let Some(end_time) = frame.actual_render_end {
                let frame_time = end_time - frame.actual_render_start;
                total_frame_time += frame_time;
                max_frame_time = max_frame_time.max(frame_time);
                min_frame_time = min_frame_time.min(frame_time);
                frame_times.push(frame_time);

                // Count dropped frames (frames that took longer than target frame time)
                if frame_time > self.target_frame_time {
                    dropped += 1;
                }
            }
        }

        let frame_count = frame_times.len();
        if frame_count > 0 {
            let average_frame_time = total_frame_time / frame_count as u32;
            
            // Calculate variance
            let avg_frame_time_secs = average_frame_time.as_secs_f32();
            let variance: f32 = frame_times.iter()
                .map(|t| {
                    let diff = t.as_secs_f32() - avg_frame_time_secs;
                    diff * diff
                })
                .sum::<f32>() / frame_count as f32;

            self.last_stats = TimingStats {
                average_frame_time_ms: average_frame_time.as_secs_f32() * 1000.0,
                fps: 1.0 / average_frame_time.as_secs_f32(),
                frame_time_variance_ms: (variance * 1000.0 * 1000.0).sqrt(),
                max_frame_time_ms: max_frame_time.as_secs_f32() * 1000.0,
                min_frame_time_ms: min_frame_time.as_secs_f32() * 1000.0,
                dropped_frames: dropped,
            };
        }

        self.last_stats_update = Instant::now();
    }

    pub fn predict_next_frame_time(&self) -> Option<xr::Time> {
        // If we have a current frame, predict based on that
        if let Some(current) = &self.current_frame {
            return Some(xr::Time::from_nanos(
                current.predicted_display_time.as_nanos() + self.target_frame_time.as_nanos() as i64
            ));
        }

        // Otherwise, if we have history, predict based on last frame
        self.frame_history.back().map(|last_frame| {
            xr::Time::from_nanos(
                last_frame.predicted_display_time.as_nanos() + self.target_frame_time.as_nanos() as i64
            )
        })
    }

    pub fn get_frame_to_photon_latency(&self) -> Option<Duration> {
        self.frame_history.back().and_then(|frame| {
            frame.actual_render_end.map(|end| {
                let start = frame.actual_render_start;
                let display_time = Duration::from_nanos(frame.predicted_display_time.as_nanos() as u64);
                display_time + (end - start)
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_frame_timing_basic() {
        let mut manager = FrameTimingManager::new(90); // 90 Hz VR
        
        // Simulate a few frames
        for i in 0..5 {
            let display_time = xr::Time::from_nanos(i * 11_111_111); // ~90Hz
            manager.begin_frame(display_time);
            thread::sleep(Duration::from_millis(10)); // Simulate work
            manager.end_frame();
        }

        // Force stats update for testing
        manager.force_stats_update();

        let stats = manager.get_stats();
        assert!(stats.fps > 0.0, "FPS should be greater than 0");
        assert!(stats.average_frame_time_ms > 0.0, "Average frame time should be greater than 0");
        assert_eq!(manager.frame_counter, 5, "Should have counted 5 frames");
    }

    #[test]
    fn test_frame_timing_prediction() {
        let mut manager = FrameTimingManager::new(90);
        
        // Start a frame
        let initial_time = xr::Time::from_nanos(1_000_000_000); // 1 second
        manager.begin_frame(initial_time);
        manager.end_frame();

        // Predict next frame
        let predicted = manager.predict_next_frame_time().unwrap();
        assert!(predicted.as_nanos() > initial_time.as_nanos(), 
            "Predicted time should be after initial time");
        
        // The difference should be approximately 1/90 second
        let diff_nanos = predicted.as_nanos() - initial_time.as_nanos();
        let expected_nanos = (1.0 / 90.0 * 1_000_000_000.0) as i64;
        let tolerance = 1_000_000; // 1ms tolerance
        assert!((diff_nanos - expected_nanos).abs() < tolerance,
            "Prediction should be approximately 1/90 second in the future");
    }

    #[test]
    fn test_frame_timing_stats() {
        let mut manager = FrameTimingManager::new(90);
        
        // Simulate frames with varying timings
        let timings = [5, 10, 15, 10, 5]; // milliseconds
        for (i, &ms) in timings.iter().enumerate() {
            let display_time = xr::Time::from_nanos(i as i64 * 11_111_111);
            manager.begin_frame(display_time);
            thread::sleep(Duration::from_millis(ms));
            manager.end_frame();
        }

        // Force stats update for testing
        manager.force_stats_update();

        let stats = manager.get_stats();
        assert!(stats.frame_time_variance_ms > 0.0, "Should have non-zero variance with varying frame times");
        assert!(stats.max_frame_time_ms >= stats.average_frame_time_ms, 
            "Max frame time should be >= average");
        assert!(stats.min_frame_time_ms <= stats.average_frame_time_ms,
            "Min frame time should be <= average");
    }
} 