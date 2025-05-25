use crate::bindings;

pub fn init_rcc() {
    let clock_config = bindings::RCC_ClkInitTypeDef {
        ClockType: bindings::RCC_CLOCKTYPE_SYSCLK,
        SYSCLKSource: bindings::RCC_SYSCLKSOURCE_MSI,
        AHBCLKDivider: bindings::RCC_SYSCLK_DIV1,
        APB1CLKDivider: bindings::RCC_HCLK_DIV1,
        APB2CLKDivider: bindings::RCC_HCLK_DIV1,
    };

    unsafe { bindings::HAL_RCC_ClockConfig(&clock_config, bindings::FLASH_LATENCY_2) };
}
