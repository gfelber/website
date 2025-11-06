#!/bin/sh
FILE=$1
# teeo is just `tee output.log`
PAGER=teeo script -efqO /dev/null -c "glow $FILE -s dark -w 60 -p"
mv output.log root/$FILE
