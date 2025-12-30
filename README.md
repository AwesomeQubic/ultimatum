# Ultimatum

Benchmarker that does not drag your framework down.

Recently I was working on a io-uring runtime (will publish once its ready) that works at the limit of what is possible and the issue I encountered is that there are no tools to measure performance at that speed.
My code was using 5% of the CPU while bombardier (a tool I used) was using 20% rest being spend doing networking.

### Usage:

You need to build a server like one in examples then run the benchmarker.

For how to use the benchmarker itself please refer to src/usage.txt

### Recommendations

Ideally you want to use TCP, UDP is supported but it can be more quirky.
Please also keep in mind that this tool is less fancy than what you might have gotten used to, the reason for it is that any of fancies just causes it to slow down.