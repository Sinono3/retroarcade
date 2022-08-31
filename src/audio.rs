use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

pub fn init() -> Result<cpal::Device> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("failed to find a default output device");
    Ok(device)
}

pub fn run<F>(device: &cpal::Device, source: F) -> Result<cpal::Stream>
where
    F: FnMut(&mut [i16]) -> bool + Send + 'static,
{
    let config = device.default_output_config()?;

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => run_with_format::<f32, F>(device, &config.into(), source)?,
        cpal::SampleFormat::I16 => run_with_format::<i16, F>(device, &config.into(), source)?,
        cpal::SampleFormat::U16 => run_with_format::<u16, F>(device, &config.into(), source)?,
    };

    Ok(stream)
}

fn run_with_format<S, F>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut source: F,
) -> Result<cpal::Stream>
where
    S: cpal::Sample,
    F: FnMut(&mut [i16]) -> bool + Send + 'static,
{
    // Temporary buffer
    let mut buf: Vec<i16> = Vec::new();

    // Create and run the stream.
    let convert_sample = |sample| -> S { cpal::Sample::from::<i16>(&sample) };
    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);
    let channels = config.channels as usize;

    assert_eq!(channels, 2, "only stereo audio is supported");

    let stream = device.build_output_stream(
        config,
        move |output: &mut [S], _: &cpal::OutputCallbackInfo| {
            // Fill buffer with new samples
            buf.resize(output.len(), 0);
            source(&mut buf);

            // libretro always outputs a **stereo** 16-bit integer interleaved sample buffer
            let mut sample_iter = buf.chunks_exact(2);

            for output_frame in output.chunks_mut(channels) {
                let sample_frame = sample_iter.next().unwrap_or(&[0, 0]);
                output_frame[0] = convert_sample(sample_frame[0]);
                output_frame[1] = convert_sample(sample_frame[1]);
            }
        },
        err_fn,
    )?;
    stream.play()?;
    Ok(stream)
}

/*fn write_data<T, F>(
    output: &mut [T],
    channels: usize,
    complete_tx: &mpsc::SyncSender<()>,
    source: &mut F,
) where
    T: cpal::Sample,
    F: FnMut(&mut [i16]) -> bool + Send + 'static,
{
}*/
