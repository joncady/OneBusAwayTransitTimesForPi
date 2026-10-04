#!/bin/sh

# LXDE can start this before its X session is fully ready; wait, then disable
# both the X screen saver and monitor power management.
sleep 8
export DISPLAY="${DISPLAY:-:0}"
export XAUTHORITY="${XAUTHORITY:-/home/pi/.Xauthority}"
xset s off
xset -dpms
