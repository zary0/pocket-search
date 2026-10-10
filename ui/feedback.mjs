// SPDX-License-Identifier: GPL-3.0-only
// A local, quiet 12 ms click. Unsupported/blocked audio never prevents typing.
export function committedEdit(before, after, inputType, composing = false) {
  return !composing && before !== after &&
    ['insertText', 'insertCompositionText', 'deleteContentBackward', 'deleteContentForward'].includes(inputType);
}
export function createInputClick(AudioContext = globalThis.AudioContext ?? globalThis.webkitAudioContext) {
  let context, last = -Infinity;
  return () => {
    if (!AudioContext) return;
    try {
      context ??= new AudioContext();
      if (context.state === 'suspended') {
        // Resume only in this input gesture; do not replay queued clicks later.
        void context.resume().catch(() => {});
        return;
      }
      if (context.state !== 'running' || context.currentTime - last < 0.03) return;
      const now = context.currentTime, oscillator = context.createOscillator(), gain = context.createGain();
      oscillator.type = 'triangle';
      oscillator.frequency.setValueAtTime(1800, now);
      oscillator.frequency.exponentialRampToValueAtTime(900, now + 0.012);
      gain.gain.setValueAtTime(0, now);
      gain.gain.linearRampToValueAtTime(0.035, now + 0.001);
      gain.gain.exponentialRampToValueAtTime(0.0001, now + 0.012);
      oscillator.connect(gain); gain.connect(context.destination);
      oscillator.onended = () => { oscillator.disconnect(); gain.disconnect(); };
      oscillator.start(now); oscillator.stop(now + 0.012);
      last = now;
    } catch { /* Audio is optional; preserve every input edit. */ }
  };
}
