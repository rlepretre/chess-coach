<script lang="ts">
	import { page } from '$app/state';

	type Score = { cp: number } | { mate: number } | null;
	type Game = {
		headers: Record<string, string>;
		moves: string[];
		fens: string[];
		analysis: ({ score: Score } | null)[];
		analysis_done: boolean;
		analysis_error: string | null;
	};
	const PIECES: Record<string, string> = {
		K: '♔', Q: '♕', R: '♖', B: '♗', N: '♘', P: '♙',
		k: '♚', q: '♛', r: '♜', b: '♝', n: '♞', p: '♟'
	};

	let game = $state<Game | null>(null);
	let error = $state('');
	let ply = $state(0); // 0 = start position, n = after move n

	// Load the game, then keep polling while the engine is still working.
	$effect(() => {
		let timer: ReturnType<typeof setTimeout>;
		async function load() {
			const res = await fetch(`/api/games/${page.params.id}`);
			const body = await res.json();
			if (!res.ok) return (error = body.error);
			game = body;
			if (!body.analysis_done && !body.analysis_error) timer = setTimeout(load, 1000);
		}
		load();
		return () => clearTimeout(timer);
	});

	const score = $derived(game?.analysis[ply]?.score ?? null);
	// Lichess's win-chance curve: a pawn matters less when one side is already winning.
	const whitePercent = $derived(
		!score ? 50 : 'mate' in score ? (score.mate > 0 ? 100 : 0) : 50 + 50 * (2 / (1 + Math.exp(-0.00368208 * score.cp)) - 1)
	);
	const scoreLabel = $derived(
		!score ? '…' : 'mate' in score ? `#${Math.abs(score.mate)}` : (score.cp / 100).toFixed(1)
	);
	const analysed = $derived(game ? game.analysis.filter(Boolean).length : 0);

	// FEN piece placement -> 8 rows of 8 squares (empty string = empty square)
	const rows = $derived(
		game
			? game.fens[ply]
					.split(' ')[0]
					.split('/')
					.map((r) => [...r].flatMap((c) => (/\d/.test(c) ? Array(+c).fill('') : [c])))
			: []
	);

	function go(to: number) {
		if (game) ply = Math.max(0, Math.min(game.moves.length, to));
	}
	function onkey(e: KeyboardEvent) {
		if (e.key === 'ArrowLeft') go(ply - 1);
		else if (e.key === 'ArrowRight') go(ply + 1);
		else if (e.key === 'Home') go(0);
		else if (e.key === 'End') go(Infinity);
	}
</script>

<svelte:window onkeydown={onkey} />

{#if error}
	<p role="alert">{error}</p>
{:else if game}
	<h1>{game.headers.White ?? '?'} vs {game.headers.Black ?? '?'}</h1>
	<div class="layout">
		<div class="evalbar" role="img" aria-label="Evaluation {scoreLabel}" title={scoreLabel}>
			<div class="black" style="height: {100 - whitePercent}%"></div>
			<span>{scoreLabel}</span>
		</div>
		<div class="board" aria-label="Board">
			{#each rows as row, r}
				{#each row as piece, c}
					<div class="sq" class:dark={(r + c) % 2 === 1}>{PIECES[piece] ?? ''}</div>
				{/each}
			{/each}
		</div>
		<ol class="moves">
			{#each game.moves as san, i}
				{#if i % 2 === 0}<li class="num">{i / 2 + 1}.</li>{/if}
				<li>
					<button class:active={ply === i + 1} onclick={() => go(i + 1)}>{san}</button>
				</li>
			{/each}
		</ol>
	</div>
	{#if game.analysis_error}
		<p role="alert" style="color: crimson">Analysis failed: {game.analysis_error}</p>
	{:else if !game.analysis_done}
		<p>Analysing… {analysed}/{game.fens.length} positions</p>
	{/if}
	<p>
		<button onclick={() => go(0)}>⏮</button>
		<button onclick={() => go(ply - 1)}>◀</button>
		<button onclick={() => go(ply + 1)}>▶</button>
		<button onclick={() => go(Infinity)}>⏭</button>
	</p>
{:else}
	<p>Loading…</p>
{/if}

<style>
	.layout { display: flex; gap: 2rem; align-items: flex-start; }
	.evalbar { position: relative; width: 1.5rem; height: 28rem; background: #fff; border: 2px solid #444; }
	.evalbar .black { background: #333; transition: height 0.2s; }
	.evalbar span { position: absolute; bottom: 0; left: 0; right: 0; font-size: 0.65rem; text-align: center; color: #888; }
	.board { display: grid; grid-template-columns: repeat(8, 3.5rem); border: 2px solid #444; }
	.sq { width: 3.5rem; height: 3.5rem; display: grid; place-items: center; font-size: 2.4rem; background: #f0d9b5; color: #000; }
	.sq.dark { background: #b58863; }
	.moves { list-style: none; padding: 0; display: grid; grid-template-columns: 2.5rem 5rem 5rem; align-items: center; margin: 0; }
	.moves .num { color: #777; }
	.moves button { background: none; border: 0; cursor: pointer; font: inherit; }
	.moves button.active { background: #cde; font-weight: bold; }
</style>
