#!/bin/bash
# Linux guest: Chrome for Testing as Xaqu starts it on --nvidia, on the same WebGL page.
export HOME=/home/xaqu XDG_RUNTIME_DIR=/run/user/1000 WAYLAND_DISPLAY=wayland-1
pkill -f chrome-window/chrome; sleep 1; rm -rf /dev/shm/chrome
setsid /home/xaqu/chrome/chrome-window/chrome --no-sandbox --ozone-platform=wayland \
  --user-data-dir=/dev/shm/chrome --no-first-run --use-angle=vulkan \
  --enable-features=DefaultANGLEVulkan --ignore-gpu-blocklist --enable-gpu-rasterization \
  --disable-crash-reporter --disable-breakpad --enable-logging=stderr --disable-infobars \
  --autoplay-policy=no-user-gesture-required file:///home/xaqu/chrome/nvidia-webgl.html \
  > /home/xaqu/lb/chrome.log 2>&1 < /dev/null &
