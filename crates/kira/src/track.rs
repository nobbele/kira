//! Organizes and applies effects to audio.

mod builder;
pub mod effect;
mod handle;
mod routes;

#[cfg(test)]
mod test;

pub use builder::*;
pub use handle::*;
pub use routes::*;

use std::sync::{
	atomic::{AtomicBool, Ordering},
	Arc,
};

use atomic_arena::Key;

use crate::{
	clock::ClockTime,
	dsp::Frame,
	tween::{Tween, Tweener},
	Volume,
};

use self::effect::Effect;

/// A unique identifier for a mixer sub-track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubTrackId(pub(crate) Key);

/// A unique identifier for a track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrackId {
	/// The main mixer track.
	Main,
	/// A sub-track.
	Sub(SubTrackId),
}

impl From<SubTrackId> for TrackId {
	fn from(id: SubTrackId) -> Self {
		Self::Sub(id)
	}
}

impl From<&TrackHandle> for TrackId {
	fn from(handle: &TrackHandle) -> Self {
		handle.id()
	}
}

pub(crate) struct TrackShared {
	removed: AtomicBool,
}

impl TrackShared {
	pub fn new() -> Self {
		Self {
			removed: AtomicBool::new(false),
		}
	}

	pub fn is_marked_for_removal(&self) -> bool {
		self.removed.load(Ordering::SeqCst)
	}

	pub fn mark_for_removal(&self) {
		self.removed.store(true, Ordering::SeqCst);
	}
}

pub(crate) struct Track {
	shared: Arc<TrackShared>,
	volume: Tweener<Volume>,
	routes: Vec<(TrackId, Tweener<Volume>)>,
	effects: Vec<Box<dyn Effect>>,
	input: Frame,
}

impl Track {
	pub fn new(builder: TrackBuilder) -> Self {
		Self {
			shared: Arc::new(TrackShared::new()),
			volume: Tweener::new(builder.volume),
			routes: builder.routes.into_vec(),
			effects: builder.effects,
			input: Frame::ZERO,
		}
	}

	pub fn init_effects(&mut self, sample_rate: u32) {
		for effect in &mut self.effects {
			effect.init(sample_rate);
		}
	}

	pub fn on_change_sample_rate(&mut self, sample_rate: u32) {
		for effect in &mut self.effects {
			effect.on_change_sample_rate(sample_rate);
		}
	}

	pub fn shared(&self) -> Arc<TrackShared> {
		self.shared.clone()
	}

	pub fn routes_mut(&mut self) -> &mut Vec<(TrackId, Tweener<Volume>)> {
		&mut self.routes
	}

	pub fn set_volume(&mut self, volume: Volume, tween: Tween) {
		self.volume.set(volume, tween);
	}

	pub fn set_route(&mut self, to: TrackId, volume: Volume, tween: Tween) {
		// TODO: determine if we should store the track routes in some
		// other data structure like an IndexMap so we don't have to do
		// linear search
		if let Some(route) =
			self.routes
				.iter_mut()
				.find_map(|(id, route)| if *id == to { Some(route) } else { None })
		{
			route.set(volume, tween);
		}
	}

	pub fn add_input(&mut self, input: Frame) {
		self.input += input;
	}

	pub fn on_start_processing(&mut self) {
		for effect in &mut self.effects {
			effect.on_start_processing();
		}
	}

	pub fn process(&mut self, dt: f64) -> Frame {
		self.volume.update(dt);
		for (_, route) in &mut self.routes {
			route.update(dt);
		}
		let mut output = std::mem::replace(&mut self.input, Frame::ZERO);
		for effect in &mut self.effects {
			output = effect.process(output, dt);
		}
		output * self.volume.value().as_amplitude() as f32
	}

	pub fn on_clock_tick(&mut self, time: ClockTime) {
		self.volume.on_clock_tick(time);
		for (_, route) in &mut self.routes {
			route.on_clock_tick(time);
		}
		for effect in &mut self.effects {
			effect.on_clock_tick(time);
		}
	}
}
