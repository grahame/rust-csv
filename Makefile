all: csv

clean:
	rm -f csv

csv: csv.rc
	rustc --lib -O $<

