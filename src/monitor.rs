use std::sync::{atomic::{AtomicBool, Ordering::Relaxed}, Arc};
use cpal::{traits::{DeviceTrait, HostTrait, StreamTrait}, Stream};
use log::{error,warn,info,debug,trace};
use crate::types::AudioConfig;



pub fn setup_audio(audio_config: &AudioConfig)->anyhow::Result<(Stream, Arc<AtomicBool>)>{
    // Conditionally compile with jack if the feature is specified.
    warn!("Setting up audio monitor...");
    error!("Something");
    let playing: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    #[cfg(all(
        any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd"
        ),
        feature = "jack"
    ))]
    // Manually check for flags. Can be passed through cargo with -- e.g.
    // cargo run --release --example beep --features jack -- --jack
    let host = if opt.jack {
        cpal::host_from_id(cpal::available_hosts()
            .into_iter()
            .find(|id| *id == cpal::HostId::Jack)
            .expect(
                "make sure --features jack is specified. only works on OSes where jack is available",
            )).expect("jack host unavailable")
    } else {
        cpal::default_host()
    };

    #[cfg(any(
        not(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd"
        )),
        not(feature = "jack")
    ))]
    let host = cpal::default_host();


    debug!("Scanning devices.");
    for dev in host.input_devices()?{
        debug!(" - Found a device: '{:?}'", dev.name().unwrap());
    }
    // Find devices.
    let input_device = if audio_config.input_device == "default" {
        info!("Using default device.... You should figure out a specific one?");
        host.default_input_device()
    } else {
        host.input_devices()?
            .find(|x| x.name().map(|y| y == audio_config.input_device).unwrap_or(false))
    }
    .expect("failed to find input device");


    info!("Using input device: \"{}\"", input_device.name()?); 
    let config: cpal::StreamConfig = input_device.default_input_config()?.into();
    let upd_playing = playing.clone();
    let input_data_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        let mut rms_sum: f32 = 0.;
        let rms_len: usize = data.len();
        for &sample in data {
            rms_sum += sample * sample;
        }
        let rms = 10. * (rms_sum/rms_len as f32).sqrt().log10();
        trace!("RMS VOLUME IS: {}db on {} samples", rms /*10. * rms.log10()*/, rms_len);
        if rms_len > 1000 && rms > -32. { // Minimum sample count for valid volume calculation.
            // println!("Playing because rms {}>{}", rms, -32.);
            upd_playing.store(true, Relaxed);
        }else{
            if upd_playing.load(Relaxed) == true{
                upd_playing.store(false, Relaxed);
            }
        }

    };

    fn err_fn(err: cpal::StreamError) {
        error!("an error occurred on stream: {}", err);
    }


    let input_stream = input_device.build_input_stream(&config, input_data_fn, err_fn, None)?;//.unwrap();
    input_stream.play()?;

    // Ok(Box::new(host))
    // Err(anyhow::anyhow!("Fuck it"))
    Ok((input_stream, playing.clone()))
}


#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Datelike, Local, NaiveDate, NaiveDateTime, Utc};
    use crate::config::load_config;
    use crate::configure_logging;

    #[test]
    fn test_listen() {
        let config = load_config().unwrap();
        configure_logging(log::LevelFilter::Debug,
            config.logfile
        );
        let (stream, is_playing) = setup_audio(&config.audio_config.unwrap()).unwrap();
        println!("Set up audio...");
        for i in 0..10{
            std::thread::sleep(std::time::Duration::from_secs(1));
            println!("Playing? {}", is_playing.load(Relaxed))
        }
        println!("Shutting down audio...");
        drop(stream);
        
    }
}
