use nnnoiseless::DenoiseState;
use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

static COOLDOWN_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static COOLDOWN_START: std::sync::Mutex<Option<Instant>> = std::sync::Mutex::new(None);

/// VoiceController manages the ffmpeg microphone process lifecycle.
pub struct VoiceController {
    ffmpeg_child: Option<Child>,
    sample_rate: u32,
    api_key: String,
}

impl VoiceController {
    pub fn new(api_key: &str) -> Self {
        VoiceController {
            ffmpeg_child: None,
            sample_rate: 16000,
            api_key: api_key.to_string(),
        }
    }

    pub fn start_mic(&mut self) -> Result<mpsc::Receiver<String>, String> {
        if self.ffmpeg_child.is_some() {
            eprintln!("[VoiceController] Mic already running, stopping first.");
            self.stop_mic();
        }

        let (tx, rx) = mpsc::channel();
        let api_key = self.api_key.clone();
        let sample_rate = self.sample_rate;

        let mut child = get_ffmpeg_command(sample_rate)?;
        let child_stdout = child.stdout.take()
            .ok_or("Failed to get ffmpeg stdout")?;
        self.ffmpeg_child = Some(child);

        thread::spawn(move || {
            run_listener_thread(child_stdout, api_key, sample_rate, tx);
        });

        eprintln!("[VoiceController] Mic started.");
        Ok(rx)
    }

    pub fn stop_mic(&mut self) {
        if let Some(mut child) = self.ffmpeg_child.take() {
            let _ = child.kill();
            let _ = child.wait();
            eprintln!("[VoiceController] Mic stopped.");
        } else {
            eprintln!("[VoiceController] No mic to stop.");
        }
    }

    pub fn restart_mic(&mut self) -> Result<mpsc::Receiver<String>, String> {
        self.stop_mic();
        thread::sleep(Duration::from_millis(500));
        self.start_mic()
    }

    pub fn is_running(&self) -> bool {
        self.ffmpeg_child.is_some()
    }
}

impl Drop for VoiceController {
    fn drop(&mut self) {
        self.stop_mic();
    }
}

pub fn trigger_cooldown() {
    COOLDOWN_ACTIVE.store(true, std::sync::atomic::Ordering::SeqCst);
    if let Ok(mut guard) = COOLDOWN_START.lock() {
        *guard = Some(Instant::now());
    }
}

fn get_ffmpeg_command(sample_rate: u32) -> Result<Child, String> {
    #[cfg(target_os = "macos")]
    {
        Command::new("ffmpeg")
            .args(&[
                "-f", "avfoundation", "-i", ":default",
                "-ac", "1", "-ar", &sample_rate.to_string(),
                "-f", "s16le", "-loglevel", "quiet", "pipe:1",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to start ffmpeg (macOS): {}", e))
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("ffmpeg")
            .args(&[
                "-f", "pulse", "-i", "default",
                "-ac", "1", "-ar", &sample_rate.to_string(),
                "-f", "s16le", "-loglevel", "quiet", "pipe:1",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to start ffmpeg (Linux): {}", e))
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("ffmpeg")
            .args(&[
                "-f", "dshow", "-i", "audio=Microphone",
                "-ac", "1", "-ar", &sample_rate.to_string(),
                "-f", "s16le", "-loglevel", "quiet", "pipe:1",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to start ffmpeg (Windows): {}", e))
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Err("Voice input not supported on this OS".to_string())
    }
}

fn run_listener_thread(
    mut reader: impl Read + Send + 'static,
    api_key: String,
    sample_rate: u32,
    tx: mpsc::Sender<String>,
) {
    let frame_size: usize = 480;
    let silence_timeout_ms: u64 = 1000;
    let min_speech_samples: usize = 8000;

    let mut denoise = DenoiseState::new();
    let mut hpf = HighPassFilter::new();

    let mut speech_buffer: Vec<i16> = Vec::new();
    let mut is_speaking = false;
    let mut silence_start: Option<Instant> = None;
    let mut frame_count: u64 = 0;

    let mut noise_floor: f64 = 80.0;
    let min_rms_for_voice: f64 = 50.0;
    let speech_threshold_factor: f64 = 3.5;
    let speech_threshold_factor_active: f64 = 2.0;

    let mut current_frame: Vec<i16> = Vec::with_capacity(frame_size);
    let mut raw_buf = vec![0u8; 2];

    eprintln!("[Voice] ffmpeg mic pipe started @ {} Hz mono.", sample_rate);

    loop {
        // Cooldown check
        if COOLDOWN_ACTIVE.load(std::sync::atomic::Ordering::SeqCst) {
            if let Ok(guard) = COOLDOWN_START.lock() {
                if let Some(start) = *guard {
                    if start.elapsed().as_secs_f64() > 2.0 {
                        COOLDOWN_ACTIVE.store(false, std::sync::atomic::Ordering::SeqCst);
                        speech_buffer.clear();
                        is_speaking = false;
                        silence_start = None;
                        current_frame.clear();
                        eprintln!("[Voice] Cooldown finished.");
                    } else {
                        current_frame.clear();
                        thread::sleep(Duration::from_millis(50));
                        continue;
                    }
                } else {
                    COOLDOWN_ACTIVE.store(false, std::sync::atomic::Ordering::SeqCst);
                }
            } else {
                COOLDOWN_ACTIVE.store(false, std::sync::atomic::Ordering::SeqCst);
            }
        }

        match reader.read_exact(&mut raw_buf) {
            Ok(_) => {
                let sample = i16::from_le_bytes([raw_buf[0], raw_buf[1]]);
                current_frame.push(sample);

                if current_frame.len() == frame_size {
                    let mut frame = std::mem::replace(&mut current_frame, Vec::with_capacity(frame_size));

                    hpf.process(&mut frame);
                    denoise_frame(&mut denoise, &mut frame);

                    frame_count += 1;

                    let frame_rms = (frame.iter().map(|&s| (s as f64).powi(2)).sum::<f64>()
                        / frame.len() as f64).sqrt();

                    let threshold = if is_speaking { speech_threshold_factor_active } else { speech_threshold_factor };
                    let is_voice = frame_rms > noise_floor * threshold && frame_rms > min_rms_for_voice;

                    if !is_voice {
                        noise_floor = noise_floor * 0.95 + frame_rms * 0.05;
                        if noise_floor < 10.0 { noise_floor = 10.0; }
                    }

                    if frame_count % 30 == 0 {
                        eprintln!(
                            "[Voice] Frame {}: RMS={:.1}, voice={}, speaking={}, noise={:.1}, thr={:.1}",
                            frame_count, frame_rms, is_voice, is_speaking, noise_floor,
                            noise_floor * threshold
                        );
                    }

                    if is_voice {
                        if !is_speaking {
                            is_speaking = true;
                            eprintln!("[Voice] Speech STARTED (RMS: {:.1}, NF: {:.1})", frame_rms, noise_floor);
                            speech_buffer.clear();
                        }
                        speech_buffer.extend_from_slice(&frame);
                        silence_start = None;
                    } else if is_speaking {
                        speech_buffer.extend_from_slice(&frame);
                        if silence_start.is_none() {
                            silence_start = Some(Instant::now());
                        } else if silence_start.unwrap().elapsed().as_millis() as u64 > silence_timeout_ms {
                            is_speaking = false;

                            if speech_buffer.len() < min_speech_samples {
                                eprintln!("[Voice] Speech too short ({} samples), ignoring.", speech_buffer.len());
                                speech_buffer.clear();
                                silence_start = None;
                                continue;
                            }

                            eprintln!("[Voice] Speech ended, transcribing {} samples", speech_buffer.len());
                            if !speech_buffer.is_empty() {
                                normalize_audio(&mut speech_buffer);
                                let wav_data = pcm_to_wav(&speech_buffer, sample_rate);
                                match transcribe_groq(&wav_data, &api_key) {
                                    Ok(text) => {
                                        eprintln!("[Voice] Transcribed: {}", text);
                                        let _ = tx.send(text);
                                    }
                                    Err(e) => eprintln!("[Voice] Transcription error: {}", e),
                                }
                            }
                            speech_buffer.clear();
                            silence_start = None;
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("[Voice] Error reading ffmpeg pipe: {}", e);
                break;
            }
        }
    }
    eprintln!("[Voice] Listener thread exiting.");
}

struct HighPassFilter { prev_x: f64, prev_y: f64 }

impl HighPassFilter {
    fn new() -> Self { Self { prev_x: 0.0, prev_y: 0.0 } }
    fn process(&mut self, frame: &mut [i16]) {
        for sample in frame.iter_mut() {
            let x = *sample as f64;
            let y = x - self.prev_x + 0.999 * self.prev_y;
            self.prev_x = x;
            self.prev_y = y;
            *sample = (y.round() as i16).clamp(-32768, 32767);
        }
    }
}

fn normalize_audio(samples: &mut [i16]) {
    if samples.is_empty() { return; }
    let max_val = samples.iter().map(|&s| s.abs()).max().unwrap_or(1);
    if max_val == 0 { return; }
    let target_peak: f64 = 0.95 * 32768.0;
    let gain = (target_peak / max_val as f64).min(10.0);
    for sample in samples.iter_mut() {
        *sample = (((*sample as f64) * gain).round() as i16).clamp(-32768, 32767);
    }
}

fn denoise_frame(denoise: &mut DenoiseState, frame: &mut [i16]) {
    let input: Vec<f32> = frame.iter().map(|&s| s as f32 / 32767.0).collect();
    let mut output = vec![0.0f32; frame.len()];
    denoise.process_frame(&mut output, &input);
    for (i, &sample_f32) in output.iter().enumerate() {
        frame[i] = (sample_f32 * 32767.0) as i16;
    }
}

fn pcm_to_wav(samples: &[i16], sample_rate: u32) -> Vec<u8> {
    let channels: u16 = 1;
    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * channels as u32 * (bits_per_sample / 8) as u32;
    let block_align = channels * (bits_per_sample / 8);
    let data_size = samples.len() as u32 * (bits_per_sample / 8) as u32;
    let file_size = 36 + data_size;

    let mut wav = Vec::new();
    wav.write_all(b"RIFF").ok();
    wav.write_all(&file_size.to_le_bytes()).ok();
    wav.write_all(b"WAVE").ok();
    wav.write_all(b"fmt ").ok();
    wav.write_all(&16u32.to_le_bytes()).ok();
    wav.write_all(&1u16.to_le_bytes()).ok();
    wav.write_all(&channels.to_le_bytes()).ok();
    wav.write_all(&sample_rate.to_le_bytes()).ok();
    wav.write_all(&byte_rate.to_le_bytes()).ok();
    wav.write_all(&block_align.to_le_bytes()).ok();
    wav.write_all(&bits_per_sample.to_le_bytes()).ok();
    wav.write_all(b"data").ok();
    wav.write_all(&data_size.to_le_bytes()).ok();
    for &sample in samples {
        wav.write_all(&sample.to_le_bytes()).ok();
    }
    wav
}

fn transcribe_groq(wav_data: &[u8], api_key: &str) -> Result<String, String> {
    let boundary = "----boundary_igris_voice";
    let mut body = Vec::new();

    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"file\"; filename=\"speech.wav\"\r\n");
    body.extend_from_slice(b"Content-Type: audio/wav\r\n\r\n");
    body.extend_from_slice(wav_data);
    body.extend_from_slice(b"\r\n");

    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"model\"\r\n\r\n");
    body.extend_from_slice(b"whisper-large-v3\r\n");

    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"response_format\"\r\n\r\n");
    body.extend_from_slice(b"json\r\n");

    body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

    let client = reqwest::blocking::Client::new();
    let response = client
        .post("https://api.groq.com/openai/v1/audio/transcriptions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", format!("multipart/form-data; boundary={}", boundary))
        .body(body)
        .send()
        .map_err(|e| format!("Groq request failed: {}", e))?;

    let text = response.text().map_err(|e| format!("Failed to read response: {}", e))?;

    let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("Failed to parse: {} -> {}", e, text))?;

    if let Some(transcript) = json["text"].as_str() {
        Ok(transcript.to_string())
    } else if let Some(error) = json["error"]["message"].as_str() {
        Err(format!("Groq error: {}", error))
    } else {
        Err(format!("Unexpected response: {}", text))
    }
}
