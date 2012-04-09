
use csv;
import csv::rowiter;

fn main(args: [str]) {
    let reader = csv::new_reader(result::get(io::file_reader(args[1])), ',', '"');
    let mut lc = 0u;
    for reader.iter() { |row|
        lc += 1u;
    };
    io::println(#fmt("read %u rows", lc));
}
