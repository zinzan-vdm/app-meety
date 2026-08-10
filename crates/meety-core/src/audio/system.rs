     1|use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
     2|use std::sync::Arc;
     3|use std::time::{SystemTime, UNIX_EPOCH};
     4|
     5|use parking_lot::Mutex;
     6|use tracing::{debug, error, info, warn};
     7|
     8|use crate::audio::resampler::StreamingResampler;
     9|use crate::audio::sys_audio::SystemAudioCapture;
    10|use crate::audio::wav_writer::AudioWavWriter;
    11|use crate::error::{MeetyError, Result};
    12|#[cfg(target_os = "macos")]
    13|use crate::qos::{set_thread_qos, QosClass};
    14|
    15|#[cfg(target_os = "macos")]
    16|pub use macos_impl::SystemCapture;
    17|
    18|#[cfg(target_os = "windows")]
    19|pub use windows_impl::SystemCapture;
    20|
    21|#[cfg(target_os = "linux")]
    22|pub use linux_impl::SystemCapture;
    23|
    24|#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    25|pub use stub_impl::SystemCapture;
    26|
    27|const SCK_SAMPLE_RATE: u32 = 48_000;
    28|const SCK_CHANNEL_COUNT: u8 = 1;
    29|
    30|const SILENCE_RMS_THRESHOLD: f32 = 0.002;
    31|
    32|const SILENCE_PAUSE_AFTER_MS: u64 = 30_000;
    33|
    34|/// Start system audio capture, dispatching to the platform-specific backend.
    35|///
    36|/// Returns a `Box<dyn SystemAudioCapture>` that the caller owns. The
    37|/// concrete type is resolved at compile time via `#[cfg]`.
    38|pub fn dispatch_start(
    39|    writer: Arc<AudioWavWriter>,
    40|    target_sample_rate: u32,
    41|) -> Result<Box<dyn SystemAudioCapture>> {
    42|    #[cfg(target_os = "macos")]
    43|    {
    44|        let cap = macos_impl::SystemCapture::start(writer, target_sample_rate)?;
    45|        return Ok(Box::new(cap));
    46|    }
    47|    #[cfg(target_os = "windows")]
    48|    {
    49|        let cap = windows_impl::SystemCapture::start(writer, target_sample_rate)?;
    50|        return Ok(Box::new(cap));
    51|    }
    52|    #[cfg(target_os = "linux")]
    53|    {
    54|        let cap = linux_impl::SystemCapture::start(writer, target_sample_rate)?;
    55|        return Ok(Box::new(cap));
    56|    }
    57|    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    58|    {
    59|        let cap = stub_impl::SystemCapture::start(writer, target_sample_rate)?;
    60|        Ok(Box::new(cap))
    61|    }
    62|}
    63|
    64|#[cfg(target_os = "macos")]
    65|mod macos_impl {
    66|    use super::*;
    67|
    68|    use core_media_rs::cm_sample_buffer::CMSampleBuffer;
    69|    use screencapturekit::shareable_content::SCShareableContent;
    70|    use screencapturekit::stream::configuration::SCStreamConfiguration;
    71|    use screencapturekit::stream::content_filter::SCContentFilter;
    72|    use screencapturekit::stream::output_trait::SCStreamOutputTrait;
    73|    use screencapturekit::stream::output_type::SCStreamOutputType;
    74|    use screencapturekit::stream::SCStream;
    75|
    76|    pub struct SystemCapture {
    77|        inner: SystemCaptureInner,
    78|        writer: Arc<AudioWavWriter>,
    79|    }
    80|
    81|    enum SystemCaptureInner {
    82|        ProcessTap(crate::audio::process_tap::ProcessTapCapture),
    83|
    84|        Sck(Option<SCStream>),
    85|    }
    86|
    87|    struct AudioOutput {
    88|        writer: Arc<AudioWavWriter>,
    89|        resampler: Arc<Mutex<StreamingResampler>>,
    90|
    91|        last_active_ms: AtomicU64,
    92|
    93|        paused: AtomicBool,
    94|    }
    95|
    96|    fn now_ms() -> u64 {
    97|        SystemTime::now()
    98|            .duration_since(UNIX_EPOCH)
    99|            .map(|d| d.as_millis() as u64)
   100|            .unwrap_or(0)
   101|    }
   102|
   103|    fn rms(samples: &[f32]) -> f32 {
   104|        if samples.is_empty() {
   105|            return 0.0;
   106|        }
   107|        let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
   108|        (sum_sq / samples.len() as f32).sqrt()
   109|    }
   110|
   111|    thread_local! {
   112|
   113|
   114|
   115|        static QOS_TAGGED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
   116|    }
   117|
   118|    impl SCStreamOutputTrait for AudioOutput {
   119|        fn did_output_sample_buffer(
   120|            &self,
   121|            sample_buffer: CMSampleBuffer,
   122|            of_type: SCStreamOutputType,
   123|        ) {
   124|            QOS_TAGGED.with(|cell| {
   125|                if !cell.get() {
   126|                    set_thread_qos(QosClass::UserInteractive);
   127|                    cell.set(true);
   128|                }
   129|            });
   130|            if of_type != SCStreamOutputType::Audio {
   131|                return;
   132|            }
   133|            let abl = match sample_buffer.get_audio_buffer_list() {
   134|                Ok(a) => a,
   135|                Err(e) => {
   136|                    error!(?e, "could not get audio buffer list from sample");
   137|                    return;
   138|                }
   139|            };
   140|
   141|            let num_buffers = abl.num_buffers();
   142|            if num_buffers == 0 {
   143|                return;
   144|            }
   145|
   146|            let first = match abl.get(0) {
   147|                Some(b) => b,
   148|                None => return,
   149|            };
   150|            let first_channels = first.number_channels as usize;
   151|            let first_bytes = first.data();
   152|            if first_bytes.is_empty() || first_bytes.len() % 4 != 0 {
   153|                return;
   154|            }
   155|
   156|            let mono: Vec<f32> = if num_buffers == 1 {
   157|                interleaved_to_mono(first_bytes, first_channels.max(1))
   158|            } else {
   159|                deinterleaved_to_mono(&abl, num_buffers)
   160|            };
   161|
   162|            if mono.is_empty() {
   163|                return;
   164|            }
   165|
   166|            let buffer_rms = rms(&mono);
   167|            let now = now_ms();
   168|            let was_paused = self.paused.load(Ordering::Relaxed);
   169|            if buffer_rms >= SILENCE_RMS_THRESHOLD {
   170|                self.last_active_ms.store(now, Ordering::Relaxed);
   171|                if was_paused {
   172|                    self.paused.store(false, Ordering::Relaxed);
   173|                    info!(
   174|                        rms = buffer_rms,
   175|                        threshold = SILENCE_RMS_THRESHOLD,
   176|                        "system audio resumed — leaving silence pause"
   177|                    );
   178|                }
   179|            } else {
   180|                let last_active = self.last_active_ms.load(Ordering::Relaxed);
   181|                let silent_for = now.saturating_sub(last_active);
   182|                if silent_for >= SILENCE_PAUSE_AFTER_MS && !was_paused {
   183|                    self.paused.store(true, Ordering::Relaxed);
   184|                    info!(
   185|                        silent_for_ms = silent_for,
   186|                        threshold = SILENCE_RMS_THRESHOLD,
   187|                        "system audio paused after sustained silence — skipping WAV writes until audio returns"
   188|                    );
   189|                }
   190|            }
   191|
   192|            if self.paused.load(Ordering::Relaxed) {
   193|                return;
   194|            }
   195|
   196|            let resampled = {
   197|                let mut guard = self.resampler.lock();
   198|
   199|                match guard.process(&mono) {
   200|                    Ok(out) => out,
   201|                    Err(e) => {
   202|                        error!(error = %e, "system audio resampler failed");
   203|                        return;
   204|                    }
   205|                }
   206|            };
   207|            if let Err(e) = self.writer.append(&resampled) {
   208|                error!(error = %e, "system audio wav append failed");
   209|            }
   210|        }
   211|    }
   212|
   213|    fn interleaved_to_mono(bytes: &[u8], channels: usize) -> Vec<f32> {
   214|        if channels == 0 {
   215|            return Vec::new();
   216|        }
   217|        let total_samples = bytes.len() / 4;
   218|        let frames = total_samples / channels;
   219|        let mut out = Vec::with_capacity(frames);
   220|        for frame in 0..frames {
   221|            let mut sum = 0.0_f32;
   222|            for c in 0..channels {
   223|                let idx = (frame * channels + c) * 4;
   224|                let s = f32::from_le_bytes([
   225|                    bytes[idx],
   226|                    bytes[idx + 1],
   227|                    bytes[idx + 2],
   228|                    bytes[idx + 3],
   229|                ]);
   230|                sum += s;
   231|            }
   232|            out.push(sum / channels as f32);
   233|        }
   234|        out
   235|    }
   236|
   237|    fn deinterleaved_to_mono(
   238|        abl: &core_audio_types_rs::audio_buffer_list::AudioBufferList,
   239|        num_buffers: usize,
   240|    ) -> Vec<f32> {
   241|        let mut min_frames = usize::MAX;
   242|        for i in 0..num_buffers {
   243|            if let Some(b) = abl.get(i) {
   244|                let frames = b.data().len() / 4;
   245|                if frames < min_frames {
   246|                    min_frames = frames;
   247|                }
   248|            }
   249|        }
   250|        if min_frames == usize::MAX {
   251|            return Vec::new();
   252|        }
   253|        let mut out = Vec::with_capacity(min_frames);
   254|        for frame in 0..min_frames {
   255|            let mut sum = 0.0_f32;
   256|            for i in 0..num_buffers {
   257|                if let Some(b) = abl.get(i) {
   258|                    let bytes = b.data();
   259|                    let idx = frame * 4;
   260|                    let s = f32::from_le_bytes([
   261|                        bytes[idx],
   262|                        bytes[idx + 1],
   263|                        bytes[idx + 2],
   264|                        bytes[idx + 3],
   265|                    ]);
   266|                    sum += s;
   267|                }
   268|            }
   269|            out.push(sum / num_buffers as f32);
   270|        }
   271|        out
   272|    }
   273|
   274|    impl SystemCapture {
   275|        pub fn start(writer: Arc<AudioWavWriter>, target_sample_rate: u32) -> Result<Self> {
   276|            if crate::audio::process_tap::is_supported() {
   277|                match crate::audio::process_tap::ProcessTapCapture::start(
   278|                    Arc::clone(&writer),
   279|                    target_sample_rate,
   280|                ) {
   281|                    Ok(tap) => {
   282|                        info!("system audio: using CoreAudio process tap (System Audio Recording Only)");
   283|                        return Ok(Self {
   284|                            inner: SystemCaptureInner::ProcessTap(tap),
   285|                            writer,
   286|                        });
   287|                    }
   288|                    Err(e) => {
   289|                        warn!(error = %e, "process tap unavailable — falling back to ScreenCaptureKit");
   290|                    }
   291|                }
   292|            }
   293|
   294|            Self::start_sck(writer, target_sample_rate)
   295|        }
   296|
   297|        fn start_sck(writer: Arc<AudioWavWriter>, target_sample_rate: u32) -> Result<Self> {
   298|            let content = SCShareableContent::get().map_err(|e| {
   299|                MeetyError::SystemAudio(format!(
   300|                    "could not enumerate shareable content (Screen Recording permission may be missing): {:?}",
   301|                    e
   302|                ))
   303|            })?;
   304|            let display = content
   305|                .displays()
   306|                .into_iter()
   307|                .next()
   308|                .ok_or_else(|| MeetyError::SystemAudio("no display available".into()))?;
   309|
   310|            let config = SCStreamConfiguration::new()
   311|                .set_captures_audio(true)
   312|                .map_err(|e| MeetyError::SystemAudio(format!("captures_audio: {:?}", e)))?
   313|                .set_excludes_current_process_audio(true)
   314|                .map_err(|e| {
   315|                    MeetyError::SystemAudio(format!("excludes_current_process_audio: {:?}", e))
   316|                })?
   317|                .set_sample_rate(SCK_SAMPLE_RATE)
   318|                .map_err(|e| MeetyError::SystemAudio(format!("sample_rate: {:?}", e)))?
   319|                .set_channel_count(SCK_CHANNEL_COUNT)
   320|                .map_err(|e| MeetyError::SystemAudio(format!("channel_count: {:?}", e)))?;
   321|
   322|            let filter = SCContentFilter::new().with_display_excluding_windows(&display, &[]);
   323|
   324|            let resampler = Arc::new(Mutex::new(StreamingResampler::new(
   325|                SCK_SAMPLE_RATE,
   326|                1,
   327|                target_sample_rate,
   328|            )?));
   329|            let output = AudioOutput {
   330|                writer: writer.clone(),
   331|                resampler,
   332|                last_active_ms: AtomicU64::new(now_ms()),
   333|                paused: AtomicBool::new(false),
   334|            };
   335|
   336|            let mut stream = SCStream::new(&filter, &config);
   337|            stream.add_output_handler(output, SCStreamOutputType::Audio);
   338|            stream
   339|                .start_capture()
   340|                .map_err(|e| MeetyError::SystemAudio(format!("start_capture: {:?}", e)))?;
   341|
   342|            info!(
   343|                sample_rate = SCK_SAMPLE_RATE,
   344|                channels = SCK_CHANNEL_COUNT,
   345|                "ScreenCaptureKit audio stream started (Screen Recording permission)"
   346|            );
   347|
   348|            Ok(Self {
   349|                inner: SystemCaptureInner::Sck(Some(stream)),
   350|                writer,
   351|            })
   352|        }
   353|
   354|        pub fn stop(mut self) -> Result<()> {
   355|            match self.inner {
   356|                SystemCaptureInner::ProcessTap(tap) => {
   357|                    tap.stop()?;
   358|                }
   359|                SystemCaptureInner::Sck(ref mut opt) => {
   360|                    if let Some(stream) = opt.take() {
   361|                        if let Err(e) = stream.stop_capture() {
   362|                            error!(error = ?e, "ScreenCaptureKit stop_capture returned error");
   363|                        }
   364|                        std::thread::sleep(std::time::Duration::from_millis(200));
   365|                    }
   366|                    self.writer.finalize()?;
   367|                }
   368|            }
   369|            debug!(
   370|                samples = self.writer.samples_written(),
   371|                "system audio capture finalized"
   372|            );
   373|            Ok(())
   374|        }
   375|    }
   376|
   377|    impl SystemAudioCapture for SystemCapture {
   378|        fn stop(self: Box<Self>) -> Result<()> {
   379|            let inner = *self;
   380|            inner.stop()
   381|        }
   382|    }
   383|}
   384|
   385|#[cfg(target_os = "windows")]
   386|mod windows_impl {
   387|    use super::*;
   388|
   389|    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
   390|    use cpal::{Sample, SampleFormat, Stream, StreamConfig};
   391|
   392|    fn build_loopback_stream(
   393|        device: &cpal::Device,
   394|        config: &StreamConfig,
   395|        sample_format: SampleFormat,
   396|        writer: Arc<AudioWavWriter>,
   397|        resampler: Arc<Mutex<StreamingResampler>>,
   398|        stopped: Arc<AtomicBool>,
   399|    ) -> Result<Stream> {
   400|        let err_fn = |err| error!(?err, "WASAPI loopback stream error");
   401|        match sample_format {
   402|            SampleFormat::F32 => device
   403|                .build_input_stream(
   404|                    config,
   405|                    move |data: &[f32], _| {
   406|                        if stopped.load(Ordering::SeqCst) {
   407|                            return;
   408|                        }
   409|                        handle_loopback_samples(data, &writer, &resampler, &stopped);
   410|                    },
   411|                    err_fn,
   412|                    None,
   413|                )
   414|                .map_err(|e| MeetyError::StreamBuild(format!("WASAPI loopback f32: {e}")))?,
   415|            SampleFormat::I16 => {
   416|                let writer = writer.clone();
   417|                let resampler = resampler.clone();
   418|                let stopped = stopped.clone();
   419|                device
   420|                    .build_input_stream(
   421|                        config,
   422|                        move |data: &[i16], _| {
   423|                            if stopped.load(Ordering::SeqCst) {
   424|                                return;
   425|                            }
   426|                            let floats: Vec<f32> =
   427|                                data.iter().map(|s| s.to_float_sample()).collect();
   428|                            handle_loopback_samples(&floats, &writer, &resampler, &stopped);
   429|                        },
   430|                        err_fn,
   431|                        None,
   432|                    )
   433|                    .map_err(|e| MeetyError::StreamBuild(format!("WASAPI loopback i16: {e}")))?
   434|            }
   435|            SampleFormat::U16 => {
   436|                let writer = writer.clone();
   437|                let resampler = resampler.clone();
   438|                let stopped = stopped.clone();
   439|                device
   440|                    .build_input_stream(
   441|                        config,
   442|                        move |data: &[u16], _| {
   443|                            if stopped.load(Ordering::SeqCst) {
   444|                                return;
   445|                            }
   446|                            let floats: Vec<f32> =
   447|                                data.iter().map(|s| s.to_float_sample()).collect();
   448|                            handle_loopback_samples(&floats, &writer, &resampler, &stopped);
   449|                        },
   450|                        err_fn,
   451|                        None,
   452|                    )
   453|                    .map_err(|e| MeetyError::StreamBuild(format!("WASAPI loopback u16: {e}")))?
   454|            }
   455|            other => {
   456|                warn!(?other, "unsupported WASAPI loopback sample format");
   457|                return Err(MeetyError::AudioDevice(format!(
   458|                    "unsupported WASAPI loopback sample format: {other:?}"
   459|                )));
   460|            }
   461|        }
   462|    }
   463|
   464|    fn handle_loopback_samples(
   465|        data: &[f32],
   466|        writer: &Arc<AudioWavWriter>,
   467|        resampler: &Arc<Mutex<StreamingResampler>>,
   468|        stopped: &Arc<AtomicBool>,
   469|    ) {
   470|        if stopped.load(Ordering::SeqCst) {
   471|            return;
   472|        }
   473|        let resampled = {
   474|            let mut guard = resampler.lock();
   475|            match guard.process(data) {
   476|                Ok(out) => out,
   477|                Err(e) => {
   478|                    error!(error = %e, "WASAPI loopback resampler failed");
   479|                    return;
   480|                }
   481|            }
   482|        };
   483|        if let Err(e) = writer.append(&resampled) {
   484|            error!(error = %e, "WASAPI loopback wav append failed");
   485|        }
   486|    }
   487|
   488|    pub struct SystemCapture {
   489|        stream: Option<Stream>,
   490|        writer: Arc<AudioWavWriter>,
   491|        stopped: Arc<AtomicBool>,
   492|    }
   493|
   494|    impl SystemCapture {
   495|        pub fn start(writer: Arc<AudioWavWriter>, target_sample_rate: u32) -> Result<Self> {
   496|            let host = cpal::host_from_id(cpal::HostId::WASAPI)
   497|                .map_err(|e| MeetyError::SystemAudio(format!("WASAPI host unavailable: {e}")))?;
   498|
   499|            let device = host
   500|                .default_output_device()
   501|