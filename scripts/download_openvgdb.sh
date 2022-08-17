#!/bin/sh
# OpenVGDB download script
url='https://github.com/OpenVGDB/OpenVGDB/releases/download/v29.0/openvgdb.zip'
filename='openvgdb.zip'

# wget and unzip are required
which wget || exit 1
which unzip || exit 1

wget "$url" -O "$filename"
unzip "$filename" openvgdb.sqlite
rm "$filename"
