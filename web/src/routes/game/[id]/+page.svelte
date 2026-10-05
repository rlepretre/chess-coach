<script lang="ts">
	import { page } from '$app/state';

	type Game = { headers: Record<string, string>; moves: string[]; fens: string[] };
	const PIECES: Record<string, string> = {
		K: '♔', Q: '♕', R: '♖', B: '♗', N: '♘', P: '♙',
		k: '♚', q: '♛', r: '♜', b: '♝', n: '♞', p: '♟'
	};

	let game = $state<Game | null>(null);
	let error = $state('');
	let ply = $state(0); // 0 = start position, n = after move n

	$effect(() => {
		fetch(`/api/games/${page.params.id}`).then(async (res) => {
			const body = await res.json();
			if (res.ok) game = body;
			else error = body.error;
		});
	});

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
	.board { display: grid; grid-template-columns: repeat(8, 3.5rem); border: 2px solid #444; }
	.sq { width: 3.5rem; height: 3.5rem; display: grid; place-items: center; font-size: 2.4rem; background: #f0d9b5; color: #000; }
	.sq.dark { background: #b58863; }
	.moves { list-style: none; padding: 0; display: grid; grid-template-columns: 2.5rem 5rem 5rem; align-items: center; margin: 0; }
	.moves .num { color: #777; }
	.moves button { background: none; border: 0; cursor: pointer; font: inherit; }
	.moves button.active { background: #cde; font-weight: bold; }
</style>
