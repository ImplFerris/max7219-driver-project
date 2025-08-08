use embedded_hal::spi::SpiDevice;

use crate::{
    MAX_DISPLAYS, NUM_DIGITS, Result,
    error::Error,
    registers::{DecodeMode, Register},
};

/// Driver for the MAX7219 LED display controller.
/// Communicates over SPI using the embedded-hal `SpiDevice` trait.
pub struct Max7219<SPI> {
    spi: SPI,
    buffer: [u8; MAX_DISPLAYS * 2],
    device_count: usize,
}

impl<SPI> Max7219<SPI>
where
    SPI: SpiDevice,
{
    pub fn new(spi: SPI) -> Self {
        Self {
            spi,
            device_count: 1, // Default to 1, use with_device_count to increase count
            buffer: [0; MAX_DISPLAYS * 2],
        }
    }
    pub fn device_count(&self) -> usize {
        self.device_count
    }

    pub fn with_device_count(mut self, count: usize) -> Result<Self> {
        if count > MAX_DISPLAYS {
            return Err(Error::InvalidDeviceCount);
        }
        self.device_count = count;
        Ok(self)
    }

    pub fn init(&mut self) -> Result<()> {
        self.power_on()?;

        self.test_all(false)?;
        self.set_scan_limit_all(NUM_DIGITS)?;
        self.set_decode_mode_all(DecodeMode::NoDecode)?;

        self.clear_all()?;

        Ok(())
    }

    pub(crate) fn write_device_register(
        &mut self,
        device_index: usize,
        register: Register,
        data: u8,
    ) -> Result<()> {
        if device_index >= self.device_count {
            return Err(Error::InvalidDeviceIndex);
        }

        self.buffer = [0; MAX_DISPLAYS * 2];

        let offset = device_index * 2; // 2 bytes(16 bits packet) per display
        self.buffer[offset] = register as u8;
        self.buffer[offset + 1] = data;

        self.spi.write(&self.buffer[0..self.device_count * 2])?;

        Ok(())
    }

    pub(crate) fn write_all_registers(&mut self, ops: &[(Register, u8)]) -> Result<()> {
        // clear the buffer: 2 bytes per device
        self.buffer = [0; MAX_DISPLAYS * 2];

        // fill in reverse order so that SPI shifts into the last device first
        for (i, &(reg, data)) in ops.iter().rev().enumerate() {
            let offset = i * 2;
            self.buffer[offset] = reg as u8;
            self.buffer[offset + 1] = data;
        }

        // send exactly device_count packets
        let len = self.device_count * 2;
        self.spi.write(&self.buffer[..len])?;

        Ok(())
    }

    pub fn power_on(&mut self) -> Result<()> {
        let ops = [(Register::Shutdown, 0x01); MAX_DISPLAYS];

        self.write_all_registers(&ops[..self.device_count])
    }
    pub fn power_off(&mut self) -> Result<()> {
        let ops = [(Register::Shutdown, 0x00); MAX_DISPLAYS];

        self.write_all_registers(&ops[..self.device_count])
    }

    pub fn power_on_device(&mut self, device_index: usize) -> Result<()> {
        self.write_device_register(device_index, Register::Shutdown, 0x01)
    }

    pub fn power_off_device(&mut self, device_index: usize) -> Result<()> {
        self.write_device_register(device_index, Register::Shutdown, 0x00)
    }
    pub fn test_device(&mut self, device_index: usize, enable: bool) -> Result<()> {
        let data = if enable { 0x01 } else { 0x00 };
        self.write_device_register(device_index, Register::DisplayTest, data)
    }

    pub fn test_all(&mut self, enable: bool) -> Result<()> {
        let data = if enable { 0x01 } else { 0x00 };
        let ops: [(Register, u8); MAX_DISPLAYS] = [(Register::DisplayTest, data); MAX_DISPLAYS];
        self.write_all_registers(&ops[..self.device_count])
    }

    pub fn clear_display(&mut self, device_index: usize) -> Result<()> {
        for digit_register in Register::digits() {
            self.write_device_register(device_index, digit_register, 0x00)?;
        }
        Ok(())
    }

    pub fn clear_all(&mut self) -> Result<()> {
        for digit_register in Register::digits() {
            let ops = [(digit_register, 0x00); MAX_DISPLAYS];
            self.write_all_registers(&ops[..self.device_count])?;
        }

        Ok(())
    }

    pub fn set_intensity(&mut self, device_index: usize, intensity: u8) -> Result<()> {
        if intensity > 0x0F {
            return Err(Error::InvalidIntensity);
        }
        self.write_device_register(device_index, Register::Intensity, intensity)
    }

    pub fn set_intensity_all(&mut self, intensity: u8) -> Result<()> {
        let ops = [(Register::Intensity, intensity); MAX_DISPLAYS];
        self.write_all_registers(&ops[..self.device_count])
    }

    pub fn set_device_scan_limit(&mut self, device_index: usize, limit: u8) -> Result<()> {
        if !(1..=8).contains(&limit) {
            return Err(Error::InvalidScanLimit);
        }

        self.write_device_register(device_index, Register::ScanLimit, limit - 1)
    }

    pub fn set_scan_limit_all(&mut self, limit: u8) -> Result<()> {
        if !(1..=8).contains(&limit) {
            return Err(Error::InvalidScanLimit);
        }
        let val = limit - 1;
        let ops: [(Register, u8); MAX_DISPLAYS] = [(Register::ScanLimit, val); MAX_DISPLAYS];
        self.write_all_registers(&ops[..self.device_count])
    }

    pub fn set_device_decode_mode(&mut self, device_index: usize, mode: DecodeMode) -> Result<()> {
        self.write_device_register(device_index, Register::DecodeMode, mode as u8)
    }

    pub fn set_decode_mode_all(&mut self, mode: DecodeMode) -> Result<()> {
        let byte = mode as u8;
        let ops: [(Register, u8); MAX_DISPLAYS] = [(Register::DecodeMode, byte); MAX_DISPLAYS];
        self.write_all_registers(&ops[..self.device_count])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MAX_DISPLAYS;
    use embedded_hal_mock::eh1::{spi::Mock as SpiMock, spi::Transaction};

    #[test]
    fn test_new() {
        let mut spi = SpiMock::new(&[]);
        let driver = Max7219::new(&mut spi);
        // Default device count => 1
        assert_eq!(driver.device_count(), 1);

        spi.done();
    }

    #[test]
    fn test_with_device_count_valid() {
        let mut spi = SpiMock::new(&[]);
        let driver = Max7219::new(&mut spi);
        let driver = driver
            .with_device_count(4)
            .expect("Should accept valid count");
        assert_eq!(driver.device_count(), 4);
        spi.done();
    }

    #[test]
    fn test_with_device_count_invalid() {
        let mut spi = SpiMock::new(&[]);
        let driver = Max7219::new(&mut spi);
        let result = driver.with_device_count(MAX_DISPLAYS + 1);
        assert!(matches!(result, Err(Error::InvalidDeviceCount)));

        spi.done();
    }

    #[test]
    fn test_write_device_register_valid_index() {
        let expected_transactions = [
            Transaction::transaction_start(),
            Transaction::write_vec(vec![
                Register::Shutdown.addr(),
                0x01,
                0x00, // no-op for second device in chain
                0x00,
            ]),
            Transaction::transaction_end(),
        ];
        let mut spi = SpiMock::new(&expected_transactions);
        let mut driver = Max7219::new(&mut spi)
            .with_device_count(2)
            .expect("Should accept valid count");

        driver
            .write_device_register(0, Register::Shutdown, 0x01)
            .expect("should write register");

        spi.done();
    }

    #[test]
    fn test_write_device_register_invalid_index() {
        let mut spi = SpiMock::new(&[]); // No SPI transactions expected
        let mut driver = Max7219::new(&mut spi)
            .with_device_count(2)
            .expect("Should accept valid count");

        let result = driver.write_device_register(2, Register::Shutdown, 0x01); // Index 2 is invalid for device_count=2
        assert_eq!(result, Err(Error::InvalidDeviceIndex));

        spi.done();
    }

    #[test]
    fn test_write_all_registers_valid() {
        let expected_transactions = [
            Transaction::transaction_start(),
            Transaction::write_vec(vec![
                Register::Intensity.addr(),
                0x01,
                Register::Intensity.addr(),
                0x01,
            ]),
            Transaction::transaction_end(),
        ];
        let mut spi = SpiMock::new(&expected_transactions);
        let mut driver = Max7219::new(&mut spi)
            .with_device_count(2)
            .expect("Should accept valid count");

        driver
            .write_all_registers(&[(Register::Intensity, 0x01), (Register::Intensity, 0x01)])
            .expect("should  write all registers");

        spi.done();
    }

    #[test]
    fn test_power_on() {
        let expected_transactions = [
            Transaction::transaction_start(),
            Transaction::write_vec(vec![Register::Shutdown.addr(), 0x01]),
            Transaction::transaction_end(),
        ];
        let mut spi = SpiMock::new(&expected_transactions);
        let mut driver = Max7219::new(&mut spi);

        driver.power_on().expect("Power on should succeed");
        spi.done();
    }

    #[test]
    fn test_power_off() {
        let expected_transactions = [
            Transaction::transaction_start(),
            Transaction::write_vec(vec![Register::Shutdown.addr(), 0x00]),
            Transaction::transaction_end(),
        ];
        let mut spi = SpiMock::new(&expected_transactions);
        let mut driver = Max7219::new(&mut spi);

        driver.power_off().expect("Power off should succeed");
        spi.done();
    }

    #[test]
    fn test_power_on_device() {
        let expected_transactions = [
            Transaction::transaction_start(),
            Transaction::write_vec(vec![Register::Shutdown.addr(), 0x01]),
            Transaction::transaction_end(),
        ];
        let mut spi = SpiMock::new(&expected_transactions);
        let mut driver = Max7219::new(&mut spi);

        driver
            .power_on_device(0)
            .expect("Power on display should succeed");
        spi.done();
    }

    // Test with multiple devices - power_on
    #[test]
    fn test_power_on_multiple_devices() {
        let device_count = 3;

        let expected_transactions = [
            Transaction::transaction_start(),
            Transaction::write_vec(vec![
                Register::Shutdown.addr(),
                0x01,
                Register::Shutdown.addr(),
                0x01,
                Register::Shutdown.addr(),
                0x01,
            ]),
            Transaction::transaction_end(),
        ];
        let mut spi = SpiMock::new(&expected_transactions);
        let mut driver = Max7219::new(&mut spi)
            .with_device_count(device_count)
            .expect("Should accept valid count");

        driver.power_on().expect("Power on should succeed");
        spi.done();
    }

    #[test]
    fn test_power_off_device() {
        let expected_transactions = [
            Transaction::transaction_start(),
            Transaction::write_vec(vec![
                // For 4 devices
                Register::NoOp.addr(),
                0x00,
                Register::NoOp.addr(),
                0x00,
                Register::Shutdown.addr(),
                0x00,
                Register::NoOp.addr(),
                0x00,
            ]),
            Transaction::transaction_end(),
        ];
        let mut spi = SpiMock::new(&expected_transactions);
        let mut driver = Max7219::new(&mut spi)
            .with_device_count(4)
            .expect("a valid device count");

        driver
            .power_off_device(2)
            .expect("Power off display should succeed");
        spi.done();
    }

    #[test]
    fn test_power_device_invalid_index() {
        let mut spi = SpiMock::new(&[]);
        let mut driver = Max7219::new(&mut spi).with_device_count(1).unwrap();

        let result = driver.power_on_device(1);
        assert_eq!(result, Err(Error::InvalidDeviceIndex));

        let result = driver.power_off_device(1);
        assert_eq!(result, Err(Error::InvalidDeviceIndex));
        spi.done();
    }

    #[test]
    fn test_test_all_enable() {
        let expected_transactions = [
            Transaction::transaction_start(),
            Transaction::write_vec(vec![
                Register::DisplayTest.addr(),
                0x01,
                Register::DisplayTest.addr(),
                0x01,
                Register::DisplayTest.addr(),
                0x01,
                Register::DisplayTest.addr(),
                0x01,
            ]),
            Transaction::transaction_end(),
        ];
        let mut spi = SpiMock::new(&expected_transactions);
        let mut driver = Max7219::new(&mut spi)
            .with_device_count(4)
            .expect("valid device count");

        driver
            .test_all(true)
            .expect("Test all enable should succeed");
        spi.done();
    }

    #[test]
    fn test_test_all_disable() {
        let expected_transactions = [
            Transaction::transaction_start(),
            Transaction::write_vec(vec![Register::DisplayTest.addr(), 0x00]),
            Transaction::transaction_end(),
        ];
        let mut spi = SpiMock::new(&expected_transactions);
        let mut driver = Max7219::new(&mut spi);

        driver
            .test_all(false)
            .expect("Test all disable should succeed");
        spi.done();
    }

    #[test]
    fn test_set_scan_limit_all_valid() {
        let limit = 4;
        let expected_transactions = [
            Transaction::transaction_start(),
            Transaction::write_vec(vec![Register::ScanLimit.addr(), limit - 1]),
            Transaction::transaction_end(),
        ];
        let mut spi = SpiMock::new(&expected_transactions);
        let mut driver = Max7219::new(&mut spi);

        driver
            .set_scan_limit_all(limit)
            .expect("Set scan limit should succeed");
        spi.done();
    }

    #[test]
    fn test_set_scan_limit_all_invalid_low() {
        let mut spi = SpiMock::new(&[]);
        let mut driver = Max7219::new(&mut spi);

        let result = driver.set_scan_limit_all(0);
        assert_eq!(result, Err(Error::InvalidScanLimit));
        spi.done();
    }

    #[test]
    fn test_set_scan_limit_all_invalid_high() {
        let mut spi = SpiMock::new(&[]); // No transactions expected for invalid input
        let mut driver = Max7219::new(&mut spi);

        let result = driver.set_scan_limit_all(9);
        assert_eq!(result, Err(Error::InvalidScanLimit));
        spi.done();
    }

    #[test]
    fn test_clear_display() {
        let mut expected_transactions = Vec::new();
        for digit_register in Register::digits() {
            expected_transactions.push(Transaction::transaction_start());
            expected_transactions.push(Transaction::write_vec(vec![digit_register.addr(), 0x00]));
            expected_transactions.push(Transaction::transaction_end());
        }

        let mut spi = SpiMock::new(&expected_transactions);
        let mut driver = Max7219::new(&mut spi);

        driver
            .clear_display(0)
            .expect("Clear display should succeed");
        spi.done();
    }

    #[test]
    fn test_clear_display_invalid_index() {
        let mut spi = SpiMock::new(&[]); // No transactions expected for invalid index
        let mut driver = Max7219::new(&mut spi)
            .with_device_count(1)
            .expect("valid device count");

        let result = driver.clear_display(1);
        assert_eq!(result, Err(Error::InvalidDeviceIndex));
        spi.done();
    }

    #[test]
    fn test_set_intensity_valid() {
        let device_index = 0;
        let intensity = 0x0A;
        let expected_transactions = [
            Transaction::transaction_start(),
            Transaction::write_vec(vec![Register::Intensity.addr(), intensity]),
            Transaction::transaction_end(),
        ];
        let mut spi = SpiMock::new(&expected_transactions);
        let mut driver = Max7219::new(&mut spi);

        driver
            .set_intensity(device_index, intensity)
            .expect("Set intensity should succeed");
        spi.done();
    }

    #[test]
    fn test_set_intensity_invalid() {
        let mut spi = SpiMock::new(&[]); // No transactions expected for invalid input
        let mut driver = Max7219::new(&mut spi);

        let result = driver.set_intensity(0, 0x10); // Invalid intensity > 0x0F
        assert_eq!(result, Err(Error::InvalidIntensity));
        spi.done();
    }

    #[test]
    fn test_test_device_enable_disable() {
        let expected_transactions = [
            Transaction::transaction_start(),
            Transaction::write_vec(vec![Register::DisplayTest.addr(), 0x01]),
            Transaction::transaction_end(),
            Transaction::transaction_start(),
            Transaction::write_vec(vec![Register::DisplayTest.addr(), 0x00]),
            Transaction::transaction_end(),
        ];
        let mut spi = SpiMock::new(&expected_transactions);
        let mut driver = Max7219::new(&mut spi);

        driver
            .test_device(0, true)
            .expect("Enable test mode failed");
        driver
            .test_device(0, false)
            .expect("Disable test mode failed");
        spi.done();
    }

    #[test]
    fn test_set_device_scan_limit_valid() {
        let scan_limit = 4;

        let expected_transactions = [
            Transaction::transaction_start(),
            Transaction::write_vec(vec![Register::ScanLimit.addr(), scan_limit - 1]),
            Transaction::transaction_end(),
        ];
        let mut spi = SpiMock::new(&expected_transactions);
        let mut driver = Max7219::new(&mut spi);

        driver
            .set_device_scan_limit(0, scan_limit)
            .expect("Scan limit set failed");
        spi.done();
    }

    #[test]
    fn test_set_device_scan_limit_invalid() {
        let mut spi = SpiMock::new(&[]);
        let mut driver = Max7219::new(&mut spi);

        let result = driver.set_device_scan_limit(0, 0); // invalid: below range
        assert_eq!(result, Err(Error::InvalidScanLimit));

        let result = driver.set_device_scan_limit(0, 9); // invalid: above range
        assert_eq!(result, Err(Error::InvalidScanLimit));
        spi.done();
    }

    #[test]
    fn test_set_intensity_all() {
        let intensity = 0x05;
        let expected_transactions = [
            Transaction::transaction_start(),
            Transaction::write_vec(vec![
                Register::Intensity.addr(),
                intensity,
                Register::Intensity.addr(),
                intensity,
            ]),
            Transaction::transaction_end(),
        ];
        let mut spi = SpiMock::new(&expected_transactions);
        let mut driver = Max7219::new(&mut spi)
            .with_device_count(2)
            .expect("valid count");

        driver
            .set_intensity_all(intensity)
            .expect("Set intensity all failed");
        spi.done();
    }
}
