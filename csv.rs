
use std;
import std::io;
import std::io::{writer_util, reader_util};
import result;

tag state {
    start(bool);
    field(uint, uint);
    escapedfield(uint, uint);
    inquote(uint, uint);
}

type rowreader = {
    delim: char,
    quote: char,
    f : io::reader,
    mutable offset : uint,
    mutable buffers : [@[char]],
    mutable state : state
};

type row = {
    fields : [ field ]
};

enum field {
    bufferfield([@[char]], uint, uint);
    emptyfield();
}

iface rowiter {
    fn readrow() -> result::t<row, str>;
}

iface rowaccess {
    fn len() -> uint;
    fn getchars(uint) -> [char];
    fn getstr(uint) -> str;
}

fn new_reader(+f: io::reader, +delim: char, +quote: char) -> rowreader {
    let r : rowreader = {
        delim: delim,
        quote: quote,
        f: f,
        mutable offset : 0u,
        mutable buffers : [],
        mutable state : start(false)
    };
    ret r;
}

impl of rowaccess for row {
    fn len() -> uint {
        vec::len(self.fields)
    }
    fn getchars(field: uint) -> [char] {
        alt self.fields[field] {
            emptyfield() { ret []; }
            bufferfield(buffers, so, eo) {
                let r : [char] = [];
                let i = 0u;
                while i < vec::len(buffers) {
                    let from = i == 0u ? so : 0u;
                    let to = (i == vec::len(buffers) - 1u) ? eo : vec::len(*buffers[i]);
                    r += vec::slice(*buffers[i], from, to);
                    i = i + 1u;
                }
                ret r;
            }
        };
    }
    fn getstr(field: uint) -> str {
        ret str::from_chars(self.getchars(field));
    }
}

impl of rowiter for rowreader {
    fn readrow() -> result::t<row, str> {
        fn row_from_buf(self: rowreader, &fields: [field]) -> bool {
            fn new_bufferfield(self: rowreader, sb: uint, so: uint, eo: uint) -> field {
                let bufs : [@[char]] = vec::slice(self.buffers, sb, vec::len(self.buffers));
                ret bufferfield(bufs, so, eo);
            }
            let cbuffer = vec::len(self.buffers) - 1u;
            let buf: @[char] = self.buffers[cbuffer];
            while self.offset < vec::len(*buf) {
                let coffset = self.offset;
                let c : char = buf[coffset];
                self.offset += 1u;
                alt self.state {
                    start(after_delim) {
                        //io::println(#fmt("start : after_delim %b", after_delim));
                        if c == self.quote {
                            self.state = escapedfield(cbuffer, coffset);
                        } else if c == '\n' {
                            if after_delim {
                                fields += [emptyfield];
                            }
                            ret true;
                        } else if c == self.delim {
                            self.state = start(true);
                            fields += [emptyfield];
                        } else {
                            self.state = field(cbuffer, coffset);
                        }
                    }
                    field(b,o) {
                        //io::println(#fmt("field : %u %u", b, o));
                        if c == '\n' {
                            fields += [new_bufferfield(self, b, o, coffset)];
                            ret true;
                        } else if c == self.delim {
                            self.state = start(true);
                            fields += [new_bufferfield(self, b, o, coffset)];
                        }
                    }
                    escapedfield(b, o) {
                        //io::println(#fmt("escapedfield : %u %u", b, o));
                        if c == self.quote {
                            self.state = inquote(b, o);
                        } else if c == self.delim {
                            self.state = start(true);
                            fields += [new_bufferfield(self, b, o, coffset)];
                        }
                    }
                    inquote(b, o) {
                        //io::println(#fmt("inquote : %u %u", b, o));
                        if c == '\n' {
                            fields += [new_bufferfield(self, b, o, coffset)];
                            ret true;
                        } else if c == self.quote {
                            // hmm what to do 
                            // self.state = escapedfield(x + [self.quote]);
                        } else if c == self.delim {
                            self.state = start(true);
                            fields += [new_bufferfield(self, b, o, coffset)];
                        }
                        // swallow odd chars, eg. space between field and "
                    }
                }
            }
            ret false;
        }

        self.state = start(false);
        let do_read = vec::len(self.buffers) == 0u;
        let fields = [];
        while true {
            if do_read {
                let data: @[char] = @self.f.read_chars(1024u);
                //io::println(#fmt("len %u '%s'", vec::len(*data), str::from_chars(*data)));
                if vec::len(*data) == 0u {
                    ret result::err("EOF");
                }
                self.buffers += [data];
                self.offset = 0u;
            }

            if row_from_buf(self, fields) {
                let r: row = { fields: fields };
                fields = [];
                if vec::len(self.buffers) > 1u {
                    self.buffers = vec::slice(self.buffers, vec::len(self.buffers) - 1u, vec::len(self.buffers));
                }
                ret result::ok(r);
            }
            do_read = true;
        }
        ret result::err("unreachable");
    }
}

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

