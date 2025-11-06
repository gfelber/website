```sh
#!/bin/sh

printenv FLAG > flag
unset FLAG

exec ./mesarthim_deploy < $INPUT_FILE > $OUTPUT_FILE
```
