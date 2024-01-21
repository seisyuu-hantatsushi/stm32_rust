# HOW to debug by openocd
## STM32F4-Discovery

## How to connet rtt-target by openocd
1. finding address of RTT control block in map file.
```
20000000 20000000       30     4         /home/kaz/work/github/seisyuu-hantatsushi/stm32_rust/stm32f4-discovery-ledblink/target/thumbv7em-none-eabihf/debug/deps/stm32f4_discovery_ledblink-fe73a9237ef1d14e.47p8xrd83xyr6qyu.rcgu.o:(.bss._SEGGER_RTT)
20000000 20000000       30     1                 _SEGGER_RTT
20000030 20000030      400     1         /home/kaz/work/github/seisyuu-hantatsushi/stm32_rust/stm32f4-discovery-ledblink/target/thumbv7em-none-eabihf/debug/deps/stm32f4_discovery_ledblink-fe73a9237ef1d14e.47p8xrd83xyr6qyu.rcgu.o:(.bss._ZN26stm32f4_discovery_ledblink18__cortex_m_rt_main19_RTT_CHANNEL_BUFFER17h5dc7f13b42457e82E)
20000030 20000030      400     1                 stm32f4_discovery_ledblink::__cortex_m_rt_main::_RTT_CHANNEL_BUFFER::h5dc7f13b42457e82

```
2. rtt server starts at openocd.
- conneting openocd console by telnet.
```
$ telnet localhost 4444
Trying 127.0.0.1...
Connected to localhost.
Escape character is '^]'.
Open On-Chip Debugger
```
