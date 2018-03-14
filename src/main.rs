extern crate vst;
extern crate hound;

use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::error::Error;

use vst::host::{Host, PluginLoader};
use vst::plugin::Plugin;
use vst::event::MidiEvent;
use vst::buffer::{AudioBuffer, SendEventBuffer};
use vst::api::Events;

const BUFFER_SIZE: usize = 1024;
const SAMPLE_RATE: u32 = 44_100;

struct SampleHost;

impl Host for SampleHost {
    fn automate(&mut self, _index: i32, _value: f32) {
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

    fn process_events(&mut self, _events: &Events) {
        println!("Host: process_events");
    }
}

fn main() {
    let path = if let Some(path) = std::env::args().nth(1) {
        PathBuf::from(path)
    } else {
        PathBuf::from(
            "D:\\Program Files\\Common Files\\VST2\\Pianoteq 5 (64-bit).dll",
        )
    };

    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut wav_writer = hound::WavWriter::create("out.wav", spec).unwrap();

    let host = Arc::new(Mutex::new(SampleHost));

    println!("Loading {}...", path.to_str().unwrap());

    let mut loader = PluginLoader::load(&path, host.clone())
        .unwrap_or_else(|e| panic!("Failed to load plugin: {}", e.description()));

    let mut instance = loader.instance().unwrap();
    let info = instance.get_info();
    println!("{:#?}", info);

    instance.init();
    instance.set_sample_rate(SAMPLE_RATE as f32);

    let mut input_buffers = Vec::new();
    let mut output_buffers = Vec::new();
    for _ in 0..info.inputs {
        input_buffers.push([0.0f32; BUFFER_SIZE]);
    }
    for _ in 0..info.outputs {
        output_buffers.push([0.0f32; BUFFER_SIZE]);
    }

    let mut input_pointers = Vec::new();
    for a in input_buffers {
        input_pointers.push(a.as_ptr());
    }

    let mut output_pointers = Vec::new();
    for out in &mut output_buffers {
        output_pointers.push(out.as_mut_ptr());
    }

    // let notes = [60, 62, 64, 65, 67, 69, 71, 72]; // C major scale
    let notes = [74, 76, 78, 79, 81, 83, 85, 86]; // D major scale

    let mut midi_events = Vec::new();
    let mut send_buffers = Vec::new();
    for note in &notes {
        midi_events.push(MidiEvent {
            data: [144, *note, 90],
            delta_frames: 0,
            live: false,
            note_length: None,
            note_offset: None,
            detune: 0,
            note_off_velocity: 0,
        });
        let mut send_buffer = SendEventBuffer::new(1);
        send_buffer.store_midi(&[midi_events[midi_events.len() - 1]]);
        send_buffers.push(send_buffer);
    }

    let mut buffer = AudioBuffer::new(
        input_pointers.as_slice(),
        output_pointers.as_mut_slice(),
        BUFFER_SIZE,
    );

    let mut k = 0;
    let mut direction = true;
    for i in 0..500 {
        if i % 10 == 0 {
            instance.process_events(send_buffers[k].events());

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

        instance.process(&mut buffer);

        let (_inputs, outputs) = buffer.split();
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
