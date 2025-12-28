# Overoptimized TCP/UDP echo benchmarker

So I had an issue for my trio framework where tool I used `bombardier` was not preformat enough for benchmarking. So I created this tool to well try making the most preformant echo tool.

### How do I use it?

You need to build a server that does same thing as one in examples.

### Why use it over my X tool?

Its just has way less overhead than standard benchmarking tools. I wrote all state machines that you need to do io_uring by paw.

### Why?

Its designed to test latency/speed of a runtime, yes TCP/UDP echo (in large amounts) is a very IO expensive task but not commonalty expensive.

### What platforms can it run on?

Every as long as they have io_uring (so only Linux)

### Usage:

``
cargo r -r -c <num connections> -p udp/tcp -t <time in seconds> address:port 
``