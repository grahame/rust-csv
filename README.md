## Note

This is for a truly ancient version of Rust. I may come back and
port to modern Rust (or would eagerly take a pull request doing
so). Right now, this isn't in a working state.

CSV: parser for RFC 4180 CSV files

This package implements a parser for the RFC 4180 CSV file format.

## API

    use csv;
    import csv::rowiter;

    fn main () {
        let reader = csv::new_reader(result::get(io::file_reader("/path/to/file")), ',', '"');
        let mut row = [];
        while reader.readrow(row) {
            row.iter {|col|
                io::println(col)
            }
        }
    }

See the `test` module in `csv.rs` for more examples.

