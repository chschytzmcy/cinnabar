pub struct VadDetector {
    threshold: f32,
}

impl VadDetector {
    pub fn new(threshold: f32) -> Self {
        Self { threshold }
    }

    pub fn is_speech(&self, samples: &[f32]) -> bool {
        let energy: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
        energy > self.threshold
    }
}

pub struct EndpointDetector {
    vad: VadDetector,
    sample_rate: u32,
    min_silence_duration: f32,
    min_speech_duration: f32,
    silence_samples: u32,
    speech_samples: u32,
}

impl EndpointDetector {
    pub fn new(
        vad_threshold: f32,
        sample_rate: u32,
        min_silence_duration: f32,
        min_speech_duration: f32,
    ) -> Self {
        Self {
            vad: VadDetector::new(vad_threshold),
            sample_rate,
            min_silence_duration,
            min_speech_duration,
            silence_samples: 0,
            speech_samples: 0,
        }
    }

    pub fn accept_waveform(&mut self, samples: &[f32]) -> bool {
        let is_speech = self.vad.is_speech(samples);

        if is_speech {
            self.speech_samples += samples.len() as u32;
            self.silence_samples = 0;
        } else {
            self.silence_samples += samples.len() as u32;
        }

        self.is_endpoint()
    }

    pub fn is_endpoint(&self) -> bool {
        let silence_duration = self.silence_samples as f32 / self.sample_rate as f32;
        let speech_duration = self.speech_samples as f32 / self.sample_rate as f32;

        speech_duration >= self.min_speech_duration && silence_duration >= self.min_silence_duration
    }

    pub fn reset(&mut self) {
        self.silence_samples = 0;
        self.speech_samples = 0;
    }

    /// 当前已采样的采样率。日志里用它把 samples 数转毫秒。
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// 累计的语音样本数（reset 后从 0 累加）。
    pub fn speech_samples(&self) -> u32 {
        self.speech_samples
    }

    /// 累计的静音样本数（reset 后从 0 累加）。
    pub fn silence_samples(&self) -> u32 {
        self.silence_samples
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vad_silence() {
        let vad = VadDetector::new(0.01);
        let silence = vec![0.0; 100];
        assert!(!vad.is_speech(&silence));
    }

    #[test]
    fn test_vad_speech() {
        let vad = VadDetector::new(0.01);
        let speech: Vec<f32> = (0..100).map(|i| (i as f32 * 0.1).sin()).collect();
        assert!(vad.is_speech(&speech));
    }

    #[test]
    fn test_vad_threshold() {
        let vad = VadDetector::new(0.5);
        let low_energy = vec![0.1; 100];
        assert!(!vad.is_speech(&low_energy));
    }

    #[test]
    fn test_endpoint_detector() {
        let mut detector = EndpointDetector::new(0.01, 16000, 1.0, 0.5);

        let speech: Vec<f32> = (0..8000).map(|i| (i as f32 * 0.1).sin()).collect();
        assert!(!detector.accept_waveform(&speech));

        let silence = vec![0.0; 16000];
        assert!(detector.accept_waveform(&silence));
    }

    #[test]
    fn test_endpoint_reset() {
        let mut detector = EndpointDetector::new(0.01, 16000, 1.0, 0.5);
        let speech: Vec<f32> = (0..8000).map(|i| (i as f32 * 0.1).sin()).collect();
        detector.accept_waveform(&speech);
        detector.reset();
        assert!(!detector.is_endpoint());
    }
}
