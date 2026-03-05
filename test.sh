#!/usr/bin/env bash

export ROPT_SESSION=$(ropt begin)

ropt push select --name "Choose and thing"
   ropt append option --value "Thing 1"
   ropt append option --value "Thing 2"
   ropt append option --value "Thing 3"
ropt pop

ropt show

output=$(ropt execute)

echo "Output: $output"


