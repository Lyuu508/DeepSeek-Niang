class CompanionMicrophone extends AudioWorkletProcessor {
  constructor() {
    super();
    this.batch = new Float32Array(1600);
    this.used = 0;
  }

  process(inputs) {
    const samples = inputs[0]?.[0];
    if (!samples) return true;
    for (const sample of samples) {
      this.batch[this.used++] = sample;
      if (this.used === this.batch.length) {
        this.port.postMessage(this.batch, [this.batch.buffer]);
        this.batch = new Float32Array(1600);
        this.used = 0;
      }
    }
    return true;
  }
}

registerProcessor('companion-microphone', CompanionMicrophone);
