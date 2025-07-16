use embedded_graphics::text::renderer::CharacterStyle;
use num_format::{Locale, ToFormattedString};
use tinybmp::Bmp;

use super::*;

// reg_data_out <= { 21'b0, lna144, a144, pa144, lna430, a430, pa430, slv_reg0[2:0], hard_fault, ptt};
const MASK_PTT: u32 = 0b1;
const MASK_HARD_FAULT: u32 = 0b10;
const MASK_PA430: u32 = 0b100000;
const MASK_A430: u32 = 0b1000000;
const MASK_LNA430: u32 = 0b10000000;
const MASK_PA144: u32 = 0b100000000;
const MASK_A144: u32 = 0b1000000000;
const MASK_LNA144: u32 = 0b10000000000;

#[derive(Debug, Clone, Copy, PartialEq)]
enum BannerState {
    None,
    FAULT,
    PTT,
    WARNING,
    RX,
}

pub struct LcdView<'a> {
    style_text_white_bg: MonoTextStyle<'a, Rgb565>,
    style_text_green_bg: MonoTextStyle<'a, Rgb565>,
    led_off: Bmp<'a, Rgb565>,
    led_on: Bmp<'a, Rgb565>,
    txf: u64,
    rxf: u64,
    data: u32,
    style_red: PrimitiveStyle<Rgb565>,
    style_green: PrimitiveStyle<Rgb565>,
    style_yellow: PrimitiveStyle<Rgb565>,
    style_text_black: MonoTextStyle<'a, Rgb565>,
    banner: BannerState,
    pwr: u16,
    swr: u16,
}

impl<'a> LcdView<'a> {
    pub fn new() -> Self {
        let bmp_data1 = include_bytes!("led_on.bmp");
        let bmp1 = Bmp::<Rgb565>::from_slice(bmp_data1).unwrap();
        let bmp_data2 = include_bytes!("led_off.bmp");
        let bmp2 = Bmp::<Rgb565>::from_slice(bmp_data2).unwrap();
        let mut s1 = MonoTextStyle::new(&FONT_10X20, Rgb565::GREEN);
        s1.set_background_color(Some(Rgb565::BLACK));
        let mut s2 = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);
        s2.set_background_color(Some(Rgb565::BLACK));
        let s_red = PrimitiveStyleBuilder::new()
            .stroke_color(Rgb565::RED)
            .stroke_width(1)
            .fill_color(Rgb565::RED)
            .build();
        let s_green = PrimitiveStyleBuilder::new()
            .stroke_color(Rgb565::GREEN)
            .stroke_width(1)
            .fill_color(Rgb565::GREEN)
            .build();
        let s_yellow = PrimitiveStyleBuilder::new()
            .stroke_color(Rgb565::YELLOW)
            .stroke_width(1)
            .fill_color(Rgb565::YELLOW)
            .build();
        LcdView {
            style_text_white_bg: s2,
            style_text_green_bg: s1,
            led_off: bmp2,
            led_on: bmp1,
            txf: 1,
            rxf: 1,
            data: 0,
            style_red: s_red,
            style_green: s_green,
            style_yellow: s_yellow,
            style_text_black: MonoTextStyle::new(&FONT_10X20, Rgb565::BLACK),
            banner: BannerState::None,
            pwr: 0,
            swr: 0,
        }
    }

    pub fn loading<O1: OutputPin, O2: OutputPin>(
        &mut self,
        lcd: &mut Ili9341<SPIInterface<SpidevDevice, O1>, O2>,
    ) {
        lcd.clear(Rgb565::BLACK).unwrap();

        Text::with_alignment(
            "R2AJP SAT STATION",
            Point::new(160, 30),
            self.style_text_white_bg,
            Alignment::Center,
        )
        .draw(lcd)
        .unwrap();

        Text::new("TX LO:", Point::new(20, 70), self.style_text_white_bg)
            .draw(lcd)
            .unwrap();
        Text::new("RX LO:", Point::new(20, 90), self.style_text_white_bg)
            .draw(lcd)
            .unwrap();
        Text::new("2 m", Point::new(20, 130), self.style_text_white_bg)
            .draw(lcd)
            .unwrap();
        Text::new("70 cm", Point::new(170, 130), self.style_text_white_bg)
            .draw(lcd)
            .unwrap();
        Text::new(
            "LNA A   PA     LNA A   PA",
            Point::new(20, 150),
            self.style_text_white_bg,
        )
        .draw(lcd)
        .unwrap();
        Image::new(&self.led_off, Point::new(20, 160))
            .draw(lcd)
            .unwrap();
        Image::new(&self.led_off, Point::new(60, 160))
            .draw(lcd)
            .unwrap();
        Image::new(&self.led_off, Point::new(100, 160))
            .draw(lcd)
            .unwrap();
        Image::new(&self.led_off, Point::new(170, 160))
            .draw(lcd)
            .unwrap();
        Image::new(&self.led_off, Point::new(210, 160))
            .draw(lcd)
            .unwrap();
        Image::new(&self.led_off, Point::new(250, 160))
            .draw(lcd)
            .unwrap();
        Text::new("PWR:", Point::new(20, 210), self.style_text_white_bg)
            .draw(lcd)
            .unwrap();
        Text::new("SWR:", Point::new(170, 210), self.style_text_white_bg)
            .draw(lcd)
            .unwrap();
    }

    pub fn render<O1: OutputPin, O2: OutputPin>(
        &mut self,
        lcd: &mut Ili9341<SPIInterface<SpidevDevice, O1>, O2>,
    ) {
        let txf = TX_FREQ.load(SeqCst);
        let rxf = RX_FREQ.load(SeqCst);
        let bd = WRITE_SEQ_STATE.load(SeqCst);
        let data = READ_SEQ_STATE.load(SeqCst);
        let pwr = TX_PWR.load(SeqCst);
        let swr = TX_SWR.load(SeqCst);

        if self.pwr != pwr {
            let pwr_str = format!("{:03}%", pwr);
            self.pwr = pwr;
            Text::new(&pwr_str, Point::new(70, 210), self.style_text_green_bg)
                .draw(lcd)
                .unwrap();
        }
        if self.swr != swr {
            let swr2f = swr as f32 / 100.0;
            let swr_str = format!("{:.02}  ", swr2f);
            self.swr = swr;
            Text::new(&swr_str, Point::new(220, 210), self.style_text_green_bg)
                .draw(lcd)
                .unwrap();
        }
        //let state_str = format!("Current state: {0:x}", data);
        if self.txf != txf {
            let txf_str = format!("{}", txf.to_formatted_string(&num_format::Locale::en));
            self.txf = txf;
            Text::new(&txf_str, Point::new(90, 70), self.style_text_green_bg)
                .draw(lcd)
                .unwrap();
        }
        if self.rxf != rxf {
            let rxf_str = format!("{}", rxf.to_formatted_string(&num_format::Locale::en));
            self.rxf = rxf;
            Text::new(&rxf_str, Point::new(90, 90), self.style_text_green_bg)
                .draw(lcd)
                .unwrap();
        }
        if (data & MASK_PA430) != (self.data & MASK_PA430) {
            if data & MASK_PA430 > 0 {
                Image::new(&self.led_on, Point::new(250, 160))
                    .draw(lcd)
                    .unwrap();
            } else {
                Image::new(&self.led_off, Point::new(250, 160))
                    .draw(lcd)
                    .unwrap();
            }
        }
        if (data & MASK_A430) != (self.data & MASK_A430) {
            if data & MASK_A430 > 0 {
                Image::new(&self.led_on, Point::new(210, 160))
                    .draw(lcd)
                    .unwrap();
            } else {
                Image::new(&self.led_off, Point::new(210, 160))
                    .draw(lcd)
                    .unwrap();
            }
        }
        if (data & MASK_LNA430) != (self.data & MASK_LNA430) {
            if data & MASK_LNA430 > 0 {
                Image::new(&self.led_on, Point::new(170, 160))
                    .draw(lcd)
                    .unwrap();
            } else {
                Image::new(&self.led_off, Point::new(170, 160))
                    .draw(lcd)
                    .unwrap();
            }
        }
        if (data & MASK_PA144) != (self.data & MASK_PA144) {
            if data & MASK_PA144 > 0 {
                Image::new(&self.led_on, Point::new(100, 160))
                    .draw(lcd)
                    .unwrap();
            } else {
                Image::new(&self.led_off, Point::new(100, 160))
                    .draw(lcd)
                    .unwrap();
            }
        }
        if (data & MASK_A144) != (self.data & MASK_A144) {
            if data & MASK_A144 > 0 {
                Image::new(&self.led_on, Point::new(60, 160))
                    .draw(lcd)
                    .unwrap();
            } else {
                Image::new(&self.led_off, Point::new(60, 160))
                    .draw(lcd)
                    .unwrap();
            }
        }
        if (data & MASK_LNA144) != (self.data & MASK_LNA144) {
            if data & MASK_LNA144 > 0 {
                Image::new(&self.led_on, Point::new(20, 160))
                    .draw(lcd)
                    .unwrap();
            } else {
                Image::new(&self.led_off, Point::new(20, 160))
                    .draw(lcd)
                    .unwrap();
            }
        }
        if data & MASK_HARD_FAULT > 0 {
            if self.banner != BannerState::FAULT {
                Rectangle::new(Point::new(30, 35), Size::new(260, 20))
                    .into_styled(self.style_red)
                    .draw(lcd)
                    .unwrap();
                Text::with_alignment(
                    "HARD FAULT",
                    Point::new(160, 50),
                    self.style_text_black,
                    Alignment::Center,
                )
                .draw(lcd)
                .unwrap();
                self.banner = BannerState::FAULT;
            }
        } else if bd == 0 {
            if self.banner != BannerState::WARNING {
                Rectangle::new(Point::new(30, 35), Size::new(260, 20))
                    .into_styled(self.style_yellow)
                    .draw(lcd)
                    .unwrap();
                Text::with_alignment(
                    "WRONG BAND SELECTED",
                    Point::new(160, 50),
                    self.style_text_black,
                    Alignment::Center,
                )
                .draw(lcd)
                .unwrap();
                self.banner = BannerState::WARNING;
            }
        } else if data & MASK_PTT == 0 {
            if self.banner != BannerState::PTT {
                Rectangle::new(Point::new(30, 35), Size::new(260, 20))
                    .into_styled(self.style_red)
                    .draw(lcd)
                    .unwrap();
                Text::with_alignment(
                    "TRANSMIT - ON AIR",
                    Point::new(160, 50),
                    self.style_text_black,
                    Alignment::Center,
                )
                .draw(lcd)
                .unwrap();
                self.banner = BannerState::PTT;
            }
        } else if data & MASK_PTT > 0 {
            if self.banner != BannerState::RX {
                Rectangle::new(Point::new(30, 35), Size::new(260, 20))
                    .into_styled(self.style_green)
                    .draw(lcd)
                    .unwrap();
                Text::with_alignment(
                    "RECEIVE",
                    Point::new(160, 50),
                    self.style_text_black,
                    Alignment::Center,
                )
                .draw(lcd)
                .unwrap();
                self.banner = BannerState::RX;
            }
        }
        self.data = data;
    }
}
