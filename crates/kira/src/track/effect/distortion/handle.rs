use ringbuf::Producer;

use crate::{tween::Tween, CommandError, Volume};

use super::{Command, DistortionKind};

/// Controls a distortion effect.
pub struct DistortionHandle {
	pub(super) command_producer: Producer<Command>,
}

impl DistortionHandle {
	/// Sets the kind of distortion to use.
	pub fn set_kind(&mut self, kind: DistortionKind) -> Result<(), CommandError> {
		self.command_producer
			.push(Command::SetKind(kind))
			.map_err(|_| CommandError::CommandQueueFull)
	}

	/// Sets how much distortion should be applied.
	pub fn set_drive(
		&mut self,
		drive: impl Into<Volume>,
		tween: Tween,
	) -> Result<(), CommandError> {
		self.command_producer
			.push(Command::SetDrive(drive.into(), tween))
			.map_err(|_| CommandError::CommandQueueFull)
	}

	/// Sets how much dry (unprocessed) signal should be blended
	/// with the wet (processed) signal. `0.0` means only the dry
	/// signal will be heard. `1.0` means only the wet signal will
	/// be heard.
	pub fn set_mix(&mut self, mix: f64, tween: Tween) -> Result<(), CommandError> {
		self.command_producer
			.push(Command::SetMix(mix, tween))
			.map_err(|_| CommandError::CommandQueueFull)
	}
}
