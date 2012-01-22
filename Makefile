all: libcsv csvtest

clean:
	rm -f csv

libcsv: csv.rc
	rustc --lib -O $<

csvtest: csv.rc csvdump.rs
	rustc -L . csvdump.rs

