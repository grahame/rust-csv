use std;
import io::{writer_util, reader_util};
import std::map;
import map::hashmap;
import result;

export rowreader, rowiter,
       new_reader, new_reader_readlen,
       hashmap_iter, hashmap_iter_full;

enum state {
    fieldstart(bool),
    infield(uint, uint),
    inquotedfield(uint, uint),
    inquote(uint, uint)
}

type rowreader = {
    readlen: uint,
    delim: char,
    quote: char,
    f : io::reader,
    mut offset : uint,
    mut buffers : [[char]],
    mut state : state,
    mut trailing_nl : bool,
    mut terminating : bool
};

type row = {
    fields : [ fieldtype ]
};

type bufferdescr = {
    escaped: bool,
    sb: uint,
    eb: uint,
    start: uint,
    end: uint
};

enum fieldtype {
    emptyfield(),
    bufferfield(bufferdescr)
}

iface rowiter {
    fn readrow(&row: [str]) -> bool;
}

fn new_reader(+f: io::reader, +delim: char, +quote: char) -> rowreader {
    {
        new_reader_readlen(f, delim, quote, 1024u)
    }
}

fn new_reader_readlen(+f: io::reader, +delim: char, +quote: char, rl: uint) -> rowreader {
    {
        readlen: rl,
        delim: delim,
        quote: quote,
        f: f,
        mut offset : 0u,
        mut buffers : [],
        mut state : fieldstart(false),
        mut trailing_nl : false,
        mut terminating: false
    }
}

impl of rowiter for rowreader {
    fn readrow(&row: [str]) -> bool {
        fn unescape(escaped: [char]) -> [char] {
            let mut r : [char] = [];
            vec::reserve(r, vec::len(escaped));
            let mut in_q = false;
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
        fn statestr(state: state) -> str {
            alt state {
                fieldstart(after_delim) {
                    #fmt("fieldstart : after_delim %b", after_delim)
                }
                infield(b,o) { 
                    #fmt("field : %u %u", b, o)
                }
                inquotedfield(b, o) {
                    #fmt("inquotedfield : %u %u", b, o)
                }
                inquote(b, o) {
                    #fmt("inquote : %u %u", b, o)
                }
            }
        }
        fn row_from_buf(self: rowreader, &fields: [fieldtype]) -> bool {
            fn new_bufferfield(self: rowreader, escaped: bool, sb: uint, so: uint, eo: uint) -> fieldtype {
                let mut eb = vec::len(self.buffers) - 1u;
                let mut sb = sb, so = so, eo = eo;
                if escaped {
                    so += 1u;
                    if so > vec::len(self.buffers[sb]) {
                        sb += 1u;
                        so = vec::len(self.buffers[sb]) - 1u;
                    }
                    if eo > 0u {
                        eo -= 1u;
                    } else {
                        eb -= 1u;
                        eo = vec::len(self.buffers[eb]) - 1u;
                    }
                }
                bufferfield({ escaped: escaped, sb: sb, eb: eb, start: so, end: eo })
            }
            let cbuffer = vec::len(self.buffers) - 1u;
            let buf = self.buffers[cbuffer];
            while self.offset < vec::len(buf) {
                let coffset = self.offset;
                let c : char = buf[coffset];
                #debug("got '%c' | %s", c, statestr(self.state));
                self.offset += 1u;
                alt self.state {
                    fieldstart(after_delim) {
                        #debug("fieldstart : after_delim %b", after_delim);
                        if c == self.quote {
                            self.state = inquotedfield(cbuffer, coffset);
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
                        #debug("field : %u %u", b, o);
                        if c == '\n' {
                            fields += [new_bufferfield(self, false, b, o, coffset)];
                            ret true;
                        } else if c == self.delim {
                            self.state = fieldstart(true);
                            fields += [new_bufferfield(self, false, b, o, coffset)];
                        }
                    }
                    inquotedfield(b, o) {
                        #debug("inquotedfield : %u %u", b, o);
                        if c == self.quote {
                            self.state = inquote(b, o);
                        }
                    }
                    inquote(b, o) {
                        #debug("inquote : %u %u", b, o);
                        if c == '\n' {
                            fields += [new_bufferfield(self, true, b, o, coffset)];
                            ret true;
                        } else if c == self.quote {
                            self.state = inquotedfield(b, o);
                        } else if c == self.delim {
                            self.state = fieldstart(true);
                            fields += [new_bufferfield(self, true, b, o, coffset)];
                        }
                        // swallow odd chars, eg. space between field and "
                    }
                }
                #debug("now %s", statestr(self.state));
            }
            ret false;
        }
        self.state = fieldstart(false);
        let mut do_read = vec::len(self.buffers) == 0u;
        let mut fields = [];

        while !self.terminating {
            if do_read {
                let mut data = self.f.read_chars(self.readlen);
                //log(error, ("aa", str::from_chars(data)));
                if vec::len(data) == 0u {
                    if !self.trailing_nl {
                        self.terminating = true;
                        data = ['\n'];
                    } else {
                        ret false;
                    }
                }
                // this is horrible, but it avoids the whole parser needing 
                // to know about \r.
                data = vec::filter(data) { |c| c != '\r' };
                let data_len = vec::len(data);
                if data_len == 0u {
                    cont;
                }
                //log(error, ("here", str::from_chars(data)));
                self.trailing_nl = data[data_len - 1u] == '\n';
                self.buffers += [data];
                self.offset = 0u;
            }

            if row_from_buf(self, fields) {
                let l = vec::len(fields);
                vec::reserve(row, l);
                row = vec::map(fields) { |field| 
                    alt field {
                        emptyfield() { "" }
                        bufferfield(desc) {
                            let mut buf = [];
                            { 
                                let mut i = desc.sb;
                                while i <= desc.eb {
                                    let from = if (i == desc.sb)
                                        { desc.start } else { 0u };
                                    let to = if (i == desc.eb)
                                        { desc.end } else { vec::len(self.buffers[i]) };
                                    buf += vec::slice(self.buffers[i], from, to);
                                    i = i + 1u;
                                }
                            }
                            if desc.escaped {
                                buf = unescape(buf);
                            }
                            str::from_chars(buf)
                        }
                    }
                };
                if vec::len(self.buffers) > 1u {
                    self.buffers = vec::slice(self.buffers, vec::len(self.buffers) - 1u, vec::len(self.buffers));
                }
                fields = [];
                ret true;
            }
            do_read = true;
        }
        ret false;
    }
}

fn hashmap_iter_cols(r: rowreader, cols: [str], f: fn(map::hashmap<str, str>)) {
    let mut fields : [str] = [];
    // can reuse, we're just shoving new vals in same cols..
    let m : map::hashmap<str, str> = map::str_hash();
    let ncols = vec::len(cols);
    while r.readrow(fields) {
        if vec::len(fields) != ncols {
            cont; // FIXME: how to flag that we dropped a crazy row?
        }
        let mut col = 0u;
        vec::iter(fields) { |s|
            m.insert(cols[col], s);
            col += 1u;
        };
        f(m);
    }
}

// reads the first row as a header, to derive keys for a hashmap
// emitted for each subsequent row
fn hashmap_iter(r: rowreader, f: fn(map::hashmap<str, str>)) {
    let mut row: [str] = [];
    if r.readrow(row) {
        hashmap_iter_cols(r, row, f);
    }
}

// as hashmap_iter, but first apply 'hc' to each header; allows
// cleaning up headers; also allows verification that heads are 
// satisfactory
fn hashmap_iter_full(r: rowreader, hmap: fn(&&h: str) -> str, hver: fn(cols: [str]) -> bool, f: fn(map::hashmap<str, str>)) {
    let mut row: [str] = [];
    if r.readrow(row) {
        let cols : [str] = vec::map(row, hmap);
        if !hver(cols) {
            ret;
        }
        hashmap_iter_cols(r, cols, f);
    }
}

#[cfg(test)]
mod test {
    fn rowmatch(testdata: str, expected: [[str]]) {
        let chk = fn@(s: str, mk: fn(io::reader) -> rowreader) {
            let f = io::str_reader(s);
            let r = mk(f);
            let mut i = 0u;
            loop {
                let mut row: [str] = [];
                if !r.readrow(row) {
                    break;
                }
                let expect = expected[i];
                assert(vec::len(row) == vec::len(expect));
                let mut j = 0u;
                while j < row.len() {
                    assert(row[j] == expect[j]);
                    j += 1u;
                }
                i += 1u;
            }
            assert(i == vec::len(expected));
        };
        let runchecks = fn@(s: str) {
            // test default reader params
            chk(s) { |inp|
                new_reader_readlen(inp, ',', '"', 2u)
            };
            // test default constructor
            chk(s) { |inp|
                new_reader(inp, ',', '"')
            };
            // test continuations over read buffers
            let mut j = 1u;
            while j < str::len(s) {
                chk(s) { |inp|
                    new_reader_readlen(inp, ',', '"', j)
                };
                j += 1u;
            }
            ret;
        };
        // so we can test trailing newline case, testdata
        // must not end in \n - leave off the last newline
        runchecks(testdata);
        runchecks(str::replace(testdata, "\n", "\r\n"));
        if !str::ends_with(testdata, "\n") {
            runchecks(testdata+"\n");
            runchecks(str::replace(testdata+"\n", "\n", "\r\n"));
        }
    }

    #[test]
    fn test_simple() {
        rowmatch("a,b,c,d\n1,2,3,4",
                 [["a", "b", "c", "d"], ["1", "2", "3", "4"]]);
    }

    #[test]
    fn test_trailing_comma() {
        rowmatch("a,b,c,d\n1,2,3,4,",
                 [["a", "b", "c", "d"], ["1", "2", "3", "4", ""]]);
    }

    #[test]
    fn test_leading_comma() {
        rowmatch("a,b,c,d\n,1,2,3,4",
                 [["a", "b", "c", "d"], ["", "1", "2", "3", "4"]]);
    }

    #[test]
    fn test_quote_simple() {
        rowmatch("\"Hello\",\"There\"\na,b,\"c\",d",
                 [["Hello", "There"], ["a", "b", "c", "d"]]);
    }

    #[test]
    fn test_quote_nested() {
        rowmatch("\"Hello\",\"There is a \"\"fly\"\" in my soup\"\na,b,\"c\",d",
                 [["Hello", "There is a \"fly\" in my soup"], ["a", "b", "c", "d"]]);
    }

    #[test]
    fn test_quote_with_comma() {
        rowmatch("\"1,2\"",
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

