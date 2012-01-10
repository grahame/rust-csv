all: csv

clean:
	rm -f csv

csv: csv.rs
	rustc -O csv.rs
