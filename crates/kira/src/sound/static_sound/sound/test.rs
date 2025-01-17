use std::{sync::Arc, time::Duration};

use crate::{
	clock::ClockTime,
	dsp::Frame,
	manager::{backend::mock::MockBackend, AudioManager},
	sound::{
		static_sound::{PlaybackState, StaticSoundData, StaticSoundSettings},
		Sound,
	},
	tween::Tween,
	ClockSpeed, LoopBehavior, StartTime, Volume,
};

use super::StaticSound;

/// Tests that a `StaticSound` will play all of its samples before finishing.
#[test]
fn plays_all_samples() {
	let data = StaticSoundData {
		sample_rate: 1,
		frames: Arc::new(vec![
			Frame::from_mono(1.0),
			Frame::from_mono(2.0),
			Frame::from_mono(3.0),
		]),
		settings: StaticSoundSettings::new(),
	};
	let (mut sound, _) = data.split();

	assert!(!sound.finished());

	for i in 1..=3 {
		assert_eq!(sound.process(1.0), Frame::from_mono(i as f32).panned(0.5));
		assert!(!sound.finished());
	}

	// give some time for the resample buffer to empty. in the meantime we should
	// get silent output.
	for _ in 0..10 {
		assert_eq!(sound.process(1.0), Frame::from_mono(0.0).panned(0.5));
	}

	// the sound should be finished and stopped by now
	assert!(sound.finished());
	assert_eq!(sound.state, PlaybackState::Stopped);
}

/// Tests that a `StaticSound` correctly reports its playback state
/// to be queried by StaticSoundHandle::state.
#[test]
fn reports_playback_state() {
	let data = StaticSoundData {
		sample_rate: 1,
		frames: Arc::new(vec![Frame::from_mono(0.0); 10]),
		settings: StaticSoundSettings::new(),
	};
	let (mut sound, handle) = data.split();

	for _ in 0..20 {
		assert_eq!(handle.state(), sound.state);
		sound.process(1.0);
	}
}

/// Tests that a `StaticSound` correctly reports its playback state
/// to be queried by StaticSoundHandle::state.
#[test]
#[allow(clippy::float_cmp)]
fn reports_playback_position() {
	let data = StaticSoundData {
		sample_rate: 1,
		frames: Arc::new(vec![Frame::from_mono(0.0); 10]),
		settings: StaticSoundSettings::new(),
	};
	let (mut sound, handle) = data.split();

	for i in 0..20 {
		assert_eq!(
			handle.position(),
			i.clamp(0, 9) as f64 / sound.data.sample_rate as f64
		);
		sound.process(1.0);
		sound.on_start_processing();
	}
}

/// Tests that a `StaticSound` fades out fully before pausing
/// and fades back in when resuming.
#[test]
#[allow(clippy::float_cmp)]
fn pauses_and_resumes_with_fades() {
	let data = StaticSoundData {
		sample_rate: 1,
		frames: Arc::new(vec![Frame::from_mono(1.0); 100]),
		settings: StaticSoundSettings::new(),
	};
	let (mut sound, mut handle) = data.split();

	sound.process(1.0);
	assert_eq!(sound.state, PlaybackState::Playing);

	handle
		.pause(Tween {
			duration: Duration::from_secs(4),
			..Default::default()
		})
		.unwrap();
	sound.on_start_processing();
	assert_eq!(sound.state, PlaybackState::Pausing);

	// allow for a few samples of delay because of the resampling, but the
	// sound should fade out soon.
	expect_frame_soon(
		Frame::from_mono(Volume::Decibels(-15.0).as_amplitude() as f32).panned(0.5),
		&mut sound,
	);
	assert_eq!(
		sound.process(1.0),
		Frame::from_mono(Volume::Decibels(-30.0).as_amplitude() as f32).panned(0.5)
	);
	assert_eq!(
		sound.process(1.0),
		Frame::from_mono(Volume::Decibels(-45.0).as_amplitude() as f32).panned(0.5)
	);

	sound.on_start_processing();
	let position = handle.position();
	for _ in 0..10 {
		assert_eq!(sound.process(1.0), Frame::from_mono(0.0).panned(0.5));
		sound.on_start_processing();
		assert_eq!(handle.position(), position);
		assert_eq!(sound.state, PlaybackState::Paused);
	}

	handle
		.resume(Tween {
			duration: Duration::from_secs(4),
			..Default::default()
		})
		.unwrap();
	sound.on_start_processing();

	// allow for a few samples of delay because of the resampling, but the
	// sound should fade back in soon.
	expect_frame_soon(
		Frame::from_mono(Volume::Decibels(-45.0).as_amplitude() as f32).panned(0.5),
		&mut sound,
	);
	assert_eq!(sound.state, PlaybackState::Playing);
	assert_eq!(
		sound.process(1.0),
		Frame::from_mono(Volume::Decibels(-30.0).as_amplitude() as f32).panned(0.5)
	);
	assert_eq!(sound.state, PlaybackState::Playing);
	assert_eq!(
		sound.process(1.0),
		Frame::from_mono(Volume::Decibels(-15.0).as_amplitude() as f32).panned(0.5)
	);
	assert_eq!(sound.state, PlaybackState::Playing);

	for _ in 0..3 {
		assert_eq!(sound.process(1.0), Frame::from_mono(1.0).panned(0.5));
		assert_eq!(sound.state, PlaybackState::Playing);
	}
}

/// Tests that a `StaticSound` stops and finishes after a fade-out.
#[test]
#[allow(clippy::float_cmp)]
fn stops_with_fade_out() {
	let data = StaticSoundData {
		sample_rate: 1,
		frames: Arc::new(vec![Frame::from_mono(1.0); 100]),
		settings: StaticSoundSettings::new(),
	};
	let (mut sound, mut handle) = data.split();

	sound.process(1.0);
	assert_eq!(sound.state, PlaybackState::Playing);

	handle
		.stop(Tween {
			duration: Duration::from_secs(4),
			..Default::default()
		})
		.unwrap();
	sound.on_start_processing();
	assert_eq!(sound.state, PlaybackState::Stopping);

	// allow for a few samples of delay because of the resampling, but the
	// sound should fade out soon.
	expect_frame_soon(
		Frame::from_mono(Volume::Decibels(-15.0).as_amplitude() as f32).panned(0.5),
		&mut sound,
	);
	assert_eq!(
		sound.process(1.0),
		Frame::from_mono(Volume::Decibels(-30.0).as_amplitude() as f32).panned(0.5)
	);
	assert_eq!(
		sound.process(1.0),
		Frame::from_mono(Volume::Decibels(-45.0).as_amplitude() as f32).panned(0.5)
	);

	sound.on_start_processing();
	let position = handle.position();
	for _ in 0..3 {
		assert_eq!(sound.process(1.0), Frame::from_mono(0.0).panned(0.5));
		sound.on_start_processing();
		assert_eq!(handle.position(), position);
		assert_eq!(sound.state, PlaybackState::Stopped);
		assert!(sound.finished());
	}
}

/// Tests that a `StaticSound` can be paused and resumed on a clock tick.
#[test]
fn pauses_resumes_and_stops_on_clock_tick() {
	let mut manager = AudioManager::<MockBackend>::new(Default::default()).unwrap();
	let clock = manager.add_clock(ClockSpeed::SecondsPerTick(1.0)).unwrap();
	let data = StaticSoundData {
		sample_rate: 1,
		frames: Arc::new(vec![Frame::from_mono(1.0); 100]),
		settings: StaticSoundSettings::new(),
	};
	let (mut sound, mut handle) = data.split();

	// pause on clock tick
	handle
		.pause(Tween {
			duration: Duration::ZERO,
			start_time: StartTime::ClockTime(clock.time() + 1),
			..Default::default()
		})
		.unwrap();
	sound.on_start_processing();
	for _ in 0..10 {
		assert_eq!(sound.process(1.0), Frame::from_mono(1.0).panned(0.5));
	}
	sound.on_clock_tick(ClockTime {
		clock: clock.id(),
		ticks: 1,
	});
	expect_frame_soon(Frame::ZERO, &mut sound);
	sound.on_start_processing();
	assert_eq!(handle.state(), PlaybackState::Paused);

	// resume on clock tick
	handle
		.resume(Tween {
			duration: Duration::ZERO,
			start_time: StartTime::ClockTime(clock.time() + 2),
			..Default::default()
		})
		.unwrap();
	sound.on_start_processing();
	for _ in 0..10 {
		assert_eq!(sound.process(1.0), Frame::ZERO);
	}
	sound.on_clock_tick(ClockTime {
		clock: clock.id(),
		ticks: 2,
	});
	expect_frame_soon(Frame::from_mono(1.0).panned(0.5), &mut sound);
	sound.on_start_processing();
	assert_eq!(handle.state(), PlaybackState::Playing);

	// stop on clock tick
	handle
		.stop(Tween {
			duration: Duration::ZERO,
			start_time: StartTime::ClockTime(clock.time() + 1),
			..Default::default()
		})
		.unwrap();
	sound.on_start_processing();
	for _ in 0..10 {
		assert_eq!(sound.process(1.0), Frame::from_mono(1.0).panned(0.5));
	}
	sound.on_clock_tick(ClockTime {
		clock: clock.id(),
		ticks: 1,
	});
	expect_frame_soon(Frame::ZERO, &mut sound);
	sound.on_start_processing();
	assert_eq!(handle.state(), PlaybackState::Stopped);
}

/// Tests that a `StaticSound` will wait for its start clock time
/// when appropriate.
#[test]
#[allow(clippy::float_cmp)]
fn waits_for_start_time() {
	// create some fake ClockIds
	let mut manager = AudioManager::<MockBackend>::new(Default::default()).unwrap();
	let clock_id_1 = manager
		.add_clock(ClockSpeed::TicksPerSecond(1.0))
		.unwrap()
		.id();
	let clock_id_2 = manager
		.add_clock(ClockSpeed::TicksPerSecond(1.0))
		.unwrap()
		.id();

	let data = StaticSoundData {
		sample_rate: 1,
		frames: Arc::new((1..100).map(|i| Frame::from_mono(i as f32)).collect()),
		settings: StaticSoundSettings::new().start_time(ClockTime {
			clock: clock_id_1,
			ticks: 2,
		}),
	};
	let (mut sound, _) = data.split();

	// the sound should not be playing yet
	for _ in 0..3 {
		assert_eq!(sound.process(1.0), Frame::from_mono(0.0));
	}

	// the sound is set to start at tick 2, so it should not
	// play yet
	sound.on_clock_tick(ClockTime {
		clock: clock_id_1,
		ticks: 1,
	});

	for _ in 0..3 {
		assert_eq!(sound.process(1.0), Frame::from_mono(0.0));
	}

	// this is a tick event for a different clock, so the
	// sound should not play yet
	sound.on_clock_tick(ClockTime {
		clock: clock_id_2,
		ticks: 2,
	});

	for _ in 0..3 {
		assert_eq!(sound.process(1.0), Frame::from_mono(0.0));
	}

	// the sound should start playing now
	sound.on_clock_tick(ClockTime {
		clock: clock_id_1,
		ticks: 2,
	});

	for i in 1..10 {
		assert_eq!(sound.process(1.0), Frame::from_mono(i as f32).panned(0.5));
	}
}

/// Tests that a `StaticSound` can be paused and resumed immediately
/// even if playback is waiting for a clock time to start.
#[test]
fn immediate_pause_and_resume_with_clock_start_time() {
	let mut manager = AudioManager::<MockBackend>::new(Default::default()).unwrap();
	let clock_id_1 = manager
		.add_clock(ClockSpeed::TicksPerSecond(1.0))
		.unwrap()
		.id();

	let data = StaticSoundData {
		sample_rate: 1,
		frames: Arc::new((1..100).map(|i| Frame::from_mono(i as f32)).collect()),
		settings: StaticSoundSettings::new().start_time(ClockTime {
			clock: clock_id_1,
			ticks: 2,
		}),
	};
	let (mut sound, _) = data.split();

	sound.pause(Tween {
		duration: Duration::from_secs(0),
		..Default::default()
	});
	sound.process(1.0);
	assert!(sound.state == PlaybackState::Paused);
	sound.resume(Tween {
		duration: Duration::from_secs(0),
		..Default::default()
	});
	sound.process(1.0);
	assert!(sound.state == PlaybackState::Playing);
}

/// Tests that a `StaticSound` can be stopped immediately even if playback
/// is waiting for a clock time to start.
#[test]
fn immediate_stop_with_clock_start_time() {
	let mut manager = AudioManager::<MockBackend>::new(Default::default()).unwrap();
	let clock_id_1 = manager
		.add_clock(ClockSpeed::TicksPerSecond(1.0))
		.unwrap()
		.id();

	let data = StaticSoundData {
		sample_rate: 1,
		frames: Arc::new((1..100).map(|i| Frame::from_mono(i as f32)).collect()),
		settings: StaticSoundSettings::new().start_time(ClockTime {
			clock: clock_id_1,
			ticks: 2,
		}),
	};
	let (mut sound, _) = data.split();

	sound.stop(Tween {
		duration: Duration::from_secs(0),
		..Default::default()
	});
	sound.process(1.0);
	assert!(sound.state == PlaybackState::Stopped);
}

/// Tests that a `StaticSound` can be started partway through the sound.
#[test]
#[allow(clippy::float_cmp)]
fn start_position() {
	let data = StaticSoundData {
		sample_rate: 1,
		frames: Arc::new((0..10).map(|i| Frame::from_mono(i as f32)).collect()),
		settings: StaticSoundSettings::new().start_position(3.0),
	};
	let (mut sound, handle) = data.split();

	assert_eq!(handle.position(), 3.0);
	assert_eq!(sound.process(1.0), Frame::from_mono(3.0).panned(0.5));
}

/// Tests that a `StaticSound` can be played backwards.
#[test]
#[allow(clippy::float_cmp)]
fn reverse() {
	let data = StaticSoundData {
		sample_rate: 1,
		frames: Arc::new((0..10).map(|i| Frame::from_mono(i as f32)).collect()),
		settings: StaticSoundSettings::new().reverse(true).start_position(2.0),
	};
	let (mut sound, _) = data.split();

	// start position should be from the end and decrease over time
	for i in (0..=7).rev() {
		assert_eq!(sound.process(1.0), Frame::from_mono(i as f32).panned(0.5));
	}
}

/// Tests that a `StaticSound` properly obeys looping behavior when
/// playing forward.
#[test]
#[allow(clippy::float_cmp)]
fn loops_forward() {
	let data = StaticSoundData {
		sample_rate: 1,
		frames: Arc::new((0..10).map(|i| Frame::from_mono(i as f32)).collect()),
		settings: StaticSoundSettings::new().loop_behavior(LoopBehavior {
			start_position: 3.0,
		}),
	};
	let (mut sound, _) = data.split();

	for i in 0..10 {
		assert_eq!(sound.process(1.0), Frame::from_mono(i as f32).panned(0.5));
	}

	assert_eq!(sound.process(3.0), Frame::from_mono(3.0).panned(0.5));
	assert_eq!(sound.process(3.0), Frame::from_mono(6.0).panned(0.5));
	assert_eq!(sound.process(3.0), Frame::from_mono(9.0).panned(0.5));
	assert_eq!(sound.process(3.0), Frame::from_mono(5.0).panned(0.5));
}

/// Tests that a `StaticSound` properly obeys looping behavior when
/// playing backward.
#[test]
#[allow(clippy::float_cmp)]
fn loops_backward() {
	let data = StaticSoundData {
		sample_rate: 1,
		frames: Arc::new((0..10).map(|i| Frame::from_mono(i as f32)).collect()),
		settings: StaticSoundSettings::new()
			.loop_behavior(LoopBehavior {
				start_position: 3.0,
			})
			.reverse(true),
	};
	let (mut sound, _) = data.split();

	for i in (3..10).rev() {
		assert_eq!(sound.process(1.0), Frame::from_mono(i as f32).panned(0.5));
	}

	assert_eq!(sound.process(4.0), Frame::from_mono(9.0).panned(0.5));
	assert_eq!(sound.process(4.0), Frame::from_mono(5.0).panned(0.5));
	assert_eq!(sound.process(4.0), Frame::from_mono(8.0).panned(0.5));
}

/// Tests that the volume of a `StaticSound` can be adjusted.
#[test]
#[allow(clippy::float_cmp)]
fn volume() {
	let data = StaticSoundData {
		sample_rate: 1,
		frames: Arc::new(vec![Frame::from_mono(1.0); 10]),
		settings: StaticSoundSettings::new().volume(0.5),
	};
	let (mut sound, _) = data.split();

	assert_eq!(sound.process(1.0), Frame::from_mono(0.5).panned(0.5));
}

/// Tests that the volume of a `StaticSound` can be changed
/// after the sound is started.
#[test]
fn set_volume() {
	let data = StaticSoundData {
		sample_rate: 1,
		frames: Arc::new(vec![Frame::from_mono(1.0); 10]),
		settings: StaticSoundSettings::new(),
	};
	let (mut sound, mut handle) = data.split();

	assert_eq!(sound.process(1.0), Frame::from_mono(1.0).panned(0.5));
	handle
		.set_volume(
			0.5,
			Tween {
				duration: Duration::ZERO,
				..Default::default()
			},
		)
		.unwrap();
	sound.on_start_processing();
	expect_frame_soon(Frame::from_mono(0.5).panned(0.5), &mut sound);
}

/// Tests that the volume of a `StaticSound` can be changed
/// on a clock tick.
#[test]
fn set_volume_on_clock_tick() {
	let mut manager = AudioManager::<MockBackend>::new(Default::default()).unwrap();
	let clock = manager.add_clock(ClockSpeed::TicksPerSecond(1.0)).unwrap();
	let data = StaticSoundData {
		sample_rate: 1,
		frames: Arc::new(vec![Frame::from_mono(1.0); 100]),
		settings: StaticSoundSettings::new(),
	};
	let (mut sound, mut handle) = data.split();

	handle
		.set_volume(
			0.5,
			Tween {
				duration: Duration::ZERO,
				start_time: StartTime::ClockTime(clock.time() + 1),
				..Default::default()
			},
		)
		.unwrap();
	sound.on_start_processing();
	for _ in 0..10 {
		assert_eq!(sound.process(1.0), Frame::from_mono(1.0).panned(0.5));
	}
	sound.on_clock_tick(ClockTime {
		clock: clock.id(),
		ticks: 1,
	});
	expect_frame_soon(Frame::from_mono(0.5).panned(0.5), &mut sound);
}

/// Tests that the panning of a `StaticSound` can be adjusted.
#[test]
#[allow(clippy::float_cmp)]
fn panning() {
	let data = StaticSoundData {
		sample_rate: 1,
		frames: Arc::new(vec![Frame::from_mono(1.0); 10]),
		settings: StaticSoundSettings::new().panning(0.0),
	};
	let (mut sound, _) = data.split();

	assert_eq!(sound.process(1.0), Frame::from_mono(1.0).panned(0.0));
}

/// Tests that the panning of a `StaticSound` can be changed
/// after the sound is started.
#[test]
fn set_panning() {
	let data = StaticSoundData {
		sample_rate: 1,
		frames: Arc::new(vec![Frame::from_mono(1.0); 10]),
		settings: StaticSoundSettings::new(),
	};
	let (mut sound, mut handle) = data.split();

	assert_eq!(sound.process(1.0), Frame::from_mono(1.0).panned(0.5));
	handle
		.set_panning(
			0.0,
			Tween {
				duration: Duration::ZERO,
				..Default::default()
			},
		)
		.unwrap();
	sound.on_start_processing();
	expect_frame_soon(Frame::from_mono(1.0).panned(0.0), &mut sound);
}

/// Tests that the panning of a `StaticSound` can be changed
/// on a clock tick.
#[test]
fn set_panning_on_clock_tick() {
	let mut manager = AudioManager::<MockBackend>::new(Default::default()).unwrap();
	let clock = manager.add_clock(ClockSpeed::TicksPerSecond(1.0)).unwrap();
	let data = StaticSoundData {
		sample_rate: 1,
		frames: Arc::new(vec![Frame::from_mono(1.0); 100]),
		settings: StaticSoundSettings::new(),
	};
	let (mut sound, mut handle) = data.split();

	handle
		.set_panning(
			0.0,
			Tween {
				duration: Duration::ZERO,
				start_time: StartTime::ClockTime(clock.time() + 1),
				..Default::default()
			},
		)
		.unwrap();
	sound.on_start_processing();
	for _ in 0..10 {
		assert_eq!(sound.process(1.0), Frame::from_mono(1.0).panned(0.5));
	}
	sound.on_clock_tick(ClockTime {
		clock: clock.id(),
		ticks: 1,
	});
	expect_frame_soon(Frame::from_mono(1.0).panned(0.0), &mut sound);
}

/// Tests that the playback rate of a `StaticSound` can be adjusted.
#[test]
#[allow(clippy::float_cmp)]
fn playback_rate() {
	let data = StaticSoundData {
		sample_rate: 1,
		frames: Arc::new((0..10).map(|i| Frame::from_mono(i as f32)).collect()),
		settings: StaticSoundSettings::new().playback_rate(2.0),
	};
	let (mut sound, _) = data.split();

	assert_eq!(sound.process(1.0), Frame::from_mono(0.0).panned(0.5));
	assert_eq!(sound.process(1.0), Frame::from_mono(2.0).panned(0.5));
}

/// Tests that the playback rate of a `StaticSound` can be adjusted after
/// it's started.
#[test]
#[allow(clippy::float_cmp)]
fn set_playback_rate() {
	let data = StaticSoundData {
		sample_rate: 1,
		frames: Arc::new((0..100).map(|i| Frame::from_mono(i as f32)).collect()),
		settings: StaticSoundSettings::new(),
	};
	let (mut sound, mut handle) = data.split();

	assert_eq!(sound.process(1.0), Frame::from_mono(0.0).panned(0.5));
	assert_eq!(sound.process(1.0), Frame::from_mono(1.0).panned(0.5));

	handle
		.set_playback_rate(
			2.0,
			Tween {
				duration: Duration::ZERO,
				..Default::default()
			},
		)
		.unwrap();
	sound.on_start_processing();

	assert_eq!(sound.process(1.0), Frame::from_mono(2.0).panned(0.5));
	assert_eq!(sound.process(1.0), Frame::from_mono(4.0).panned(0.5));
}

/// Tests that the playback rate of a `StaticSound` can be adjusted on
/// a clock tick.
#[test]
#[allow(clippy::float_cmp)]
fn set_playback_rate_on_clock_tick() {
	let mut manager = AudioManager::<MockBackend>::new(Default::default()).unwrap();
	let clock = manager.add_clock(ClockSpeed::TicksPerSecond(1.0)).unwrap();
	let data = StaticSoundData {
		sample_rate: 1,
		frames: Arc::new((0..100).map(|i| Frame::from_mono(i as f32)).collect()),
		settings: StaticSoundSettings::new(),
	};
	let (mut sound, mut handle) = data.split();

	handle
		.set_playback_rate(
			2.0,
			Tween {
				duration: Duration::ZERO,
				start_time: StartTime::ClockTime(clock.time() + 1),
				..Default::default()
			},
		)
		.unwrap();
	sound.on_start_processing();

	assert_eq!(sound.process(1.0), Frame::from_mono(0.0).panned(0.5));
	assert_eq!(sound.process(1.0), Frame::from_mono(1.0).panned(0.5));
	sound.on_clock_tick(ClockTime {
		clock: clock.id(),
		ticks: 1,
	});
	assert_eq!(sound.process(1.0), Frame::from_mono(2.0).panned(0.5));
	assert_eq!(sound.process(1.0), Frame::from_mono(4.0).panned(0.5));
}

/// Tests that a `StaticSound` outputs interpolated samples when
/// its playback position is between samples.
#[test]
fn interpolates_samples() {
	let data = StaticSoundData {
		sample_rate: 1,
		frames: Arc::new(vec![
			Frame::from_mono(0.0),
			Frame::from_mono(1.0),
			Frame::from_mono(1.0),
			Frame::from_mono(1.0),
			Frame::from_mono(1.0),
			Frame::from_mono(1.0),
			Frame::from_mono(-10.0),
		]),
		settings: Default::default(),
	};
	let (mut sound, _) = data.split();

	assert_eq!(sound.process(0.5), Frame::from_mono(0.0).panned(0.5));
	// at sample 0.5, the output should be somewhere between 0 and 1.
	// i don't care what exactly, that's up the to the interpolation algorithm.
	let frame = sound.process(5.0);
	assert!(frame.left > 0.0 && frame.left < 1.0);
	// at sample 5.5, the output should be between 1 and -10.
	let frame = sound.process(1.0);
	assert!(frame.left < 0.0 && frame.left > -10.0);
}

/// Tests that a `StaticSound` outputs interpolated samples correctly
/// when looping.
#[test]
fn interpolates_samples_when_looping() {
	let data = StaticSoundData {
		sample_rate: 1,
		frames: Arc::new(vec![Frame::from_mono(10.0), Frame::from_mono(9.0)]),
		settings: StaticSoundSettings::new().loop_behavior(LoopBehavior {
			start_position: 0.0,
		}),
	};
	let (mut sound, _) = data.split();
	sound.process(1.5);
	// because we're looping back to the first sample, which is 10.0,
	// the interpolated sample should be be tween 9.0 and 10.0
	let frame = sound.process(1.0);
	assert!(frame.left > 9.0 && frame.left < 10.0);
}

/// Tests that a `StaticSound` can seek to a position.
#[test]
fn seek_to() {
	let data = StaticSoundData {
		sample_rate: 1,
		frames: Arc::new((0..100).map(|i| Frame::from_mono(i as f32)).collect()),
		settings: StaticSoundSettings::new(),
	};
	let (mut sound, mut handle) = data.split();
	handle.seek_to(15.0).unwrap();
	sound.on_start_processing();
	expect_frame_soon(Frame::from_mono(15.0).panned(0.5), &mut sound);
}

/// Tests that a `StaticSound` can seek to a position past the end of
/// the sound when it's looping. The resulting position should be what
/// it would be (seek_point - duration) samples after playback reached
/// the end of the sound if it was playing normally..
#[test]
fn seek_to_while_looping() {
	let data = StaticSoundData {
		sample_rate: 1,
		frames: Arc::new((0..100).map(|i| Frame::from_mono(i as f32)).collect()),
		settings: StaticSoundSettings::new().loop_behavior(LoopBehavior {
			start_position: 5.0,
		}),
	};
	let (mut sound, mut handle) = data.split();
	handle.seek_to(120.0).unwrap();
	sound.on_start_processing();
	expect_frame_soon(Frame::from_mono(25.0).panned(0.5), &mut sound);
}

/// Tests that a `StaticSound` can seek by an amount of time.
#[test]
fn seek_by() {
	let data = StaticSoundData {
		sample_rate: 1,
		frames: Arc::new((0..100).map(|i| Frame::from_mono(i as f32)).collect()),
		settings: StaticSoundSettings::new().start_position(10.0),
	};
	let (mut sound, mut handle) = data.split();
	handle.seek_by(5.0).unwrap();
	sound.on_start_processing();
	// we wouldn't actually expect the position to be 10.0 seconds right at
	// this moment - the sound probably ran ahead a few samples to fill
	// the resample buffer. so let's just say it should reach 20.0 soon.
	expect_frame_soon(Frame::from_mono(20.0).panned(0.5), &mut sound);
}

fn expect_frame_soon(expected_frame: Frame, sound: &mut StaticSound) {
	const NUM_SAMPLES_TO_WAIT: usize = 10;
	for _ in 0..NUM_SAMPLES_TO_WAIT {
		let frame = sound.process(1.0);
		if frame == expected_frame {
			return;
		}
	}
	panic!(
		"Sound did not output frame with value {:?} within {} samples",
		expected_frame, NUM_SAMPLES_TO_WAIT
	);
}
