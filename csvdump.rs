
use std::io;
use csv;

fn main(args : [str])
{
    if (vec::len(args) != 2u) {
        ret;
    }
    let f : io::reader = result::get(io::file_reader(args[1]));
    let reader = new_reader(f, ',', '"');
    while true {
        let res = reader.readrow();
        if result::failure(res) {
            break;
        }
        let row = result::get(res);
        io::println(#fmt("---- ROW %u fields -----", row.len()));
        let i = 0u;
        while i < row.len() {
            io::println(row.getstr(i));
            i = i + 1u;
        }
    }
}

