use std;
import std::io;
import std::io::{writer_util, reader_util};
import std::map;
import result;

export rowreader, rowaccess, rowiter,
       new_reader, new_reader_readlen,
       hashmap_iter, hashmap_iter_full;

enum state {
    fieldstart(bool),
    infield(uint, uint),
    inescapedfield(uint, uint),
    inquote(uint, uint)
}

type rowreader = {
    readlen: uint,
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

type bufferdescr = {
    escaped: bool,
    buffers: [@[char]],
    start: uint,
    end: uint
};

enum fieldtype {
    emptyfield(),
    bufferfield(bufferdescr)
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
    {
        readlen: 1024u,
        delim: delim,
        quote: quote,
        f: f,
        mutable offset : 0u,
        mutable buffers : [],
        mutable state : fieldstart(false)
    }
}

fn new_reader_readlen(+f: io::reader, +delim: char, +quote: char, rl: uint) -> rowreader {
    {
        readlen: rl,
        delim: delim,
        quote: quote,
        f: f,
        mutable offset : 0u,
        mutable buffers : [],
        mutable state : fieldstart(false)
    }
}

impl of rowaccess for row {
    fn len() -> uint {
        vec::len(self.fields)
    }
    fn getchars(field: uint) -> [char] {
        fn unescape(escaped: [char]) -> [char] {
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
            bufferfield(desc) {
                let buf = [];
                { 
                    let i = 0u;
                    while i < vec::len(desc.buffers) {
                        let from = if (i == 0u)
                            { desc.start } else { 0u };
                        let to = if (i == vec::len(desc.buffers) - 1u)
                            { desc.end } else { vec::len(*desc.buffers[i]) };
                        buf += vec::slice(*desc.buffers[i], from, to);
                        i = i + 1u;
                    }
                }
                if field == self.len() - 1u {
                    // there may be a trailing \r on the last field; we should strip it
                    // if so. bodgy here but seems the most efficient place to deal with this
                    if vec::len(buf) > 0u {
                        if buf[vec::len(buf)-1u] == '\r' {
                            buf = vec::slice(buf, 0u, vec::len(buf)-1u);
                        }
                    }
                }
                if desc.escaped {
                    buf = unescape(buf);
                }
                ret buf;
            }
        };
    }
    fn getstr(field: uint) -> str {
        ret str::from_chars(self.getchars(field));
    }
    fn getall() -> [str] {
        let a = [];
        self.map() { |s| 
            a += [s];
        }
        ret a;
    }
    fn map(f: fn(s: str)) {
        let i = 0u;
        let len = self.len();
        while i < len {
            f(self.getstr(i));
            i += 1u;
        }
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
                    if so > vec::len(*self.buffers[sb]) {
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
                bufferfield({ escaped: escaped, buffers: vec::slice(self.buffers, sb, eb), start: so, end: eo })
            }
            fn statestr(state: state) -> str {
                alt state {
                    fieldstart(after_delim) {
                        #fmt("fieldstart : after_delim %b", after_delim)
                    }
                    infield(b,o) { 
                        #fmt("field : %u %u", b, o)
                    }
                    inescapedfield(b, o) {
                        #fmt("inescapedfield : %u %u", b, o)
                    }
                    inquote(b, o) {
                        #fmt("inquote : %u %u", b, o)
                    }
                }
            }
            let cbuffer = vec::len(self.buffers) - 1u;
            let buf: @[char] = self.buffers[cbuffer];
            while self.offset < vec::len(*buf) {
                let coffset = self.offset;
                let c : char = buf[coffset];
                //io::println(#fmt("-> %c | %s", c, statestr(self.state)));
                self.offset += 1u;
                alt self.state {
                    fieldstart(after_delim) {
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
                        if c == '\n' {
                            fields += [new_bufferfield(self, false, b, o, coffset)];
                            ret true;
                        } else if c == self.delim {
                            self.state = fieldstart(true);
                            fields += [new_bufferfield(self, false, b, o, coffset)];
                        }
                    }
                    inescapedfield(b, o) {
                        if c == self.quote {
                            self.state = inquote(b, o);
                        }
                        else if c == self.delim {
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
                //io::println(#fmt(".... %s", statestr(self.state)));
            }
            ret false;
        }

        self.state = fieldstart(false);
        let do_read = vec::len(self.buffers) == 0u;
        let fields = [];
        while true {
            if do_read {
                let data: @[char] = @self.f.read_chars(self.readlen);
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

fn hashmap_iter_cols(r: rowreader, cols: [str], f: fn(map::hashmap<str, str>)) {
    while true {
        let res = r.readrow();
        if result::failure(res) {
            break;
        }
        let m : map::hashmap<str, str> = map::new_str_hash();
        let col = 0u;
        let row = result::get(res);
        if row.len() != vec::len(cols) {
            cont; // FIXME: how to flag that we dropped a crazy row?
        }
        result::get(res).map() { |s|
            m.insert(cols[col], s);
            col += 1u;
        };
        f(m);
    }
}

// reads the first row as a header, to derive keys for a hashmap
// emitted for each subsequent row
fn hashmap_iter(r: rowreader, f: fn(map::hashmap<str, str>)) {
    let res = r.readrow();
    alt res {
        result::ok(row) {
            hashmap_iter_cols(r, result::get(res).getall(), f);
        }
        result::err(_) { }
    }
}

// as hashmap_iter, but first apply 'hc' to each header; allows
// cleaning up headers; also allows verification that heads are 
// satisfactory
fn hashmap_iter_full(r: rowreader, hmap: fn(&&h: str) -> str, hver: fn(cols: [str]) -> bool, f: fn(map::hashmap<str, str>)) {
    let res = r.readrow();
    alt res {
        result::ok(row) {
            let cols : [str] = vec::map(result::get(res).getall(), hmap);
            if !hver(cols) {
                ret;
            }
            hashmap_iter_cols(r, cols, f);
        }
        result::err(_) { }
    }
}

#[cfg(test)]
mod test {
    fn rowmatch(testdata: str, expected: [[str]]) {
        let chk = fn@(mk: fn(io::reader) -> rowreader) {
            let f = io::str_reader(testdata);
            let r = mk(f);
            let i = 0u;
            while true {
                let res = r.readrow();
                if result::failure(res) {
                    break;
                }
                let row = result::get(res);
                let expect = expected[i];

                assert(row.len() == vec::len(expect));
                let j = 0u;
                while j < row.len() {
                    assert(row.getstr(j) == expect[j]);
                    j += 1u;
                }
                i += 1u;
            }
            assert(i == vec::len(expected));
        };
        let runchecks = fn@(testdata: str) {
            // test default reader params
            chk() { |inp|
                new_reader_readlen(inp, ',', '"', 2u)
            };
            // test continuations over read buffers
            let j = 1u;
            while j < str::len(testdata) {
                chk() { |inp|
                    new_reader_readlen(inp, ',', '"', j)
                };
                j += 1u;
            }
        };
        runchecks(testdata);
        runchecks(str::replace(testdata, "\n", "\r\n"));
    }

    #[test]
    fn test_simple() {
        rowmatch("a,b,c,d\n1,2,3,4\n",
                 [["a", "b", "c", "d"], ["1", "2", "3", "4"]]);
    }

    #[test]
    fn test_trailing_comma() {
        rowmatch("a,b,c,d\n1,2,3,4,\n",
                 [["a", "b", "c", "d"], ["1", "2", "3", "4", ""]]);
    }

    #[test]
    fn test_leading_comma() {
        rowmatch("a,b,c,d\n,1,2,3,4\n",
                 [["a", "b", "c", "d"], ["", "1", "2", "3", "4"]]);
    }

    #[test]
    fn test_quote_simple() {
        rowmatch("\"Hello\",\"There\"\na,b,\"c\",d\n",
                 [["Hello", "There"], ["a", "b", "c", "d"]]);
    }

    #[test]
    fn test_quote_nested() {
        rowmatch("\"Hello\",\"There is a \"\"fly\"\" in my soup\"\na,b,\"c\",d\n",
                 [["Hello", "There is a \"fly\" in my soup"], ["a", "b", "c", "d"]]);
    }

    #[test]
    fn test_quote_with_comma() {
        rowmatch("\"1,2\"\n",
                 [["1,2"]])
    }

    #[test]
    fn test_quote_with_other_comma() {
        rowmatch("1,2,3,\"a,b,c\"",
                 [["1", "2", "3", "a,b,c"]])
    }

    #[test]
    fn test_blank_line() {
        rowmatch("\n\n", [[], []]);
    }
}

