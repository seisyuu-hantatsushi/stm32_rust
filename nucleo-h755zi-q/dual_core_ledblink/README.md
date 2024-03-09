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
                   |                     wait HSEM flag
                   |                     CM4 enters Stop mode
                   +  <---------------------+
          Setup RCC                         |
          set HSEM Flag ------------------> +
          Enter main                     Enter main
```
