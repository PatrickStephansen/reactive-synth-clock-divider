// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// rust has a built-in for this but behind a feature flag
// use the native one if they get their shit together
fn clamp(min_value: f32, max_value: f32, value: f32) -> f32 {
	if value < min_value {
		return min_value;
	} else {
		if value > max_value {
			return max_value;
		} else {
			return value;
		}
	};
}

fn get_parameter(param: &Vec<f32>, min_value: f32, max_value: f32, index: usize) -> f32 {
	if param.len() > 1 {
		clamp(min_value, max_value, param[index])
	} else {
		if param.len() == 0 {
			clamp(min_value, max_value, 0.0)
		} else {
			clamp(min_value, max_value, param[0])
		}
	}
}

fn divide_clock_ticks(
	mut output_value: f32,
	mut ticks: f32,
	mut tocks: f32,
	open_after_ticks: f32,
	close_after_tocks: f32,
	ticks_on_reset: f32,
	tocks_on_reset: f32,
	clock_stage: InputGateStage,
	reset_stage: InputGateStage,
) -> (f32, f32, f32) {
	if reset_stage == InputGateStage::Opening {
		output_value = 0.0;
		ticks = ticks_on_reset;
		tocks = tocks_on_reset;
	}
	if clock_stage == InputGateStage::Opening && output_value <= 0.0 {
		ticks = ticks + 1.0;
		if ticks >= open_after_ticks {
			output_value = 1.0;
			ticks = ticks - open_after_ticks;
		}
	}
	if clock_stage == InputGateStage::Closing && output_value > 0.0 {
		tocks = tocks + 1.0;
		if tocks >= close_after_tocks {
			output_value = 0.0;
			tocks = tocks - close_after_tocks;
		}
	}

	return (output_value, ticks, tocks);
}

#[derive(Copy, Clone, PartialEq)]
#[repr(i32)]
pub enum InputGateStage {
	Opening = 1,
	Open = 2,
	Closing = 3,
	Closed = 4,
}

pub struct ClockDivider {
	clock_gate: Vec<f32>,
	reset_gate: Vec<f32>,
	open_after_ticks: Vec<f32>,
	close_after_tocks: Vec<f32>,
	ticks_on_reset: Vec<f32>,
	tocks_on_reset: Vec<f32>,
	render_quantum_samples: usize,
	output: Vec<f32>,
	ticks: f32,
	tocks: f32,
	clock_gate_stage: InputGateStage,
	reset_gate_stage: InputGateStage,
	output_gate: f32,
}

impl ClockDivider {
	pub fn new(render_quantum_samples: usize) -> ClockDivider {
		let mut output = Vec::with_capacity(render_quantum_samples);
		output.resize(render_quantum_samples, 0.0);
		ClockDivider {
			clock_gate: Vec::with_capacity(render_quantum_samples),
			reset_gate: Vec::with_capacity(render_quantum_samples),
			open_after_ticks: Vec::with_capacity(render_quantum_samples),
			close_after_tocks: Vec::with_capacity(render_quantum_samples),
			ticks_on_reset: Vec::with_capacity(render_quantum_samples),
			tocks_on_reset: Vec::with_capacity(render_quantum_samples),
			render_quantum_samples,
			output,
			ticks: 0.0,
			tocks: 0.0,
			clock_gate_stage: InputGateStage::Closed,
			reset_gate_stage: InputGateStage::Closed,
			output_gate: 0.0,
		}
	}

	pub fn process(
		&mut self,
		clock_gate_changed: unsafe extern "C" fn(bool),
		reset_gate_changed: unsafe extern "C" fn(bool),
	) {
		for sample_index in 0..self.render_quantum_samples {
			let clock_gate_value = get_parameter(&self.clock_gate, -1e9, 1e9, sample_index);
			let reset_gate_value = get_parameter(&self.reset_gate, -1e9, 1e9, sample_index);
			// TODO learn how to use pattern matching to make this less shit
			if clock_gate_value > 0.0 {
				if self.clock_gate_stage == InputGateStage::Open
					|| self.clock_gate_stage == InputGateStage::Opening
				{
					self.clock_gate_stage = InputGateStage::Open;
				} else {
					self.clock_gate_stage = InputGateStage::Opening;
					unsafe {
						clock_gate_changed(true);
					}
				}
			} else {
				if self.clock_gate_stage == InputGateStage::Open
					|| self.clock_gate_stage == InputGateStage::Opening
				{
					self.clock_gate_stage = InputGateStage::Closing;
					unsafe {
						clock_gate_changed(false);
					}
				} else {
					self.clock_gate_stage = InputGateStage::Closed;
				}
			}
			if reset_gate_value > 0.0 {
				if self.reset_gate_stage == InputGateStage::Open
					|| self.reset_gate_stage == InputGateStage::Opening
				{
					self.reset_gate_stage = InputGateStage::Open;
				} else {
					self.reset_gate_stage = InputGateStage::Opening;
					unsafe {
						reset_gate_changed(true);
					}
				}
			} else {
				if self.reset_gate_stage == InputGateStage::Open
					|| self.reset_gate_stage == InputGateStage::Opening
				{
					self.reset_gate_stage = InputGateStage::Closing;
					unsafe {
						reset_gate_changed(false);
					}
				} else {
					self.reset_gate_stage = InputGateStage::Closed;
				}
			}
			let (output, ticks, tocks) = divide_clock_ticks(
				self.output_gate,
				self.ticks,
				self.tocks,
				get_parameter(&self.open_after_ticks, 1.0, 1e9, sample_index),
				get_parameter(&self.close_after_tocks, 1.0, 1e9, sample_index),
				get_parameter(&self.ticks_on_reset, -1e9, 1e9, sample_index),
				get_parameter(&self.tocks_on_reset, -1e9, 1e9, sample_index),
				self.clock_gate_stage,
				self.reset_gate_stage,
			);
			self.output_gate = output;
			self.output[sample_index] = output;
			self.ticks = ticks;
			self.tocks = tocks;
		}
	}
}

#[link(wasm_import_module = "imports")]
extern "C" {
	fn clockChange(active: bool);
	fn resetChange(active: bool);
}

#[no_mangle]
pub unsafe extern "C" fn get_clock_gate_ptr(me: *mut ClockDivider) -> *mut f32 {
	(*me).clock_gate.as_mut_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn get_reset_gate_ptr(me: *mut ClockDivider) -> *mut f32 {
	(*me).reset_gate.as_mut_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn get_open_after_ticks_ptr(me: *mut ClockDivider) -> *mut f32 {
	(*me).open_after_ticks.as_mut_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn get_close_after_tocks_ptr(me: *mut ClockDivider) -> *mut f32 {
	(*me).close_after_tocks.as_mut_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn get_ticks_on_reset_ptr(me: *mut ClockDivider) -> *mut f32 {
	(*me).ticks_on_reset.as_mut_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn get_tocks_on_reset_ptr(me: *mut ClockDivider) -> *mut f32 {
	(*me).tocks_on_reset.as_mut_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn init(render_quantum_samples: i32) -> *mut ClockDivider {
	Box::into_raw(Box::new(ClockDivider::new(render_quantum_samples as usize)))
}

#[no_mangle]
pub unsafe extern "C" fn process_quantum(
	me: *mut ClockDivider,
	clock_gate_length: usize,
	reset_gate_length: usize,
	open_after_ticks_length: usize,
	close_after_tocks_length: usize,
	ticks_on_reset_length: usize,
	tocks_on_reset_length: usize,
) -> *const f32 {
	// the expectation is that the parameters are copied directly into memory before this is called
	// so fix the length if it changed
	(*me).clock_gate.set_len(clock_gate_length);
	(*me).reset_gate.set_len(reset_gate_length);
	(*me).open_after_ticks.set_len(open_after_ticks_length);
	(*me).close_after_tocks.set_len(close_after_tocks_length);
	(*me).ticks_on_reset.set_len(ticks_on_reset_length);
	(*me).tocks_on_reset.set_len(tocks_on_reset_length);
	(*me).process(clockChange, resetChange);
	(*me).output.as_ptr()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn basic_sanity() {
		let (out, ticks, tocks) = divide_clock_ticks(
			0.0,
			2.0,
			2.0,
			3.0,
			4.0,
			1.0,
			1.0,
			InputGateStage::Opening,
			InputGateStage::Closed,
		);
		assert_eq!(out, 1.0, "output");
		assert_eq!(ticks, 0.0, "ticks");
		assert_eq!(tocks, 2.0, "tocks");
	}

	#[test]
	fn reset_and_tick_simultaneously() {
		let (out, ticks, tocks) = divide_clock_ticks(
			0.0,
			0.0,
			0.0,
			3.0,
			1.0,
			2.0,
			0.0,
			InputGateStage::Opening,
			InputGateStage::Opening,
		);
		assert_eq!(out, 1.0, "output");
		assert_eq!(ticks, 0.0, "ticks");
		assert_eq!(tocks, 0.0, "tocks");
	}

	#[test]
	fn just_after_reset() {
		let (out, ticks, tocks) = divide_clock_ticks(
			1.0,
			0.0,
			0.0,
			3.0,
			1.0,
			2.0,
			0.0,
			InputGateStage::Open,
			InputGateStage::Open,
		);
		assert_eq!(out, 1.0, "output");
		assert_eq!(ticks, 0.0, "ticks");
		assert_eq!(tocks, 0.0, "tocks");
	}
}
