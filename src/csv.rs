use anyhow::Result;
use csv::{Reader, ReaderBuilder, Trim, Writer, WriterBuilder};
use std::io::{Read, Write};
use std::path::Path;

/// Struct to build CSV readers with the right parameters
pub struct CsvReaderBuilder<R: Read> {
    reader: Reader<R>,
}

impl<R: Read> CsvReaderBuilder<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader: ReaderBuilder::new()
                .has_headers(true)
                .trim(Trim::All)
                .from_reader(reader),
        }
    }

    /// Create csv::Reader
    pub fn build(self) -> Reader<R> {
        self.reader
    }
}

/// Struct to build CSV writers with the right parameters
pub struct CsvWriterBuilder<W: Write> {
    writer: Writer<W>,
}

impl<W: Write> CsvWriterBuilder<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer: WriterBuilder::new().has_headers(true).from_writer(writer),
        }
    }

    /// Create csv::Writer
    pub fn build(self) -> Writer<W> {
        self.writer
    }
}

/// Convenient wrapper for creating a proper csv::Reader from a file
pub fn csv_reader_from_file(file_path: &Path) -> Result<Reader<std::fs::File>> {
    let file = std::fs::File::open(file_path)?;
    Ok(CsvReaderBuilder::new(file).build())
}

// The whole test suite tests csv together with `Record` and `OutRecord` deser.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::deser::Record;
    use itertools::Itertools;
    use rust_decimal::Decimal;
    use std::io::{Cursor, Write};
    use tempfile::NamedTempFile;

    #[test]
    fn test_csv_read_from_file() {
        // create temp file with weird column spacing
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, " type ,client, tx,amount").unwrap();
        writeln!(temp_file, "deposit, 1 , 1,1.33").unwrap();
        writeln!(temp_file, "dispute ,1,   1   ,").unwrap();

        let mut rdr = csv_reader_from_file(temp_file.path()).unwrap();

        let expected = vec![
            Record {
                command: "deposit".to_string(),
                client: 1,
                tx: 1,
                amount: Some(Decimal::new(133, 2)), // 1.33
            },
            Record {
                command: "dispute".to_string(),
                client: 1,
                tx: 1,
                amount: None,
            },
        ];
        for (entry, expected_record) in rdr.deserialize().zip_eq(expected.iter()) {
            let record: Record = entry.unwrap();
            assert_eq!(&record, expected_record);
        }
    }

    #[test]
    #[should_panic]
    fn test_csv_read_negative_numbers() {
        let data = "type,client,tx,amount\ndeposit,1,-2,";
        let mut rdr = CsvReaderBuilder::new(Cursor::new(data)).build();
        let _: Record = rdr.deserialize().next().unwrap().unwrap();
    }

    #[test]
    fn test_csv_read_ok() {
        let data = "type,client,tx,amount\ndeposit, 1000, 2, 1.2";
        let mut rdr = CsvReaderBuilder::new(Cursor::new(data)).build();
        let record: Record = rdr.deserialize().next().unwrap().unwrap();
        assert_eq!(record.client, 1_000);
        assert_eq!(record.tx, 2);
        assert_eq!(record.amount, Some(Decimal::new(12, 1)));
    }

    #[test]
    #[should_panic]
    fn test_csv_read_huge_client_id() {
        let data = "type,client,tx,amount\ndeposit, 100000, 2, 1.2";
        let mut rdr = CsvReaderBuilder::new(Cursor::new(data)).build();
        let _: Record = rdr.deserialize().next().unwrap().unwrap();
    }

    // Test Bankers rounding
    #[test]
    fn test_csv_with_extra_precision() {
        let data = "type,client,tx,amount\ndeposit, 1, 1, 1.23455\ndeposit, 2, 2, 1.23465";
        let mut rdr = CsvReaderBuilder::new(Cursor::new(data)).build();
        let mut rdr_iter = rdr.deserialize();
        let record: Record = rdr_iter.next().unwrap().unwrap();
        assert_eq!(record.client, 1);
        assert_eq!(record.tx, 1);
        assert_eq!(record.amount, Some(Decimal::new(12346, 4)));
        let record: Record = rdr_iter.next().unwrap().unwrap();
        assert_eq!(record.client, 2);
        assert_eq!(record.tx, 2);
        assert_eq!(record.amount, Some(Decimal::new(12346, 4)));
    }

    #[test]
    fn test_csv_write_ok() {
        // ToDo
    }
}
