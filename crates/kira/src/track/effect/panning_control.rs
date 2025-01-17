//! Adjusts the panning of audio.

mod builder;
mod handle;

pub use builder::*;
pub use handle::*;

use ringbuf::Consumer;

use crate::{
	clock::ClockTime,
	dsp::Frame,
	tween::{Tween, Tweener},
};

use super::Effect;

enum Command {
	SetPanning(f64, Tween),
}

struct PanningControl {
	command_consumer: Consumer<Command>,
	panning: Tweener,
}

impl PanningControl {
	fn new(builder: PanningControlBuilder, command_consumer: Consumer<Command>) -> Self {
		Self {
			command_consumer,
			panning: Tweener::new(builder.0),
		}
	}
}

impl Effect for PanningControl {
	fn on_start_processing(&mut self) {
		while let Some(command) = self.command_consumer.pop() {
			match command {
				Command::SetPanning(panning, tween) => self.panning.set(panning, tween),
			}
		}
	}

	fn process(&mut self, input: Frame, dt: f64) -> Frame {
		self.panning.update(dt);
		input.panned(self.panning.value() as f32)
	}

	fn on_clock_tick(&mut self, time: ClockTime) {
		self.panning.on_clock_tick(time);
	}
}
