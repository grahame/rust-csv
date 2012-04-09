#!/usr/bin/env python3

import csv, sys

reader = csv.reader(open(sys.argv[1]))
nl =0 
for row in reader: nl += len(row)
print("read %d rows" % nl)
