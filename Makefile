all: csv

clean:
	rm -f csv

csv: csv.rs
	rustc csv.rs
