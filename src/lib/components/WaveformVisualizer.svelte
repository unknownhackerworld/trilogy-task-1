<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { audioLevel, pipelineState } from "../stores/pipeline";

  let canvas: HTMLCanvasElement;
  let animFrame: number;

  // Rolling buffer of level samples for the waveform
  const BUFFER_SIZE = 120;
  const buffer: number[] = new Array(BUFFER_SIZE).fill(0);
  let bufferIndex = 0;

  // Track last pushed level to animate smoothly even between events
  let currentLevel = 0;

  // Subscribe to audio level updates from pipeline
  const unsubLevel = audioLevel.subscribe((level) => {
    currentLevel = level;
  });

  function pushSample(value: number) {
    buffer[bufferIndex % BUFFER_SIZE] = value;
    bufferIndex++;
  }

  function draw() {
    if (!canvas) {
      animFrame = requestAnimationFrame(draw);
      return;
    }

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const W = canvas.width;
    const H = canvas.height;
    const midY = H / 2;
    const isRunning = $pipelineState === "running";

    // Push current level into rolling buffer every frame
    pushSample(isRunning ? currentLevel : 0);

    // Clear
    ctx.clearRect(0, 0, W, H);

    // Background
    ctx.fillStyle = "#111111";
    ctx.fillRect(0, 0, W, H);

    // Center line
    ctx.beginPath();
    ctx.strokeStyle = "#2a2a2a";
    ctx.lineWidth = 1;
    ctx.moveTo(0, midY);
    ctx.lineTo(W, midY);
    ctx.stroke();

    if (!isRunning) {
      // Draw flat idle line
      ctx.beginPath();
      ctx.strokeStyle = "#333333";
      ctx.lineWidth = 1.5;
      ctx.moveTo(0, midY);
      ctx.lineTo(W, midY);
      ctx.stroke();
      animFrame = requestAnimationFrame(draw);
      return;
    }

    // Draw waveform from rolling buffer
    const slotWidth = W / BUFFER_SIZE;

    ctx.beginPath();
    ctx.lineWidth = 1.5;

    for (let i = 0; i < BUFFER_SIZE; i++) {
      // Read from buffer in chronological order
      const idx = (bufferIndex + i) % BUFFER_SIZE;
      const level = buffer[idx];

      const x = i * slotWidth;
      // Oscillate: alternate up/down to look like a waveform
      const sign = i % 2 === 0 ? 1 : -1;
      const amplitude = level * (H / 2 - 4) * sign;
      const y = midY - amplitude;

      if (i === 0) {
        ctx.moveTo(x, midY);
      }
      ctx.lineTo(x + slotWidth / 2, y);
      ctx.lineTo(x + slotWidth, midY);
    }

    // Color based on level: green → amber → red
    const r = Math.min(255, Math.floor(currentLevel * 600));
    const g = Math.floor(200 - currentLevel * 150);
    const b = 50;
    ctx.strokeStyle = `rgb(${r},${g},${b})`;

    // Glow effect when loud
    if (currentLevel > 0.5) {
      ctx.shadowBlur = 6;
      ctx.shadowColor = `rgb(${r},${g},${b})`;
    } else {
      ctx.shadowBlur = 0;
    }

    ctx.stroke();

    animFrame = requestAnimationFrame(draw);
  }

  onMount(() => {
    // Match canvas pixel size to its CSS size for sharp rendering
    const resizeObserver = new ResizeObserver(() => {
      if (canvas) {
        canvas.width = canvas.offsetWidth;
        canvas.height = canvas.offsetHeight;
      }
    });
    resizeObserver.observe(canvas);
    canvas.width = canvas.offsetWidth;
    canvas.height = canvas.offsetHeight;

    animFrame = requestAnimationFrame(draw);

    return () => resizeObserver.disconnect();
  });

  onDestroy(() => {
    cancelAnimationFrame(animFrame);
    unsubLevel();
  });
</script>

<div class="waveform-container">
  <canvas bind:this={canvas} class="waveform-canvas"></canvas>
  <span class="label">
    {$pipelineState === "running" ? "Audio Input" : "Waiting..."}
  </span>
</div>

<style>
  .waveform-container {
    position: relative;
    width: 100%;
    height: 52px;
    border-radius: 6px;
    overflow: hidden;
    border: 1px solid #2a2a2a;
  }

  .waveform-canvas {
    width: 100%;
    height: 100%;
    display: block;
  }

  .label {
    position: absolute;
    top: 4px;
    right: 8px;
    font-size: 10px;
    color: #4b5563;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    pointer-events: none;
  }
</style>
