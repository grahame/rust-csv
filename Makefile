all: libcsv.stamp examples/bench

RUSTC=rustc
RUSTARGS=-O -L rust-csv/ -L .

libcsv.stamp: csv.rc csv.rs
	$(RUSTC) $(RUSTARGS) $< && touch $@

examples/bench: examples/bench.rs libcsv.stamp
	$(RUSTC) $(RUSTARGS) $<

check:
	$(RUSTC) --test csv.rc && ./csv

clean:
	rm -f *.stamp *.dylib csv bench
	rm -rf *.dSYM

