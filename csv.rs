
use std;
import std::io;
import std::io::{writer_util, reader_util};
import result;

tag state {
    fieldstart(bool);
    infield(uint, uint);
    inescapedfield(uint, uint);
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
    fields : [ fieldtype ]
};

type field = {
    escaped: bool,
    buffers: [@[char]],
    start: uint, // offset into first buffer
    end: uint // offset into last buffer
};

enum fieldtype {
    bufferfield(field);
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
        mutable state : fieldstart(false)
    };
    ret r;
}

impl of rowaccess for row {
    fn len() -> uint {
        vec::len(self.fields)
    }
    fn getchars(field: uint) -> [char] {
        fn extract_field(field: field, &r: [char]) {
            let i = 0u;
            while i < vec::len(field.buffers) {
                let from = i == 0u ? field.start : 0u;
                let to = (i == vec::len(field.buffers) - 1u) ? 
                         field.end : vec::len(*field.buffers[i]);
                r += vec::slice(*field.buffers[i], from, to);
                i = i + 1u;
            }
        }
        fn unescape(escaped: [char]) -> [char] {
            io::println("unescape");
            let r : [char] = [];
            vec::reserve(r, vec::len(escaped));
            let in_q = false;
            for c in escaped { 
                if in_q { 
                    assert(c == '"');
                    in_q = false;
                } else {
                    in_q = c == '"';
                    r += [c];
                }
            }
            ret r;
        }
        alt self.fields[field] {
            emptyfield() { ret []; }
            bufferfield(field) {
                let buf = [];
                extract_field(field, buf);
                if field.escaped {
                    buf = unescape(buf);
                }
                ret buf;
            }
        };
    }
    fn getstr(field: uint) -> str {
        ret str::from_chars(self.getchars(field));
    }
}

impl of rowiter for rowreader {
    fn readrow() -> result::t<row, str> {
        fn row_from_buf(self: rowreader, &fields: [fieldtype]) -> bool {
            fn new_bufferfield(self: rowreader, escaped: bool, sb: uint, so: uint, eo: uint) -> fieldtype {
                let eb = vec::len(self.buffers);
                let sb = sb, so = so, eo = eo;
                if escaped {
                    so += 1u;
                    if so >= vec::len(*self.buffers[sb]) {
                        sb += 1u;
                        so = vec::len(*self.buffers[sb]) - 1u;
                    }
                    if eo > 0u {
                        eo -= 1u;
                    } else {
                        eb -= 1u;
                        eo = vec::len(*self.buffers[sb]) - 1u;
                    }
                }
                bufferfield({escaped: escaped, buffers: vec::slice(self.buffers, sb, eb), 
                    start: so, end: eo})
            }
            let cbuffer = vec::len(self.buffers) - 1u;
            let buf: @[char] = self.buffers[cbuffer];
            while self.offset < vec::len(*buf) {
                let coffset = self.offset;
                let c : char = buf[coffset];
                self.offset += 1u;
                alt self.state {
                    fieldstart(after_delim) {
                        //io::println(#fmt("fieldstart : after_delim %b", after_delim));
                        if c == self.quote {
                            self.state = inescapedfield(cbuffer, coffset);
                        } else if c == '\n' {
                            if after_delim {
                                fields += [emptyfield];
                            }
                            ret true;
                        } else if c == self.delim {
                            self.state = fieldstart(true);
                            fields += [emptyfield];
                        } else {
                            self.state = infield(cbuffer, coffset);
                        }
                    }
                    infield(b,o) {
                        //io::println(#fmt("field : %u %u", b, o));
                        if c == '\n' {
                            fields += [new_bufferfield(self, false, b, o, coffset)];
                            ret true;
                        } else if c == self.delim {
                            self.state = fieldstart(true);
                            fields += [new_bufferfield(self, false, b, o, coffset)];
                        }
                    }
                    inescapedfield(b, o) {
                        //io::println(#fmt("inescapedfield : %u %u", b, o));
                        if c == self.quote {
                            self.state = inquote(b, o);
                        } else if c == self.delim {
                            self.state = fieldstart(true);
                            fields += [new_bufferfield(self, true, b, o, coffset)];
                        }
                    }
                    inquote(b, o) {
                        //io::println(#fmt("inquote : %u %u", b, o));
                        if c == '\n' {
                            fields += [new_bufferfield(self, true, b, o, coffset)];
                            ret true;
                        } else if c == self.quote {
                            self.state = inescapedfield(b, o);
                        } else if c == self.delim {
                            self.state = fieldstart(true);
                            fields += [new_bufferfield(self, true, b, o, coffset)];
                        }
                        // swallow odd chars, eg. space between field and "
                    }
                }
            }
            ret false;
        }

        self.state = fieldstart(false);
        let do_read = vec::len(self.buffers) == 0u;
        let fields = [];
        while true {
            if do_read {
                let data: @[char] = @self.f.read_chars(1u);
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

