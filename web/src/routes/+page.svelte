<script lang="ts">
	import { goto } from '$app/navigation';

	let pgn = $state('');
	let error = $state('');

	async function submit() {
		error = '';
		const res = await fetch('/api/games', {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ pgn })
		});
		const body = await res.json();
		if (res.ok) await goto(`/game/${body.id}`);
		else error = body.error;
	}
</script>

<h1>Chess Coach</h1>
<textarea bind:value={pgn} rows="12" cols="60" placeholder="Paste a PGN here"></textarea>
<p><button onclick={submit} disabled={!pgn.trim()}>Review game</button></p>
{#if error}<p role="alert" style="color: crimson">{error}</p>{/if}
