<!--
  Embeds the Ну, погоди! wasm build in a SvelteKit page.

  Copy the release build into your app's static folder first:
    task web:export DEST=../my-site/static/nupogodi

  Then: <NuPogodi /> (or <NuPogodi base="/games/nupogodi" />).

  The game binds to its <canvas> once and has no teardown, so the canvas and the wasm module
  live for the whole page session. Unmounting this component only detaches the canvas (the
  game pauses itself while detached); mounting it again re-attaches the same canvas.
-->
<script module lang="ts">
	let canvas: HTMLCanvasElement | null = null;
	let started: Promise<void> | null = null;

	function getCanvas(base: string): HTMLCanvasElement {
		if (!canvas) {
			canvas = document.createElement('canvas');
			canvas.id = 'nupogodi';
			canvas.tabIndex = 0;
			canvas.dataset.assets = `${base}/assets`;
			canvas.setAttribute('aria-label', 'Nu, pogodi! handheld game');
		}
		return canvas;
	}

	function start(base: string): Promise<void> {
		started ??= import(/* @vite-ignore */ `${base}/nupogodi.js`).then((m) => m.default());
		return started;
	}
</script>

<script lang="ts">
	interface Props {
		/** URL path the build was exported to (no trailing slash). */
		base?: string;
		/** Focus the game on mount so the keyboard works right away. */
		autofocus?: boolean;
	}

	let { base = '/nupogodi', autofocus = false }: Props = $props();

	let error = $state<string | null>(null);

	/** Attach the shared canvas to `slot`, start the game once, detach on unmount. */
	function mount(slot: HTMLElement) {
		const c = getCanvas(base);
		slot.appendChild(c);
		if (autofocus) c.focus();
		// The canvas must be in the document before the game starts, as it looks it up by id.
		start(base).catch((e) => {
			// winit throws a control-flow exception once the event loop is running; that is expected.
			if (!String(e).includes('Using exceptions for control flow')) {
				error = String(e);
				started = null;
			}
		});
		return () => c.remove();
	}
</script>

<div class="nupogodi">
	<div class="slot" {@attach mount}></div>
	{#if error}
		<p class="error">Could not start the game: {error}</p>
	{/if}
</div>

<style>
	.nupogodi {
		position: relative;
		width: 100%;
		max-width: 1120px;
		aspect-ratio: 1120 / 672;
	}
	.slot {
		position: absolute;
		inset: 0;
	}
	.slot :global(canvas#nupogodi) {
		display: block;
		/* winit sets an inline size; the container decides instead. */
		width: 100% !important;
		height: 100% !important;
		outline: none;
		touch-action: none;
		user-select: none;
		-webkit-user-select: none;
		-webkit-touch-callout: none;
	}
	.error {
		position: absolute;
		inset: 0;
		margin: 0;
		display: grid;
		place-items: center;
		color: #b8322a;
		font: 14px system-ui, sans-serif;
	}
</style>
