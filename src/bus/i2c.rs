use embedded_hal::delay::DelayNs;
use embedded_hal::i2c::I2c;

use crate::error::{Error, Port};
use crate::{bus::DataBus, error::Result};

pub struct I2CBus<I2C> {
	i2c_bus: I2C,
	address: u8,
	backlight: bool,
}

const BACKLIGHT: u8 = 0b0000_1000;
const ENABLE: u8 = 0b0000_0100;
// const READ_WRITE: u8 = 0b0000_0010; // Not used as no reading of the `HD44780` is done
const REGISTER_SELECT: u8 = 0b0000_0001;

impl<I2C> I2CBus<I2C> {
	pub fn new(i2c_bus: I2C, address: u8) -> I2CBus<I2C> {
		I2CBus { i2c_bus, address, backlight: true }
	}

	pub fn destroy(self) -> I2C {
		self.i2c_bus
	}
}

impl<I2C: I2c> I2CBus<I2C> {
	/// Write a nibble to the lcd
	/// The nibble should be in the upper part of the byte
	fn write_nibble<D: DelayNs>(&mut self, nibble: u8, data: bool, delay: &mut D) -> Result<(), I2C::Error> {
		let rs = match data {
			false => 0u8,
			true => REGISTER_SELECT,
		};
		let byte = nibble | rs | if self.backlight { BACKLIGHT } else { 0 };

		self.i2c_bus.write(self.address, &[byte, byte | ENABLE]).map_err(Error::wrap_io(Port::I2C))?;
		delay.delay_ms(2u32);
		self.i2c_bus.write(self.address, &[byte]).map_err(Error::wrap_io(Port::I2C))
	}
}

impl<I2C: I2c> DataBus for I2CBus<I2C> {
	type Error = I2C::Error;

	fn write<D: DelayNs>(&mut self, byte: u8, data: bool, delay: &mut D) -> Result<(), Self::Error> {
		let upper_nibble = byte & 0xF0;
		self.write_nibble(upper_nibble, data, delay)?;

		let lower_nibble = (byte & 0x0F) << 4;
		self.write_nibble(lower_nibble, data, delay)?;

		Ok(())
	}

	fn set_backlight<D: DelayNs>(&mut self, state: bool, delay: &mut D) -> Result<(), Self::Error> {
		self.backlight = state;
		self.write(0, false, delay)?;

		Ok(())
	}
}

#[cfg(feature = "async")]
mod non_blocking {
	use core::future::Future;
	use embedded_hal_async::delay::DelayNs;
	use embedded_hal_async::i2c::I2c;

	use crate::{
		error::{Error, Port, Result},
		non_blocking::bus::DataBus,
	};

	use super::{I2CBus, BACKLIGHT, ENABLE, REGISTER_SELECT};

	impl<I2C: I2c> I2CBus<I2C> {
		/// Write a nibble to the lcd
		/// The nibble should be in the upper part of the byte
		async fn write_nibble_non_blocking<'a, D: DelayNs + 'a>(
			&mut self,
			nibble: u8,
			data: bool,
			delay: &'a mut D,
		) -> Result<(), I2C::Error> {
			let rs = match data {
				false => 0u8,
				true => REGISTER_SELECT,
			};
			let byte = nibble | rs | if self.backlight { BACKLIGHT } else { 0 };

			self.i2c_bus.write(self.address, &[byte, byte | ENABLE]).await.map_err(Error::wrap_io(Port::I2C))?;
			delay.delay_ms(2).await;
			self.i2c_bus.write(self.address, &[byte]).await.map_err(Error::wrap_io(Port::I2C))
		}
	}

	impl<I2C: I2c + 'static> DataBus for I2CBus<I2C> {
		type Error = I2C::Error;

		type WriteFuture<'a, D: 'a + DelayNs> = impl Future<Output = Result<(), Self::Error>> + 'a;
		type SetBacklightFuture<'a, D: 'a + DelayNs> = impl Future<Output = Result<(), Self::Error>> + 'a;

		fn write<'a, D: DelayNs + 'a>(
			&'a mut self,
			byte: u8,
			data: bool,
			delay: &'a mut D,
		) -> Self::WriteFuture<'a, D> {
			async move {
				let upper_nibble = byte & 0xF0;
				self.write_nibble_non_blocking(upper_nibble, data, delay).await?;

				let lower_nibble = (byte & 0x0F) << 4;
				self.write_nibble_non_blocking(lower_nibble, data, delay).await?;

				Ok(())
			}
		}

		fn set_backlight<'a, D: DelayNs + 'a>(
			&'a mut self,
			state: bool,
			delay: &'a mut D,
		) -> Self::SetBacklightFuture<'a, D> {
			async move {
				self.backlight = state;
				self.write(0, false, delay).await?;

				Ok(())
			}
		}
	}
}
