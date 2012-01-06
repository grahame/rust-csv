all: csv

csv: csv.rs
	rustc csv.rs
