use std::ffi::{c_double, c_float, c_int, c_uint, c_void};

pub(super) type State = *mut c_void;

pub(super) const PROCESS_OFFLINE: c_int = 0x0000_0000;
pub(super) const TRANSIENTS_CRISP: c_int = 0x0000_0000;
pub(super) const DETECTOR_PERCUSSIVE: c_int = 0x0000_0400;
pub(super) const CHANNELS_TOGETHER: c_int = 0x1000_0000;
pub(super) const ENGINE_FASTER: c_int = 0x0000_0000;
pub(super) const ENGINE_FINER: c_int = 0x2000_0000;

extern "C" {
    pub(super) fn rubberband_new(
        sample_rate: c_uint,
        channels: c_uint,
        options: c_int,
        initial_time_ratio: c_double,
        initial_pitch_scale: c_double,
    ) -> State;
    pub(super) fn rubberband_delete(state: State);
    pub(super) fn rubberband_set_expected_input_duration(state: State, samples: c_uint);
    pub(super) fn rubberband_study(
        state: State,
        input: *const *const c_float,
        samples: c_uint,
        final_block: c_int,
    );
    pub(super) fn rubberband_process(
        state: State,
        input: *const *const c_float,
        samples: c_uint,
        final_block: c_int,
    );
    pub(super) fn rubberband_available(state: State) -> c_int;
    pub(super) fn rubberband_retrieve(
        state: State,
        output: *const *mut c_float,
        samples: c_uint,
    ) -> c_uint;
    pub(super) fn rubberband_get_engine_version(state: State) -> c_int;
}
