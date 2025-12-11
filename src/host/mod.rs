use clack_host::events::event_types::*;
use clack_host::prelude::*;
use std::path::PathBuf;

pub fn list_plugins() -> Vec<(String, PathBuf)> {
    Vec::new()
}

// Prepare our (extremely basic) host implementation

struct PluginHostShared;

impl<'a> SharedHandler<'a> for PluginHostShared {
    /* ... */
    fn request_restart(&self) { /* ... */
    }
    fn request_process(&self) { /* ... */
    }
    fn request_callback(&self) { /* ... */
    }
}

struct PluginHost;

impl HostHandlers for PluginHost {
    type Shared<'a> = PluginHostShared;

    type MainThread<'a> = ();
    type AudioProcessor<'a> = ();
}

pub fn load_and_process() -> Result<(), Box<dyn std::error::Error>> {
    // Information about our totally legit host.
    let host_info = HostInfo::new("Boxy-Synth", "", "", "Number")?;

    let bundle =
        unsafe { PluginBundle::load(shellexpand::tilde("~/.clap/Wt Synth.clap").to_string())? };
    let plugin_factory = bundle.get_plugin_factory().unwrap();

    let plugin_descriptor = plugin_factory
        .plugin_descriptors()
        .find(|d| d.id().unwrap().to_bytes() == b"online.eoghan-west.wt-synth")
        .unwrap();

    let mut plugin_instance = PluginInstance::<PluginHost>::new(
        |_| PluginHostShared,
        |_| (),
        &bundle,
        plugin_descriptor.id().unwrap(),
        &host_info,
    )?;

    let audio_configuration = PluginAudioConfiguration {
        sample_rate: 48_000.0,
        min_frames_count: 4,
        max_frames_count: 4,
    };
    let audio_processor = plugin_instance.activate(|_, _| (), audio_configuration)?;

    // let note_on_event = NoteOnEvent::new(0, Pckn::new(0u16, 0u16, 12u16, 60u32), 4.2);
    // let input_events_buffer = [note_on_event];
    // let input_events_buffer: [InputEvents; 0] = [];
    let input_events_buffer: Vec<&UnknownEvent> = Vec::new();
    let mut output_events_buffer = EventBuffer::new();

    println!("before input audio buffer");
    // let mut input_audio_buffers = [[0.0f32; 4]; 0]; // 2 channels (stereo), 1 port
    let mut output_audio_buffers = [[0.0f32; 4]; 1];
    println!("after audio buffer");

    // let mut input_ports = AudioPorts::with_capacity(0, 0); // 2 channels (stereo), 1 port
    let mut output_ports = AudioPorts::with_capacity(1, 1);
    println!("after ports");

    // Let's send the audio processor to a dedicated audio processing thread.
    let audio_processor = std::thread::scope(|s| {
        s.spawn(|| {
            println!("before start_processing");
            let mut audio_processor = audio_processor.start_processing().unwrap();
            println!("after start_processing");

            // TODO: output to Jack

            loop {
                let input_events = InputEvents::from_buffer(&input_events_buffer);
                let mut output_events = OutputEvents::from_buffer(&mut output_events_buffer);

                // let mut input_audio = input_ports.with_input_buffers([AudioPortBuffer {
                //     latency: 0,
                //     channels: AudioPortBufferType::f32_input_only(
                //         input_audio_buffers
                //             .iter_mut()
                //             .map(|b| InputChannel::constant(b)),
                //     ),
                // }]);

                let mut output_audio = output_ports.with_output_buffers([AudioPortBuffer {
                    latency: 0,
                    channels: AudioPortBufferType::f32_output_only(
                        output_audio_buffers.iter_mut().map(|b| b.as_mut_slice()),
                    ),
                }]);

                // println!("before processor.process");

                // Finally do the processing itself.
                let status = audio_processor
                    .process(
                        &InputAudioBuffers::empty(),
                        &mut output_audio,
                        &input_events,
                        &mut output_events,
                        None,
                        None,
                    )
                    .unwrap();

                // println!("after processor.process");

                if status == clack_host::process::ProcessStatus::ContinueIfNotQuiet
                    && output_audio
                        .as_raw_buffers()
                        .iter()
                        .map(|buffer| unsafe { **buffer.data32 })
                        .sum::<f32>()
                        < 0.000001
                {
                    break;
                }

                // Send the audio processor back to be deallocated by the main thread.
            }

            audio_processor.stop_processing()
        })
        .join()
        .unwrap()
    });

    plugin_instance.deactivate(audio_processor);
    Ok(())
}
