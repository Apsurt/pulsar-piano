//! Print every raw message from every MIDI input, SysEx and clock included.
//! Useful for finding out what a keyboard's buttons send: `cargo run -p pulsar-engine --example sniff`

use std::time::Duration;

use midir::{Ignore, MidiInput};

fn main() {
    let probe = MidiInput::new("Pulsar sniff").expect("MIDI unavailable");
    let mut connections = Vec::new();
    for port in probe.ports() {
        let name = probe.port_name(&port).unwrap_or_default();
        let mut input = MidiInput::new("Pulsar sniff").expect("MIDI unavailable");
        input.ignore(Ignore::None);
        let label = name.clone();
        match input.connect(
            &port,
            "sniff",
            move |stamp, bytes, _| {
                let hex: Vec<String> = bytes.iter().map(|b| format!("{b:02X}")).collect();
                println!("{:>10} µs  {label:<20} {}", stamp, hex.join(" "));
            },
            (),
        ) {
            Ok(c) => {
                println!("listening: {name}");
                connections.push(c);
            }
            Err(e) => println!("could not open {name}: {e}"),
        }
    }
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}
