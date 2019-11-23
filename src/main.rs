use std::error::Error;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use hound::{SampleFormat, WavSpec, WavWriter};
use vst::api::Events;
use vst::buffer::SendEventBuffer;
use vst::event::MidiEvent;
use vst::host::{Host, HostBuffer, PluginLoader};
use vst::plugin::Plugin;

const BUFFER_SIZE: usize = 1024;
const SAMPLE_RATE: u32 = 44_100;

struct SampleHost;

impl Host for SampleHost {
    fn automate(&self, _index: i32, _value: f32) {
        unimplemented!();
    }

    fn get_plugin_id(&self) -> i32 {
        unimplemented!();
    }

    fn idle(&self) {
        unimplemented!();
    }

    fn get_info(&self) -> (isize, String, String) {
        println!("Host: get_info");
        (0, "Hello".into(), "World".into())
    }

    fn process_events(&self, _events: &Events) {
        println!("Host: process_events");
    }
}

fn main() {
    let path = if let Some(path) = std::env::args().nth(1) {
        PathBuf::from(path)
    } else {
        PathBuf::from("D:\\Program Files\\Common Files\\VST2\\Pianoteq 6 (64-bit).dll")
    };

    let spec = WavSpec {
        channels: 2,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut wav_writer = WavWriter::create("out.wav", spec).unwrap();

    let host = Arc::new(Mutex::new(SampleHost));

    println!("Loading {}...", path.display());

    let mut loader = PluginLoader::load(&path, host)
        .unwrap_or_else(|e| panic!("Failed to load plugin: {}", e.description()));

    let mut plugin = loader.instance().unwrap();
    let info = plugin.get_info();
    println!("{:#?}", info);

    plugin.init();
    plugin.set_sample_rate(SAMPLE_RATE as f32);

    // let notes = [60, 62, 64, 65, 67, 69, 71, 72]; // C major scale
    let notes = [74, 76, 78, 79, 81, 83, 85, 86]; // D major scale

    let mut midi_events = Vec::new();
    for note in &notes {
        midi_events.push(MidiEvent {
            data: [0x90, *note, 90], // Note on
            delta_frames: 0,
            live: false,
            note_length: None,
            note_offset: None,
            detune: 0,
            note_off_velocity: 0,
        });
    }

    let mut host_buffer: HostBuffer<f32> = HostBuffer::from_info(&info);
    let inputs = vec![vec![0.0; BUFFER_SIZE]; host_buffer.input_count()];
    let mut outputs = vec![vec![0.0; BUFFER_SIZE]; host_buffer.output_count()];
    let mut audio_buffer = host_buffer.bind(&inputs, &mut outputs);

    let mut send_buffer = SendEventBuffer::new(1);

    let mut k = 0;
    let mut direction = true;
    for i in 0..500 {
        if i % 10 == 0 {
            send_buffer.send_events_to_plugin(&[midi_events[k]], &mut plugin);

            if k == 0 {
                direction = true;
            } else if k == notes.len() - 1 {
                direction = false;
            }

            if direction {
                k += 1;
            } else {
                k -= 1;
            }
        }

        plugin.process(&mut audio_buffer);

        let (_inputs, outputs) = audio_buffer.split();
        for s in outputs[0].iter().zip(outputs[1].iter()) {
            wav_writer
                .write_sample((*s.0 * std::i16::MAX as f32) as i16)
                .unwrap();
            wav_writer
                .write_sample((*s.1 * std::i16::MAX as f32) as i16)
                .unwrap();
        }
    }
}
