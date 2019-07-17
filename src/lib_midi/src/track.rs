use crate::tracks_parser::TracksParser;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TempoEvent {
    pub time_in_units: f32,
    pub tempo: u32,
}

#[derive(Debug, Clone)]
pub struct MidiNote {
    pub start: f32,
    pub duration: f32,
    pub note: u8,
    pub vel: u8,
    pub ch: u8,
    pub track_id: usize,
    pub id: usize,
}

#[derive(Debug, Clone)]
pub struct MidiTrack {
    pub tempo: u32,
    pub tempo_events: Vec<TempoEvent>,
    pub has_tempo: bool,
    pub notes: Vec<MidiNote>,
    pub track_id: usize,
}

impl MidiTrack {
    pub fn new(track: &[midly::Event], track_id: usize) -> MidiTrack {
        let mut tempo = 500_000; // 120 bpm

        use midly::EventKind;
        use midly::MetaMessage;

        let mut has_tempo = false;
        let mut tempo_events = Vec::new();

        let mut time_in_units: f32 = 0.0;
        for event in track.iter() {
            time_in_units += event.delta.as_int() as f32;

            if let EventKind::Meta(meta) = &event.kind {
                if let MetaMessage::Tempo(t) = &meta {
                    if !has_tempo {
                        tempo = t.as_int();
                        has_tempo = true;
                    }
                    tempo_events.push(TempoEvent {
                        time_in_units,
                        tempo: t.as_int(),
                    });
                }
            };

        }

        MidiTrack {
            tempo,
            tempo_events,
            has_tempo,
            track_id,
            notes: Vec::new(),
        }
    }

    pub fn extract_notes(&mut self, events: &[midly::Event], parent_parser: &mut TracksParser) {
        self.notes.clear();

        let mut time_in_units = 0.0;

        struct Note {
            time_in_units: f32,
            vel: u8,
            channel: u8,
        };
        let mut current_notes: HashMap<u8, Note> = HashMap::new();

        macro_rules! end_note {
            (k => $e:expr) => {
                let k = $e;
                if current_notes.contains_key(&k) {
                    let n = current_notes.get(&k).unwrap();

                    let start = parent_parser.pulses_to_ms(n.time_in_units) / 1000.0;
                    let duration = parent_parser.pulses_to_ms(time_in_units) / 1000.0 - start;

                    let mn = MidiNote {
                        start: start as f32,
                        duration: duration as f32,
                        note: k,
                        vel: n.vel,
                        ch: n.channel,
                        track_id: self.track_id,
                        id: 0, // Placeholder
                    };
                    self.notes.push(mn);
                    current_notes.remove(&k);
                }
            };
        }

        for event in events.iter() {
            use midly::EventKind;
            use midly::MidiMessage;

            time_in_units += event.delta.as_int() as f32;

            if let EventKind::Midi { channel, message } = &event.kind {
                match &message {
                    MidiMessage::NoteOn(data0, data1) => {
                        let data0 = data0.as_int();
                        let data1 = data1.as_int();
                        if data1 > 0 {
                            let k = data0;

                            match current_notes.entry(k) {
                                std::collections::hash_map::Entry::Occupied(_e) => {
                                    end_note!(k=>k);
                                }
                                std::collections::hash_map::Entry::Vacant(_e) => {
                                    current_notes.insert(
                                        k,
                                        Note {
                                            time_in_units,
                                            vel: data1,
                                            channel: channel.as_int(),
                                        },
                                    );
                                }
                            }

                        } else if data1 == 0 {
                            end_note!(k=>data0);
                        }

                    }
                    MidiMessage::NoteOff(data0, _data1) => {
                        let data0 = data0.as_int();

                        end_note!(k=>data0);
                    }
                    _ => {}
                }
            }
        }

        self.notes
            .sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());
    }
}
