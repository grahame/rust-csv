all: libcsv.stamp

RUSTC=rustc
RUSTARGS=-O -L rust-csv/ -L .

libcsv.stamp: csv.rc csv.rs
	$(RUSTC) $(RUSTARGS) $< && touch $@

check:
	$(RUSTC) --test csv.rc && ./csv

clean:
	rm -f *.stamp csv
	rm -rf *.dSYM

