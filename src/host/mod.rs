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
    let note_on_event = NoteOnEvent::new(0, Pckn::new(0u16, 0u16, 48u16, 60u32), 126.0);
    let input_events_buffer = [note_on_event];

    let mut output_ports = AudioPorts::with_capacity(1, 1);
    println!("after ports");

    println!("before start_processing");
    let mut audio_processor = audio_processor.start_processing().unwrap();
    println!("after start_processing");

    // TODO: output to Jack

    // Create client
    let (client, _status) = jack::Client::new("WT Synth", jack::ClientOptions::default())
        .expect("failed to build client");

    println!("client made");

    let mut out_port = client
        .register_port("Mono", AudioOut::default())
        .expect("failed to register audio out port");

    let mut midi_in_port = client
        .register_port("MidiIn", MidiIn::default())
        .expect("failed to register midi input port");

    let input_events = InputEvents::from_buffer(&input_events_buffer);
    let mut output_audio_buffers = [[0.0f32; 1024]; 1];
    let mut output_audio = output_ports.with_output_buffers([AudioPortBuffer {
        latency: 0,
        channels: AudioPortBufferType::f32_output_only(
            output_audio_buffers.iter_mut().map(|b| b.as_mut_slice()),
        ),
    }]);

    audio_processor
        .process(
            &InputAudioBuffers::empty(),
            // &mut OutputAudioBuffers::empty(),
            &mut output_audio,
            &input_events,
            &mut OutputEvents::void(),
            None,
            None,
        )
        .unwrap();

    let input_events_buffer: Vec<&UnknownEvent> = Vec::new();

    let cback = move |_: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
        // let mut put_p = maker.writer(ps);
        // put_p
        //     .write(&jack::RawMidi {
        //         time: 0,
        //         bytes: &[
        //             0b10010000, /* Note On, channel 1 */
        //             0b01000000, /* Key number */
        //             0b01111111, /* Velocity */
        //         ],
        //     })
        //     .unwrap();

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
