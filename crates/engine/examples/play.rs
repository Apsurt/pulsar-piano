//! Play without the app: `cargo run -p pulsar-engine --example play -- <sounds dir> [instrument id]`

use std::path::PathBuf;

use pulsar_engine::{Engine, EngineEvent};

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args.next().map(PathBuf::from).unwrap_or_else(|| ".".into());
    let (engine, events) = Engine::start(dir).expect("failed to start engine");

    let status = engine.status();
    println!(
        "output: {} @ {} Hz",
        status.output_device, status.sample_rate
    );
    let instruments = engine.instruments();
    for i in &instruments {
        println!("  {:<12} {}", i.kind, i.id);
    }
    let id = args.next().unwrap_or_else(|| instruments[0].id.clone());
    match engine.load_instrument(&id) {
        Ok(i) => println!("loaded {}", i.name),
        Err(e) => eprintln!("load failed: {e}"),
    }
    std::thread::sleep(std::time::Duration::from_millis(500));
    println!("output latency: {:.1} ms", engine.status().latency_ms);

    println!("play your MIDI keyboard; Ctrl-C to quit");
    for event in events {
        match event {
            EngineEvent::Midi(m) => println!(
                "{m:?}  ({:.1} ms output latency)",
                engine.status().latency_ms
            ),
            other => println!("{other:?}"),
        }
    }
}
