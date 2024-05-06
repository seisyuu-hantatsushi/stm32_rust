# STM32H745/STM32H755 dual core example

## Checked Boards
- NUCLEO-H755ZI-Q(MB1363)

## request tools.
- openocd 0.12.0

## How to build.
run "cargo build" at "cm4_ledblink" and "cm7_ledblink" directories.

## How to run. (example for debug build binary)
1. open 3 terminal.
   - Start openocd in one terminal, the other two go to cm4_ledblink, cm7_ledblink respectively.
2. connect gdb to cm7 and write binary in flash.
   1. move to cm7_ledblink.
   2. "arm-none-eabi-gdb target/thumbv7em-none-eabihf/debug/cm7_ledblink"
   3. connect gdb to cm7 and load binary as follows.
   ```
   $ arm-none-eabi-gdb target/thumbv7em-none-eabihf/debug/cm7_ledblink
   GNU gdb (Arm GNU Toolchain 11.3.Rel1) 12.1.90.20220802-git
   Copyright (C) 2022 Free Software Foundation, Inc.
   License GPLv3+: GNU GPL version 3 or later <http://gnu.org/licenses/gpl.html>
   This is free software: you are free to change and redistribute it.
   There is NO WARRANTY, to the extent permitted by law.
   Type "show copying" and "show warranty" for details.
   This GDB was configured as "--host=x86_64-pc-linux-gnu --target=arm-none-eabi".
   Type "show configuration" for configuration details.
   For bug reporting instructions, please see:
   <https://bugs.linaro.org/>.
   Find the GDB manual and other documentation resources online at:
       <http://www.gnu.org/software/gdb/documentation/>.

   For help, type "help".
   Type "apropos word" to search for commands related to "word"...
   Reading symbols from target/thumbv7em-none-eabihf/debug/cm7_ledblink...
   (gdb) target remote localhost:3333
   Remote debugging using localhost:3333
   cortex_m::peripheral::SYST::has_wrapped (self=0x2001fec0)
       at /home/kaz/.cargo/registry/src/index.crates.io-6f17d22bba15001f/cortex-m-0.7.7/src/peripheral/syst.rs:134
   134	    pub fn has_wrapped(&mut self) -> bool {
   (gdb) monitor reset init
   [stm32h7x.cpu0] halted due to debug-request, current mode: Thread 
   xPSR: 0x01000000 pc: 0x08000298 msp: 0x20020000
   [stm32h7x.cpu1] halted due to debug-request, current mode: Thread 
   xPSR: 0x01000000 pc: 0x08100298 msp: 0x30020000
   Unable to match requested speed 4000 kHz, using 3300 kHz
   Unable to match requested speed 4000 kHz, using 3300 kHz
   (gdb) load
   Loading section .vector_table, size 0x298 lma 0x8000000
   Loading section .text, size 0x12ec8 lma 0x8000298
   Loading section .rodata, size 0x2038 lma 0x8013160
   Loading section .data, size 0x8 lma 0x8015198
   Start address 0x08000298, load size 86432
   Transfer rate: 39 KB/sec, 10804 bytes/write.
   ```
3. connect gdb to cm4 and write binary in flash.
   - Same procedure as CM7, but the connection port is 3334.
4. start cm7 and cm4 by gdb continue command.

## Memory Map
|Memory|Size|Address|Cortex-M7|Cortex-M4|Note|
|:-|:-|:-|:-|:-|:-|
|DTCM|0x2000_0000-0x2001_FFFF|128KiB|Stack|Don't use||
|AXI SRAM|0x2400_0000-0x2407_FFFF|512KiB|Shared|Shared||
|SRAM1|0x3000_0000-0x3001_FFFF|128KiB|Don't use|Stack||

## Boot Sequence
```
                  CM7                      CM4
power supply-------+------------------------+
                   |                        |
          waiting CM4 enters Stop mode   Enable HSEM
                   |                     Enable interrupt of HSEM
                   |                     Unmask EXTI of hsem_int2_it
                   |                     CM4 enters Stop mode
                   +  <---------------------+
          Setup RCC                         |
          take and release HSEM             |
          interrupt hsem_int2_it ---------> +
          Enter main                     Enter main
```
