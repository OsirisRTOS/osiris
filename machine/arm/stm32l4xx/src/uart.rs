use crate::HAL_OK;
use crate::bindings::{self};

const LPUART1: *mut bindings::USART_TypeDef =
    bindings::LPUART1_BASE as *mut bindings::USART_TypeDef;
const GPIOG: *mut bindings::GPIO_TypeDef = bindings::GPIOG_BASE as *mut bindings::GPIO_TypeDef;

static mut HLPUART1: bindings::UART_HandleTypeDef =
    unsafe { core::mem::zeroed::<bindings::UART_HandleTypeDef>() };

pub fn lpuart1_init() {
    // Initialize the LPUART1 peripheral, as this is repr C an all zero pattern is valid.
    unsafe {
        HLPUART1.Instance = LPUART1;
        HLPUART1.Init.BaudRate = 115200;
        HLPUART1.Init.WordLength = bindings::UART_WORDLENGTH_8B;
        HLPUART1.Init.StopBits = bindings::UART_STOPBITS_1;
        HLPUART1.Init.Parity = bindings::UART_PARITY_NONE;
        HLPUART1.Init.Mode = bindings::UART_MODE_TX_RX;
        HLPUART1.Init.HwFlowCtl = bindings::UART_HWCONTROL_NONE;
        HLPUART1.Init.OneBitSampling = bindings::UART_ONE_BIT_SAMPLE_DISABLE;
        HLPUART1.Init.ClockPrescaler = bindings::UART_PRESCALER_DIV1;
        HLPUART1.AdvancedInit.AdvFeatureInit = bindings::UART_ADVFEATURE_NO_INIT;
        HLPUART1.FifoMode = bindings::UART_FIFOMODE_DISABLE;
    }

    unsafe {
        if bindings::HAL_UART_Init(&raw mut HLPUART1) != HAL_OK {
            panic!("LPUART1 init failed");
        }

        if bindings::HAL_UARTEx_SetTxFifoThreshold(
            &raw mut HLPUART1,
            bindings::UART_TXFIFO_THRESHOLD_1_8,
        ) != HAL_OK
        {
            panic!("LPUART1 set tx fifo threshold failed");
        }

        if bindings::HAL_UARTEx_SetRxFifoThreshold(
            &raw mut HLPUART1,
            bindings::UART_RXFIFO_THRESHOLD_1_8,
        ) != HAL_OK
        {
            panic!("LPUART1 set rx fifo threshold failed");
        }

        if bindings::HAL_UARTEx_DisableFifoMode(&raw mut HLPUART1) != HAL_OK {
            panic!("LPUART1 disable fifo mode failed");
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HAL_UART_MspInit(handle: *mut bindings::UART_HandleTypeDef) {
    let mut gpio_init = unsafe { core::mem::zeroed::<bindings::GPIO_InitTypeDef>() };
    let mut clk_init = unsafe { core::mem::zeroed::<bindings::RCC_PeriphCLKInitTypeDef>() };

    if unsafe { (*handle).Instance } == LPUART1 {
        clk_init.PeriphClockSelection = bindings::RCC_PERIPHCLK_LPUART1;
        clk_init.Lpuart1ClockSelection = bindings::RCC_LPUART1CLKSOURCE_PCLK1;

        if (unsafe { bindings::HAL_RCCEx_PeriphCLKConfig(&raw mut clk_init) }) != HAL_OK {
            panic!("LPUART1 clock config failed");
        }

        // Configure the GPIO pins for LPUART1
        gpio_init.Pin = bindings::GPIO_PIN_7 | bindings::GPIO_PIN_8;
        gpio_init.Mode = bindings::GPIO_MODE_AF_PP;
        gpio_init.Pull = bindings::GPIO_NOPULL;
        gpio_init.Speed = bindings::GPIO_SPEED_FREQ_VERY_HIGH;
        gpio_init.Alternate = bindings::GPIO_AF8_LPUART1;

        unsafe {
            // Enable the LPUART1 clock
            bindings::HAL_RCC_LPUART1_CLK_ENABLE();

            // Enable the GPIO clock
            bindings::HAL_RCC_GPIOG_CLK_ENABLE();

            // Enable the VDDIO2 supply for LPUART1
            bindings::HAL_PWREx_EnableVddIO2();

            // Initialize the GPIO pins
            bindings::HAL_GPIO_Init(GPIOG, &raw mut gpio_init);
        }
    }
}

pub fn lpuart1_write(data: &[u8]) -> Result<(), ()> {
    crate::disable_interrupts();

    let ret = unsafe {
        bindings::HAL_UART_Transmit(
            &raw mut HLPUART1,
            data.as_ptr() as *mut u8,
            data.len() as u16,
            1000,
        )
    };

    if ret != HAL_OK {
        return Err(());
    }

    crate::enable_interrupts();

    Ok(())
}
