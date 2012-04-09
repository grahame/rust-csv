#!/bin/bash

rustc -O ../csv.rc
rustc -L .. bench.rs

