CSV: parser for RFC 4180 CSV files

This package implements a parser for the RFC 4180 CSV file format.

## API

    let reader = csv::new_reader(std::io::file_reader("/path/to/file", ',', '"');
    while true {
        let res = reader.readrow();
        if result::failure(res) {
            break;
        }
        let row = result::get(res);
        let i = 0u;
        while i < row.len() {
            io::println(row.getstr(i));
            i = i + 1u;
        }
    }

See the `test` module in `csv.rs` for more examples.

