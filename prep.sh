#!/bin/sh
FILE=$1
# teeo is just `tee output.log`
PAGER=teeo script -efqO /dev/null -c "glow $FILE -s dark -w 60 -p"
sed -i -E 's/\x1B\[48;5;203m//g' output.log
mv output.log root/$FILE
