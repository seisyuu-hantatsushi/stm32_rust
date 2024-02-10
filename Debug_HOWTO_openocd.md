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
  The control block of RTT is located at address 0x20000000 and its size is 1072 bytes.

2. starting rtt server at openocd.
- connecting openocd console by telnet.
  ```
  $ telnet localhost 4444
  Trying 127.0.0.1...
  Connected to localhost.
  Escape character is '^]'.
  Open On-Chip Debugger
  >
  ```
- running program. (by gdb etc.)
- starting rtt server by openocd console
  ```
  > rtt setup 0x20000000 1072 "SEGGER RTT"
  > rtt stop
  > rtt start
  rtt: Searching for control block 'SEGGER RTT'
  rtt: Control block found at 0x20000000
  > rtt channels
  Channels: up=1, down=0
  Up-channels:
  0: Terminal 1024 2
  Down-channels:
  > rtt server
    rtt server
      rtt server start <port> <channel> [message]
      rtt server stop <port>
  > rtt server start 5000 0 
  Listening on port 5000 for rtt connections
  ```
3. connecting rtt server.
  ```
  $ telnet localhost 5000
  Trying 127.0.0.1...
  Connected to localhost.
  Escape character is '^]'.
  hello from RTT
  ```
