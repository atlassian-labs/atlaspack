#!/bin/bash
#
# This file detects the environment and executes 
# the correct script for the platform environment
#
# Linux:   Bash
# MacOS:   Bash
# Windows: PowerShell
#
# The file extension .ps1 will force Windows to use
# PowerShell .Unix will execute it as a bash script

which ping.exe && . $PSScriptRoot\install.ps1 || source $( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )/install.bash
