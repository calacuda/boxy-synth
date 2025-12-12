use clack_extensions::params::PluginParams;
use clack_host::events::event_types::*;
use clack_host::prelude::*;
use jack::{AudioOut, MidiIn};
use std::io;
use std::path::PathBuf;
use std::thread::spawn;

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

struct MyHostMainThread<'a> {
    shared: &'a PluginHostShared,
    instance: Option<InitializedPluginHandle<'a>>,
    // The latency that is sent to us by the plugin's Latency extension.
    // latency_changed: bool
}

impl<'a> MainThreadHandler<'a> for MyHostMainThread<'a> {
    // The plugin's instance handle is required to call extension methods.
    fn initialized(&mut self, instance: InitializedPluginHandle<'a>) {
        self.instance = Some(instance);
    }
}

// impl<'a> HostLatencyImpl for MyHostMainThread<'a> {
//     // This method is called by the plugin whenever its latency changed.
//     fn changed(&mut self) {
//         // Ensure that the plugin is instantiated and supports the Latency extension.
//         if let Some(Some(_latency)) = self.shared.latency_extension.get() {
//             self.latency_changed = true
//         }
//     }
// }

struct PluginHost;

impl HostHandlers for PluginHost {
    type Shared<'a> = PluginHostShared;

    type MainThread<'a> = (); // MyHostMainThread<'a>;
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
    let mut output_ports = AudioPorts::with_capacity(1, 1);
    let mut audio_processor = audio_processor.start_processing().unwrap();

    // Create client
    let (client, _status) = jack::Client::new("WT Synth", jack::ClientOptions::default())
        .expect("failed to build client");

    // println!("client made");

    let mut out_port = client
        .register_port("Mono", AudioOut::default())
        .expect("failed to register audio out port");

    let midi_in_port = client
        .register_port("MidiIn", MidiIn::default())
        .expect("failed to register midi input port");

    let cback = move |_: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
        let show_p = midi_in_port.iter(ps);
        let mut input_events_buffer = EventBuffer::new();

        for e in show_p {
            let time = e.time;

            if e.bytes.len() == 3 {
                input_events_buffer.push(&MidiEvent::new(
                    time,
                    0,
                    [e.bytes[0], e.bytes[1], e.bytes[2]],
                ));
            } else {
                input_events_buffer.push(&Midi2Event::new(
                    time,
                    0,
                    [
                        e.bytes[0].into(),
                        e.bytes[1].into(),
                        e.bytes[2].into(),
                        e.bytes[3].into(),
                    ],
                ));
            }
        }

        let writer = out_port.as_mut_slice(ps);
        let input_events = InputEvents::from_buffer(&input_events_buffer);
        let mut output_audio = output_ports.with_output_buffers([AudioPortBuffer {
            latency: 0,
            channels: AudioPortBufferType::f32_output_only([writer.iter_mut().into_slice()]),
        }]);

        // Finally do the processing itself.
        let status = audio_processor
            .process(
                &InputAudioBuffers::empty(),
                &mut output_audio,
                &input_events,
                &mut OutputEvents::void(),
                None,
                None,
            )
            .unwrap();

        if status == clack_host::process::ProcessStatus::ContinueIfNotQuiet
            && output_audio
                .as_raw_buffers()
                .iter()
                .map(|buffer| unsafe { **buffer.data32 })
                .sum::<f32>()
                < 0.000001
        {
            jack::Control::Quit
        } else {
            jack::Control::Continue
        }
    };

    let active_client = spawn(|| {
        client
            .activate_async((), jack::contrib::ClosureProcessHandler::new(cback))
            .expect("failed to activate_async")
    });

    // sleep(Duration::from_secs(10));

    let mut user_input = String::new();
    io::stdin()
        .read_line(&mut user_input)
        .expect("problem waiting");

    // Optional deactivation.
    if let Err(err) = active_client.join().expect("failed to join").deactivate() {
        eprintln!("JACK exited with error: {err}");
    } else {
        println!("JACK deactivated successfully");
    };

    Ok(())
}
