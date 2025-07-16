mod bandcode;
mod renderer;

use clap::{arg, value_parser, ArgAction, Command};
use std::sync::atomic::{AtomicU16, AtomicU32, AtomicU64, Ordering::SeqCst};
use std::sync::{Condvar, WaitTimeoutResult};
use std::time::Duration;
use std::{
    any::TypeId,
    process,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread, time,
};

use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{ErrorType, OutputPin, PinState, StatefulOutputPin};
//use linux_embedded_hal::{ Delay };
use dac_lib_mcp4725::*;
use ddevmem::{register_map, DevMem};
use display_interface_spi::SPIInterface;
use embedded_ads111x::*;
use embedded_graphics::{
    image::Image,
    mono_font::{ascii::FONT_10X20, ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, PrimitiveStyleBuilder, Rectangle},
    text::{Alignment, Text},
};
use embedded_hal_bus::i2c::AtomicDevice;
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
use embedded_hal_bus::util::AtomicCell;
use ili9341::{DisplaySize240x320, Ili9341, Orientation};
use linux_embedded_hal::spidev::{SpiModeFlags, Spidev, SpidevOptions, SpidevTransfer};
use linux_embedded_hal::{I2cdev, SpidevDevice};

use serialport::{available_ports, SerialPortType};
use std::io::{self, BufRead, BufReader, Write};

use crate::bandcode::*;
use crate::renderer::*;

// mapping of sequencer control registers
const SEQ_ADDR: usize = 0x43C0_0000; // address from Vivado!
register_map! {
    pub unsafe map MyRegisterMap {
        0x00 => rw reg0: u32,
        0x04 => ro reg1: u32
    }
}

// переменные состояния для обмена между потоками данными
static READ_SEQ_STATE: AtomicU32 = AtomicU32::new(0);
static WRITE_SEQ_STATE: AtomicU32 = AtomicU32::new(0);
static RX_FREQ: AtomicU64 = AtomicU64::new(0);
static TX_FREQ: AtomicU64 = AtomicU64::new(0);
static TX_PWR: AtomicU16 = AtomicU16::new(0);
static TX_SWR: AtomicU16 = AtomicU16::new(0);

fn main() {
    let args = Command::new("libremon")
        .version(clap::crate_version!())
        .about("LibreSDR state monitor. (C) R2AJP")
        .disable_help_flag(true)
        .disable_version_flag(true)
        .args(&[
            arg!(-t --iotest "IO test").action(ArgAction::SetTrue),
            arg!(-'v' --version "Print version information").action(ArgAction::Version),
            arg!(-'?' --help "Print help information")
                .global(true)
                .action(ArgAction::Help),
        ])
        .get_matches();

    // true - do some tests?
    let some_test: bool = *args.get_one("iotest").unwrap();
    //print!("iotest: {some_test}");
    /*
    ).unwrap_or_else(|_err| {
        println!("Couldn't open IIO context.");
        process::exit(1);
    });
    */
    let devmem = unsafe { DevMem::new(SEQ_ADDR, None).unwrap() };
    let mut reg_map = unsafe { MyRegisterMap::new(std::sync::Arc::new(devmem)).unwrap() };
    //let (reg0_address, reg0_offset) = (reg_map.reg0_address(), reg_map.reg0_offset());
    //    let reg1_value = *reg_map.reg1();
    //    *reg_map.reg0_mut() = 4;

    // Настройка UART
    let mut uart_port = if let Ok(port) = serialport::new("/dev/ttyUL0", 9600)
        .timeout(Duration::from_millis(3000))
        .open()
    {
        port
    } else {
        eprintln!("Failed to open UART.");
        ::std::process::exit(1);
    };

    // ---- Handle ^C since we want a graceful shutdown -----
    let quit = Arc::new(AtomicBool::new(false));
    let q = quit.clone();

    ctrlc::set_handler(move || {
        q.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    // настройки дисплея
    let mut lcd = {
        // настройки пинов для дисплея
        let mut delay = linux_embedded_hal::Delay;
        //delay.delay_ms(100u32);

        let mut spi = SpidevDevice::open("/dev/spidev0.0").unwrap_or_else(|_err| {
            println!("Couldn't open spi /dev/spidev0.0");
            process::exit(1);
        });

        let options = SpidevOptions::new()
            .bits_per_word(8)
            .max_speed_hz(20_000_000)
            .mode(SpiModeFlags::SPI_MODE_0)
            .build();

        spi.configure(&options).unwrap_or_else(|_err| {
            println!("Couldn't configure spi.");
            process::exit(1);
        });

        /*
        // cs - fake 15 led - по факту пином CS управляет линукс
        let mut led15 = gpiocdev_embedded_hal::OutputPin
            ::new("/dev/gpiochip0", 15, PinState::Low)
            .unwrap_or_else(|_err| {
                println!("Couldn't open pin15.");
                process::exit(1);
            });
        */

        // dc
        let pin_dc = gpiocdev_embedded_hal::OutputPin::new("/dev/gpiochip0", 55, PinState::Low)
            .unwrap_or_else(|_err| {
                println!("Couldn't open pin55.");
                process::exit(1);
            });
        // reset
        let pin_reset = gpiocdev_embedded_hal::OutputPin::new("/dev/gpiochip0", 54, PinState::Low)
            .unwrap_or_else(|_err| {
                println!("Couldn't open pin54.");
                process::exit(1);
            });
        let spi_iface = SPIInterface::new(spi, pin_dc);

        Ili9341::new(
            spi_iface,
            pin_reset,
            &mut delay,
            Orientation::Landscape,
            DisplaySize240x320,
        )
        .unwrap()
    };

    let mut my_view = LcdView::new();
    my_view.loading(&mut lcd);

    // шина для АЦП и ЦАП
    let i2cdev = I2cdev::new("/dev/i2c-0").unwrap_or_else(|_err| {
        println!("Couldn't open i2c");
        process::exit(1);
    });
    let i2c_cell = AtomicCell::new(i2cdev);

    {
        // Configure the MCP4725 DAC
        let mut dac = MCP4725::new(AtomicDevice::new(&i2c_cell), DEFAULT_ADDR);
        let dac_data = if let Ok(retval) = dac.read_dac() {
            //println!("MCP4725 data register is {:?}, eeprom is", retval.register, retval.eeprom);
            println!("MCP4725 read");
            retval
        } else {
            println!("Couldn't read MCP4725");
            process::exit(1);
        };
        /*
            // write 0 to eeprom/ power up after this will be 0 Volt out!!!
            if let Ok(_) = dac.write_dac_eeprom(0) {
                println!("MCP4725 eerprom set to 0");
            } else {
                println!("Couldn't set MCP4725");
                process::exit(1);
            };
        */
        // включаем питание платы коммутации
        if let Ok(_) = dac.write_dac(0x0FFF) {
            println!("MCP4725 set to VCC");
        } else {
            println!("Couldn't set MCP4725");
            process::exit(1);
        };
    }

    // поток печати при условии события печати
    let signal2redraw = Arc::new((Mutex::new(false), Condvar::new()));
    let signal2redraw2 = signal2redraw.clone();
    let signal2redraw3 = signal2redraw.clone();
    let signal2redraw4 = signal2redraw.clone();
    let signal2redraw5 = signal2redraw.clone();

    // создаем поток печати на экран
    thread::spawn(move || {
        let (lock, cvar) = &*signal2redraw2;
        loop {
            // ждем, что кто-то выставит redraw true
            {
                let mut redraw = lock.lock().unwrap();
                while !*redraw {
                    let result = cvar.wait_timeout(redraw, Duration::from_secs(3)).unwrap();
                    redraw = result.0;
                    if result.1.timed_out() {
                        println!("Timeout redraw");
                        break;
                    }
                }
                *redraw = false;
            }
            // делаем что-то с уже освобожденным локом
            my_view.render(&mut lcd);
        }
    });
    
        // создаем поток чтения из UART
        thread::spawn(move || {
            let mut old_txf = 0u64;
            let mut old_rxf = 0u64;
            let reader = BufReader::new(uart_port);
            let (lock, cvar) = &*signal2redraw3;
            for line_tmp in reader.lines() {
                match line_tmp {
                    Ok(line) => {
                        let mut txf: u64 = 0;
                        let mut rxf: u64 = 0;
                        let parts = line.split("R");
                        for part in parts {
                            if part.chars().nth(0) == Some('T') {
                                //println!("TX {}", &part[1..]);
                                txf = if let Ok(converted) = part[1..].parse::<u64>() {
                                    converted
                                } else {
                                    0
                                };
                                //println!("TXI {}", txf);
                            } else {
                                rxf = if let Ok(converted) = part.parse::<u64>() {
                                    converted
                                } else {
                                    0
                                };
                                //println!("RXI {}", rxf);
                                //println!("RX {}", &part);
                            }
                        }
                        //println!("Last read RX: {} TX: {}", rxf, txf);

                        let new_band_data = code_band(rxf, txf);

                        // если частоты не изменились, уходим на новый цикл чтения из UART
                        if txf == old_txf && rxf == old_rxf {
                            continue;
                        }
                        old_txf = txf;
                        old_rxf = rxf;
                        RX_FREQ.store(rxf, SeqCst);
                        TX_FREQ.store(txf, SeqCst);

                        // прописываем и обновляем экран
                        WRITE_SEQ_STATE.store(new_band_data, SeqCst);
                        // если есть новые данные band_data из порта - сигналим перерисовать экран
                        {
                            let mut redraw = lock.lock().unwrap();
                            *redraw = true;
                            cvar.notify_one();
                        } // освобождаем лок
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                        eprintln!("UART Timeout");
                        //::std::process::exit(1);
                    }
                    Err(e) => {
                        eprintln!("Failed to read UART line. Error: {}", e);
                        //::std::process::exit(1);
                    }
                }
            }
        });
    
    // создаем поток обмена с DEVMEM
    thread::spawn(move || {
        let mut old_band_data = 0u32;
        let mut old_state_data = 0u32;
        let (lock, cvar) = &*signal2redraw4;
        loop {
            // синхронизируем данные с портами каждые 10 мс
            thread::sleep(Duration::from_millis(10));

            // пишем в секвенсор по необходимости данные о новом диапазоне
            let new_band_data = WRITE_SEQ_STATE.load(SeqCst);
            if old_band_data != new_band_data {
                // write new band data to seq reg0
                old_band_data = new_band_data;
                *reg_map.reg0_mut() = new_band_data;
            }
            // read seq state data from reg1
            let new_state_data = *reg_map.reg1();

            // если состояние секвенсера не изменилось
            if old_state_data == new_state_data {
                continue;
            }
            // если состояние изменилось, обновляем экран
            old_state_data = new_state_data;
            READ_SEQ_STATE.store(new_state_data, SeqCst);
            // если есть новые данные состояния - сигналим перерисовать экран
            {
                let mut redraw = lock.lock().unwrap();
                *redraw = true;
                cvar.notify_one();
            } // освобождаем лок
        }
    });

    // создаем поток чтения ADC
    thread::spawn(move || {
        // настраиваем ADC
        let adc_config = ADS111xConfig::default()
            .mux(InputMultiplexer::AIN0GND)
            .dr(DataRate::SPS128)
            .pga(ProgramableGainAmplifier::V6_144);

        // Create a new ADS111x instance with the specified configuration
        // Note: This only creates the instance, it doesn't write the configuration to the chip
        let mut adc = if let Ok(x) = ADS111x::new(AtomicDevice::new(&i2c_cell), 0x48u8, adc_config)
        {
            x
        } else {
            println!("Couldn't find ADS1115");
            process::exit(1);
        };
        // Write the configuration to the chip's registers
        // This step is necessary to apply the configuration
        if let Err(_e) = adc.write_config(None) {
            println!("Couldn't write config ADS1115");
            process::exit(1);
        }

        let mut old_tx_pwr = 0u16;
        let mut old_tx_swr = 0u16;
        let (lock, cvar) = &*signal2redraw5;
        loop {
            // синхронизируем данные ADC каждые 1 мс
            thread::sleep(Duration::from_millis(1));

            // читаем данные о диапазоне передачи
            let band_data = WRITE_SEQ_STATE.load(SeqCst);
            let bd = TxRxMode::try_from(band_data).unwrap();
            let (adc_pwr_channel, adc_swr_channel) = match bd {
                TxRxMode::All430 | TxRxMode::Tx430Rx144 => {
                    (InputMultiplexer::AIN2GND, InputMultiplexer::AIN3GND)
                }
                _ => (InputMultiplexer::AIN0GND, InputMultiplexer::AIN1GND),
            };
            // читаем данные ADC
            let pwr_v = if let Ok(v) = adc.read_single_voltage(Some(adc_pwr_channel)) {
                v * 4.0
            } else {
                0.0
            };
            let swr_v = if let Ok(v) = adc.read_single_voltage(Some(adc_swr_channel)) {
                v * 4.0
            } else {
                0.0
            };
            let power_percent: u16 = unsafe { ((pwr_v / 6.144) * 100.0).to_int_unchecked::<u16>() };
            // тут некоторая дичь по расчёту КСВ. Выход КСВ*100 как целое
            let swr_p1 = pwr_v + swr_v;
            let swr_p2 = (pwr_v - swr_v).abs();
            let swr_u16 = if swr_p2 != 0.0 {
                let swr = (swr_p1 / swr_p2) * 100.0;
                if swr > 10000.0 {
                    10000
                } else if swr < 100.0 {
                    100
                } else {
                    unsafe { swr.to_int_unchecked::<u16>() }
                }
            } else {
                10000
            };

            // если значения не изменились
            if old_tx_pwr == power_percent && old_tx_swr == swr_u16 {
                continue;
            }
            // если состояние изменилось, обновляем экран
            old_tx_pwr = power_percent;
            old_tx_swr = swr_u16;

            TX_PWR.store(power_percent, SeqCst);
            TX_SWR.store(swr_u16, SeqCst);
            // если есть новые данные - сигналим перерисовать экран
            {
                let mut redraw = lock.lock().unwrap();
                *redraw = true;
                cvar.notify_one();
            } // освобождаем лок
        }
    });

    // red_led
    let mut red_led = gpiocdev_embedded_hal::OutputPin::new("/dev/gpiochip0", 37, PinState::Low)
        .unwrap_or_else(|_err| {
            println!("Couldn't open pin37.");
            process::exit(1);
        });

    while !quit.load(Ordering::SeqCst) {
        red_led.toggle().ok();
        thread::sleep(time::Duration::from_millis(200));
        /*
        match adc.read_single_voltage(Some(InputMultiplexer::AIN0GND)){
            Ok(v) => println!("Val 0 single {:.6}", v),
            Err(e) => println!("Error {:?}", e),
        }
        match adc.read_single_voltage(Some(InputMultiplexer::AIN1GND)){
            Ok(v) => println!("Val 1 single {:.6}", v),
            Err(e) => println!("Error {:?}", e),
        }
        match adc.read_single_voltage(Some(InputMultiplexer::AIN2GND)){
            Ok(v) => println!("Val 2 single {:.6}", v),
            Err(e) => println!("Error {:?}", e),
        }
        match adc.read_single_voltage(Some(InputMultiplexer::AIN3GND)){
            Ok(v) => println!("Val 3 single {:.6}", v),
            Err(e) => println!("Error {:?}", e),
        }*/

        // для теста
        /*
        let rxtest = 144_200_000u64;
        let txtest = 145_200_010u64;
        //      let txtest = 0u64;
        //    let rxtest = 0u64;
        let new_band_data = code_band(rxtest, txtest);
        RX_FREQ.store(rxtest, SeqCst);
        TX_FREQ.store(txtest, SeqCst);
        WRITE_SEQ_STATE.store(new_band_data, SeqCst);*/
    }

    println!("All was OK! Bye");
}
