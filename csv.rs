
use std;
import std::io;
import result;

type reader = obj {
    fn read_row() -> result::t<[str], str>;
};

fn mk_reader(f: std::io::reader, delim: char, quote: char, has_header: bool) -> reader {
    tag state {
        start;
        field([char]);
        escapedfield([char]);
        inquote([char]);
        escapeend([char]);
    };
    type readerstate = {
        f: std::io::reader,
        delim: char,
        quote: char,
        has_header: bool,
        mutable buf : [char],
        mutable offset : uint,
        mutable state : state,
    };
    obj reader(st: readerstate) {
        fn read_row() -> result::t<[str], str> {
            st.buf = st.f.read_chars(1024u);
            /* should probably use a result */
            if vec::len(st.buf) == 0u {
                ret result::err("EOF");
            }
            st.offset = 0u;
            st.state = start;
            let row: [str] = [];
            while st.offset < vec::len(st.buf) {
                let c : char = st.buf[st.offset];
                st.offset += 1u;
                alt st.state {
                    start() {
                        if c == '"' {
                            st.state = escapedfield([]);
                        } if c == '\n' {
                            break;
                        } else {
                            st.state = field([c]);
                        }
                    }
                    field(x) {
                        if c == '\n' {
                            row += [str::from_chars(x)];
                            break;
                        } else if c == ',' {
                            st.state = start;
                            row += [str::from_chars(x)];
                        } else {
                            st.state = field(x + [c]);
                        }
                    }
                    escapedfield(x) {
                        if c == '"' {
                            st.state = inquote(x);
                        } else {
                            st.state = field(x + [c]);
                        }
                    }
                    inquote(x) {
                        if c == '"' {
                            st.state = escapedfield(x + ['"']);
                        } else {
                            st.state = escapeend(x);
                        }
                    }
                    escapeend(x) {
                        // swallow odd chars, eg. space between field and "
                        if c == ',' {
                            st.state = start;
                            row += [str::from_chars(x)];
                        }
                    }
                }
            }
            ret result::ok(row);
        }
    }

    let st = { f: f, delim: delim, quote: quote, has_header: has_header, mutable buf: [], mutable offset: 0u, mutable state: start };
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
    while true {
        let res = r.read_row();
        if result::failure(res) {
            break;
        }
        for field in result::get(res) {
            io::println("FIELD: " + field);
        }
    }
}

