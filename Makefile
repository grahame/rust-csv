all: csv csvtest

clean:
	rm -f csv

csv: csv.rc
	rustc --lib -O $<

csvtest: csv.rc csvdump.rs
	rustc -L . csvdump.rs

