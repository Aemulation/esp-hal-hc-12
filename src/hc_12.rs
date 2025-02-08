use core::fmt::Write;
use embassy_embedded_hal::SetConfig;
use embassy_time::Timer;
use esp_hal::{
    gpio::OutputOpenDrain,
    peripheral::Peripheral,
    uart::{Config, Uart},
    Async, Blocking, DriverMode,
};
use heapless::{String, Vec};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Hc12Error {
    Test,
    BaudRate,
    AutoBaudRate,
    TransmissionMode,
    Default,
    Config,
    UartError(esp_hal::uart::Error),
    InvalidResponse,
}

impl From<esp_hal::uart::Error> for Hc12Error {
    fn from(error: esp_hal::uart::Error) -> Self {
        Hc12Error::UartError(error)
    }
}

pub enum TransmissionMode {
    Fu1,
    Fu2,
    Fu3,
    Fu4,
}

impl From<&TransmissionMode> for u32 {
    fn from(transmission_mode: &TransmissionMode) -> Self {
        match transmission_mode {
            TransmissionMode::Fu1 => 1,
            TransmissionMode::Fu2 => 2,
            TransmissionMode::Fu3 => 3,
            TransmissionMode::Fu4 => 4,
        }
    }
}

impl From<TransmissionMode> for u32 {
    fn from(transmission_mode: TransmissionMode) -> Self {
        match transmission_mode {
            TransmissionMode::Fu1 => 1,
            TransmissionMode::Fu2 => 2,
            TransmissionMode::Fu3 => 3,
            TransmissionMode::Fu4 => 4,
        }
    }
}

#[derive(Default, Clone, Copy)]
pub enum BaudRate {
    Baud1200,
    Baud2400,
    Baud4800,
    #[default]
    Baud9600,
    Baud19200,
    Baud38400,
    Baud57600,
    Baud115200,
}

impl From<&BaudRate> for &str {
    fn from(value: &BaudRate) -> Self {
        match value {
            BaudRate::Baud1200 => "1200",
            BaudRate::Baud2400 => "2400",
            BaudRate::Baud4800 => "4800",
            BaudRate::Baud9600 => "9600",
            BaudRate::Baud19200 => "19200",
            BaudRate::Baud38400 => "38400",
            BaudRate::Baud57600 => "57600",
            BaudRate::Baud115200 => "115200",
        }
    }
}

impl From<BaudRate> for u32 {
    fn from(baud_rate: BaudRate) -> Self {
        match baud_rate {
            BaudRate::Baud1200 => 1200,
            BaudRate::Baud2400 => 2400,
            BaudRate::Baud4800 => 4800,
            BaudRate::Baud9600 => 9600,
            BaudRate::Baud19200 => 19200,
            BaudRate::Baud38400 => 38400,
            BaudRate::Baud57600 => 57600,
            BaudRate::Baud115200 => 115200,
        }
    }
}

impl From<&BaudRate> for u32 {
    fn from(baud_rate: &BaudRate) -> Self {
        match baud_rate {
            BaudRate::Baud1200 => 1200,
            BaudRate::Baud2400 => 2400,
            BaudRate::Baud4800 => 4800,
            BaudRate::Baud9600 => 9600,
            BaudRate::Baud19200 => 19200,
            BaudRate::Baud38400 => 38400,
            BaudRate::Baud57600 => 57600,
            BaudRate::Baud115200 => 115200,
        }
    }
}

pub struct Hc12<'d, Dm: esp_hal::DriverMode> {
    uart: Uart<'d, Dm>,
    set: OutputOpenDrain<'d>,
}

impl<'d, Dm: DriverMode> Hc12<'d, Dm> {
    pub fn read_buffered(&mut self, buffer: &mut [u8]) -> Result<usize, esp_hal::uart::Error> {
        self.uart.read_buffered_bytes(buffer)
    }
}

impl<'d> Hc12<'d, Blocking> {
    pub fn new(
        uart: impl Peripheral<P = esp_hal::uart::AnyUart> + 'd,
        rx: impl Peripheral<P = esp_hal::gpio::AnyPin> + 'd,
        tx: impl Peripheral<P = esp_hal::gpio::AnyPin> + 'd,
        set: impl Peripheral<P = esp_hal::gpio::AnyPin> + 'd,
    ) -> Result<Self, Hc12Error> {
        let uart = Uart::new(uart, Default::default())
            .map_err(|_| Hc12Error::Config)?
            .with_rx(rx)
            .with_tx(tx);

        let mut set =
            OutputOpenDrain::new(set, esp_hal::gpio::Level::Low, esp_hal::gpio::Pull::None);
        set.set_high();
        esp_hal::delay::Delay::new().delay_millis(200);
        set.set_low();
        esp_hal::delay::Delay::new().delay_millis(200);

        Ok(Self { uart, set })
    }
}

impl<'d> Hc12<'d, Async> {
    pub async fn new(
        uart: impl Peripheral<P = esp_hal::uart::AnyUart> + 'd,
        rx: impl Peripheral<P = esp_hal::gpio::AnyPin> + 'd,
        tx: impl Peripheral<P = esp_hal::gpio::AnyPin> + 'd,
        set: impl Peripheral<P = esp_hal::gpio::AnyPin> + 'd,
    ) -> Result<Self, Hc12Error> {
        let uart = Uart::new(uart, Config::default())
            .map_err(|_| Hc12Error::Config)?
            .with_rx(rx)
            .with_tx(tx)
            .into_async();

        let mut set =
            OutputOpenDrain::new(set, esp_hal::gpio::Level::Low, esp_hal::gpio::Pull::None);
        set.set_high();
        Timer::after_millis(200).await;

        Ok(Self { uart, set })
    }
}

impl Hc12<'_, Blocking> {
    fn send_command<const N: usize>(
        &mut self,
        command: &String<N>,
    ) -> Result<String<14>, Hc12Error> {
        let mut buffer = [0u8; 14];
        while self
            .uart
            .read_buffered_bytes(&mut buffer)
            .is_ok_and(|bytes_read: usize| bytes_read != 0)
        {}

        self.set.set_low();
        esp_hal::delay::Delay::new().delay_millis(200);

        self.uart.write_bytes(command.as_bytes())?;
        esp_hal::delay::Delay::new().delay_millis(200);

        let bytes_read = self.uart.read_buffered_bytes(&mut buffer)?;
        self.set.set_high();
        esp_hal::delay::Delay::new().delay_millis(200);

        String::from_utf8(Vec::from_slice(&buffer[0..bytes_read]).unwrap())
            .map_err(|_| Hc12Error::InvalidResponse)
    }

    pub fn test(&mut self) -> Result<(), Hc12Error> {
        let mut command = String::<14>::new();
        command.push_str("AT").unwrap();
        let result = self.send_command(&command)?;

        if result != "OK\r\n" {
            return Err(Hc12Error::Test);
        }

        Ok(())
    }

    pub fn auto_baud(&mut self) -> Result<BaudRate, Hc12Error> {
        for baud_rate in [
            BaudRate::Baud1200,
            BaudRate::Baud2400,
            BaudRate::Baud4800,
            BaudRate::Baud9600,
            BaudRate::Baud19200,
            BaudRate::Baud38400,
            BaudRate::Baud57600,
            BaudRate::Baud115200,
        ] {
            self.uart
                .set_config(&Config::default().with_baudrate(u32::from(baud_rate)))
                .unwrap();
            esp_hal::delay::Delay::new().delay_millis(40);

            if self.test().is_ok() {
                return Ok(baud_rate);
            }
        }

        Err(Hc12Error::AutoBaudRate)
    }

    pub fn set_baud(&mut self, baud_rate: &BaudRate) -> Result<(), Hc12Error> {
        let mut command = String::<14>::new();
        write!(command, "AT+B{}", u32::from(baud_rate)).unwrap();

        let result = self.send_command(&command)?;
        self.uart
            .set_config(&Config::default().with_baudrate(u32::from(baud_rate)))
            .map_err(|_| Hc12Error::TransmissionMode)?;

        let mut expected_response = String::<14>::new();
        write!(expected_response, "OK+B{}\r\n", u32::from(baud_rate)).unwrap();

        if result != expected_response {
            return Err(Hc12Error::BaudRate);
        }

        Ok(())
    }

    pub fn set_transmission_mode(
        &mut self,
        transmission_mode: &TransmissionMode,
    ) -> Result<(), Hc12Error> {
        let mut command = String::<14>::new();
        write!(command, "AT+FU{}", u32::from(transmission_mode)).unwrap();

        let result = self.send_command(&command)?;

        let mut expected_response = String::<14>::new();
        write!(expected_response, "OK+FU{}", u32::from(transmission_mode)).unwrap();

        let mut splitted = result.split(",");
        if splitted
            .next()
            .is_none_or(|result| result != expected_response)
        {
            return Err(Hc12Error::TransmissionMode);
        }

        if let Some(new_baud_rate) = splitted.next() {
            let new_baud_rate = new_baud_rate[1..].trim();
            self.uart
                .set_config(&Config::default().with_baudrate(str::parse(new_baud_rate).unwrap()))
                .map_err(|_| Hc12Error::TransmissionMode)?;
        }

        Ok(())
    }

    pub fn set_default(&mut self) -> Result<(), Hc12Error> {
        let mut command = String::<14>::new();
        write!(command, "AT+DEFAULT").unwrap();

        let result = self
            .send_command(&command)
            .map_err(|_| Hc12Error::Default)?;

        if result != ("OK+DEFAULT\r\n") {
            return Err(Hc12Error::Default);
        }

        Ok(())
    }

    pub async fn write(&mut self, data: &[u8]) -> Result<usize, esp_hal::uart::Error> {
        self.uart.write_bytes(data)
    }

    pub async fn read(&mut self, buffer: &mut [u8]) -> Result<(), esp_hal::uart::Error> {
        self.uart.read_bytes(buffer)
    }
}

impl Hc12<'_, Async> {
    async fn send_command<const N: usize>(
        &mut self,
        command: &String<N>,
    ) -> Result<String<14>, Hc12Error> {
        let mut buffer = [0u8; 14];
        while self
            .uart
            .read_buffered_bytes(&mut buffer)
            .is_ok_and(|bytes_read: usize| bytes_read != 0)
        {}

        self.set.set_low();
        Timer::after_millis(200).await;

        self.uart.write_bytes(command.as_bytes())?;
        self.uart.flush_async().await?;
        Timer::after_millis(80).await;

        let bytes_read = self.uart.read_buffered_bytes(&mut buffer)?;
        self.set.set_high();
        Timer::after_millis(200).await;

        String::from_utf8(Vec::from_slice(&buffer[0..bytes_read]).unwrap())
            .map_err(|_| Hc12Error::InvalidResponse)
    }

    pub async fn test(&mut self) -> Result<(), Hc12Error> {
        let mut command = String::<14>::new();
        command.push_str("AT").unwrap();
        let result = self.send_command(&command).await?;

        if result != "OK\r\n" {
            return Err(Hc12Error::Test);
        }

        Ok(())
    }

    pub async fn auto_baud(&mut self) -> Result<BaudRate, Hc12Error> {
        for baud_rate in [
            BaudRate::Baud1200,
            BaudRate::Baud2400,
            BaudRate::Baud4800,
            BaudRate::Baud9600,
            BaudRate::Baud19200,
            BaudRate::Baud38400,
            BaudRate::Baud57600,
            BaudRate::Baud115200,
        ] {
            self.uart
                .set_config(&Config::default().with_baudrate(u32::from(baud_rate)))
                .unwrap();
            Timer::after_millis(40).await;

            if self.test().await.is_ok() {
                return Ok(baud_rate);
            }
        }

        Err(Hc12Error::AutoBaudRate)
    }

    pub async fn set_baud(&mut self, baud_rate: &BaudRate) -> Result<(), Hc12Error> {
        let mut command = String::<14>::new();
        write!(command, "AT+B{}", u32::from(baud_rate)).unwrap();

        let result = self.send_command(&command).await?;
        self.uart
            .set_config(&Config::default().with_baudrate(u32::from(baud_rate)))
            .map_err(|_| Hc12Error::TransmissionMode)?;

        let mut expected_response = String::<14>::new();
        write!(expected_response, "OK+B{}\r\n", u32::from(baud_rate)).unwrap();

        if result != expected_response {
            return Err(Hc12Error::BaudRate);
        }

        Ok(())
    }

    pub async fn set_transmission_mode(
        &mut self,
        transmission_mode: &TransmissionMode,
    ) -> Result<(), Hc12Error> {
        let mut command = String::<14>::new();
        write!(command, "AT+FU{}", u32::from(transmission_mode)).unwrap();

        let result = self.send_command(&command).await?;

        let mut expected_response = String::<14>::new();
        write!(
            expected_response,
            "OK+FU{}\r\n",
            u32::from(transmission_mode)
        )
        .unwrap();

        let mut splitted = result.split(",");
        if splitted
            .next()
            .is_none_or(|result| result != expected_response)
        {
            return Err(Hc12Error::TransmissionMode);
        }

        if let Some(new_baud_rate) = splitted.next() {
            let new_baud_rate = new_baud_rate[1..].trim();
            self.uart
                .set_config(&Config::default().with_baudrate(str::parse(new_baud_rate).unwrap()))
                .map_err(|_| Hc12Error::TransmissionMode)?;
        }

        Ok(())
    }

    pub async fn set_default(&mut self) -> Result<(), Hc12Error> {
        let mut command = String::<14>::new();
        write!(command, "AT+DEFAULT").unwrap();

        let result = self
            .send_command(&command)
            .await
            .map_err(|_| Hc12Error::Default)?;

        if result != ("OK+DEFAULT\r\n") {
            return Err(Hc12Error::Default);
        }

        Ok(())
    }

    pub async fn write_async(&mut self, data: &[u8]) -> Result<usize, esp_hal::uart::Error> {
        self.uart.write_async(data).await
    }

    pub async fn flush_async(&mut self) -> Result<(), esp_hal::uart::Error> {
        self.uart.flush_async().await
    }

    pub async fn read_async(&mut self, buffer: &mut [u8]) -> Result<usize, esp_hal::uart::Error> {
        self.uart.read_async(buffer).await
    }
}
