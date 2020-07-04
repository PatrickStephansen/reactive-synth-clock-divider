const bytesPerMemorySlot = 32 / 8;
const renderQuantumSampleCount = 128;

registerProcessor(
	"reactive-synth-clock-divider",
	class ClockDivider extends AudioWorkletProcessor {
		static get parameterDescriptors() {
			return [
				{
					name: "clockTrigger",
					defaultValue: 0,
					automationRate: "a-rate",
				},
				{
					name: "resetTrigger",
					defaultValue: 0,
					automationRate: "a-rate",
				},
				{
					name: "attackAfterTicks",
					defaultValue: 1,
					minValue: 1,
					maxValue: 1e9,
					automationRate: "a-rate",
				},
				{
					name: "releaseAfterTocks",
					defaultValue: 1,
					minValue: 1,
					maxValue: 1e9,
					automationRate: "a-rate",
				},
				{
					name: "ticksOnReset",
					defaultValue: 0,
					minValue: -1e9,
					maxValue: 1e9,
					automationRate: "a-rate",
				},
				{
					name: "tocksOnReset",
					defaultValue: 0,
					minValue: -1e9,
					maxValue: 1e9,
					automationRate: "a-rate",
				},
			];
		}
		constructor(options) {
			super(options);
			this.port.onmessage = this.handleMessage.bind(this);
			this.clockTriggerChangeMessage = {
				type: "clock-trigger-change",
				value: false,
			};
			this.resetTriggerChangeMessage = {
				type: "reset-trigger-change",
				value: false,
			};
			this.manualClockTriggerOn = false;
			this.manualResetTriggerOn = false;
			this.initialReset = true;
		}

		handleMessage(event) {
			if (event.data && event.data.type === "manual-clock-trigger") {
				this.manualClockTriggerOn = event.data.value;
			}
			if (event.data && event.data.type === "manual-reset-trigger") {
				this.manualResetTriggerOn = event.data.value;
			}
			if (event.data && event.data.type === "wasm") {
				this.initWasmModule(event.data.wasmModule).then(() =>
					this.port.postMessage({ type: "module-ready", value: true })
				);
			}
		}

		async initWasmModule(wasmModule) {
			this.wasmModule = await WebAssembly.instantiate(wasmModule, {
				imports: {
					clockChange: (t) => {
						this.clockTriggerChangeMessage.value = t;
						this.port.postMessage(this.clockTriggerChangeMessage);
					},
					resetChange: (t) => {
						this.resetTriggerChangeMessage.value = t;
						this.port.postMessage(this.resetTriggerChangeMessage);
					},
				},
			});
			this.internalProcessorPtr = this.wasmModule.exports.init(
				renderQuantumSampleCount
			);
			this.float32WasmMemory = new Float32Array(
				this.wasmModule.exports.memory.buffer
			);
		}

		process(_inputs, outputs, parameters) {
			if (this.wasmModule) {
				this.float32WasmMemory.set(
					this.manualClockTriggerOn || this.initialReset
						? [1]
						: parameters.clockTrigger,
					this.wasmModule.exports.get_clock_gate_ptr(
						this.internalProcessorPtr
					) / bytesPerMemorySlot
				);
				this.float32WasmMemory.set(
					this.manualResetTriggerOn ? [1] : parameters.resetTrigger,
					this.wasmModule.exports.get_reset_gate_ptr(
						this.internalProcessorPtr
					) / bytesPerMemorySlot
				);
				this.float32WasmMemory.set(
					parameters.attackAfterTicks,
					this.wasmModule.exports.get_open_after_ticks_ptr(
						this.internalProcessorPtr
					) / bytesPerMemorySlot
				);
				this.float32WasmMemory.set(
					parameters.releaseAfterTocks,
					this.wasmModule.exports.get_close_after_tocks_ptr(
						this.internalProcessorPtr
					) / bytesPerMemorySlot
				);
				this.float32WasmMemory.set(
					parameters.ticksOnReset,
					this.wasmModule.exports.get_ticks_on_reset_ptr(
						this.internalProcessorPtr
					) / bytesPerMemorySlot
				);
				this.float32WasmMemory.set(
					parameters.tocksOnReset,
					this.wasmModule.exports.get_tocks_on_reset_ptr(
						this.internalProcessorPtr
					) / bytesPerMemorySlot
				);
				const outputPointer =
					this.wasmModule.exports.process_quantum(
						this.internalProcessorPtr,
						this.manualClockTriggerOn ? 1 : parameters.clockTrigger.length,
						this.manualResetTriggerOn || this.initialReset
							? 1
							: parameters.resetTrigger.length,
						parameters.attackAfterTicks.length,
						parameters.releaseAfterTocks.length,
						parameters.ticksOnReset.length,
						parameters.tocksOnReset.length
					) / bytesPerMemorySlot;
				if (this.initialReset) {
					this.initialReset = false;
				}

				for (
					let channelIndex = 0;
					channelIndex < outputs[0].length;
					channelIndex++
				) {
					for (
						let sample = 0;
						sample < outputs[0][channelIndex].length;
						sample++
					) {
						outputs[0][channelIndex][sample] = this.float32WasmMemory[
							outputPointer + sample
						];
					}
				}
			}
			return true;
		}
	}
);
