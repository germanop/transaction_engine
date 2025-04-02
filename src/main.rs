mod account;
mod csv;
mod deser;
mod engine;

use crate::deser::{OutRecord, Record};
use anyhow::Result;

fn main() -> Result<()> {
    // very basic option parsing
    let file_path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Missing filename argument");
        std::process::exit(1)
    });

    let mut rdr = csv::csv_reader_from_file((file_path).as_ref())?;

    // Start Engine thread with appropriate communication channel
    // How communication is handled, how results are printed etc. are left to the closure to implement them.
    let (tx, rx) = std::sync::mpsc::sync_channel::<Record>(1); // I don't need to feed the engine faster than this
    let mut engine = engine::Engine::new();
    let handle = std::thread::spawn(move || {
        eprintln!("Starting Engine");

        while let Ok(record) = rx.recv() {
            if let Err(err) = engine.process(&record) {
                eprintln!("Error processing record {:?}: {}", record, err);
            }
        }

        eprintln!("Stopping Engine and printing results");
        // retrieve accounts data
        let accounts = engine.get_accounts();

        // build CSV writer
        let mut wtr = csv::CsvWriterBuilder::new(std::io::stdout()).build();

        // Start writing
        for account in accounts.values() {
            let out_record = OutRecord::from(account);
            if let Err(err) = wtr.serialize(out_record) {
                eprintln!("Error writing record: {}", err);
            }
        }
        if let Err(err) = wtr.flush() {
            eprintln!("Error flushing writer: {}", err);
        }
        eprintln!("Done");
    });

    // Read from CSV and send to Engine
    for record in rdr.deserialize() {
        match record {
            Ok(record) => {
                if let Err(err) = tx.send(record) {
                    eprintln!("Error sending record: {}", err);
                }
            }
            Err(err) => eprintln!("Error reading record: {}", err),
        }
    }

    // Drop the sender so the receiver will stop
    drop(tx);
    handle.join().expect("Engine thread panicked");

    eprintln!("Main thread done");
    Ok(())
}
