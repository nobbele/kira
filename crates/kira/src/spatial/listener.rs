mod handle;
mod settings;

pub use handle::*;
pub use settings::*;

use std::sync::{
	atomic::{AtomicBool, Ordering},
	Arc,
};

use atomic_arena::{Arena, Key};

use crate::{
	dsp::Frame,
	math::{Quaternion, Vec3},
	track::TrackId,
	tween::Tweenable,
	Volume,
};

use super::{emitter::Emitter, scene::SpatialSceneId};

const EAR_DISTANCE: f32 = 0.1;

pub(crate) struct Listener {
	shared: Arc<ListenerShared>,
	position: Vec3,
	orientation: Quaternion,
	track: TrackId,
}

impl Listener {
	pub fn new(settings: ListenerSettings) -> Self {
		Self {
			shared: Arc::new(ListenerShared::new()),
			position: settings.position,
			orientation: settings.orientation,
			track: settings.track,
		}
	}

	pub fn shared(&self) -> Arc<ListenerShared> {
		self.shared.clone()
	}

	pub fn track(&self) -> TrackId {
		self.track
	}

	pub fn set_position(&mut self, position: Vec3) {
		self.position = position;
	}

	pub fn process(&mut self, emitters: &Arena<Emitter>) -> Frame {
		let mut output = Frame::ZERO;
		for (_, emitter) in emitters {
			let mut emitter_output = emitter.output();
			// attenuate volume
			if let Some(attenuation_function) = emitter.attenuation_function() {
				let distance = (emitter.position() - self.position).magnitude();
				let relative_distance = emitter.distances().relative_distance(distance);
				let relative_volume =
					attenuation_function.apply((1.0 - relative_distance).into()) as f32;
				let amplitude = Tweenable::lerp(
					Volume::Decibels(Volume::MIN_DECIBELS),
					Volume::Decibels(0.0),
					relative_volume.into(),
				)
				.as_amplitude() as f32;
				emitter_output *= amplitude;
			}
			// apply spatialization
			if emitter.enable_spatialization() {
				let (left_ear_position, right_ear_position) = self.ear_positions();
				let left_ear_direction = self.orientation.rotate_point(Vec3::LEFT);
				let right_ear_direction = self.orientation.rotate_point(Vec3::RIGHT);
				let emitter_direction_relative_to_left_ear =
					(emitter.position() - left_ear_position).normalized();
				let emitter_direction_relative_to_right_ear =
					(emitter.position() - right_ear_position).normalized();
				let left_ear_volume =
					(left_ear_direction.dot(emitter_direction_relative_to_left_ear) + 1.0) / 2.0;
				let right_ear_volume =
					(right_ear_direction.dot(emitter_direction_relative_to_right_ear) + 1.0) / 2.0;
				emitter_output.left *= left_ear_volume;
				emitter_output.right *= right_ear_volume;
			}
			output += emitter_output;
		}
		output
	}

	fn ear_positions(&self) -> (Vec3, Vec3) {
		let left = self.position + self.orientation.rotate_point(Vec3::LEFT * EAR_DISTANCE);
		let right = self.position + self.orientation.rotate_point(Vec3::RIGHT * EAR_DISTANCE);
		(left, right)
	}
}

/// A unique identifier for an listener.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ListenerId {
	pub(crate) key: Key,
	pub(crate) scene_id: SpatialSceneId,
}

impl ListenerId {
	/// Returns the ID of the spatial scene this listener belongs to.
	pub fn scene(&self) -> SpatialSceneId {
		self.scene_id
	}
}

pub(crate) struct ListenerShared {
	removed: AtomicBool,
}

impl ListenerShared {
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
