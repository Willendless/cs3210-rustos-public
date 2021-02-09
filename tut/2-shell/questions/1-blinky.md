In assignment 1-blinky, you enabled GPIO pin 16 as an output and then repeatedly set and cleared it by writing to registers GPFSEL1, GPSET0, and GPCLR0. Which three registers would you write to to do the same for GPIO pin 27? Which physical pin on the Raspberry Pi maps to GPIO pin 27?

使用GPIO pin 27的话，需要GPFSEL2，GPSET0和GPCLR0.物理引脚13对应GPIO引脚27.
