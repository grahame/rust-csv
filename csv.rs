
use std;
import std::io;
import result;

type reader = obj {
    fn read_row() -> [str];
};

fn mk_reader(f: std::io::reader, delim: char, quote: char, has_header: bool) -> reader {
    type readerstate = { f: std::io::reader, delim: char, quote: char, has_header: bool };
    obj reader(st: readerstate) {
        fn read_row() -> [str] {
            let line = st.f.read_line();
            tag rowstate { 
                none();
                field();
                escaped_field();
            };
            let sst = st; /* work around https://github.com/graydon/rust/issues/1286 */
            str::iter_chars(line, {|c|
                io::println(#fmt("char %c", c));
                io::println(#fmt("char %c", sst.delim));
            });
            ret [""];
        }
    }

    let st = { f: f, delim: delim, quote: quote, has_header: has_header};
    let r = reader(st);
    ret r;
}

fn main(args : [str])
{
    if (vec::len(args) != 2u) {
        ret;
    }
    let f : io::reader = result::get(io::file_reader(args[1]));
    let r = mk_reader(f, ',', '"', true);
    r.read_row();
}

