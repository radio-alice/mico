<script lang="ts">
  import { onMount } from 'svelte'
  import { fade, fly } from 'svelte/transition'
  import { backOut } from 'svelte/easing'
  import toast from './toast'
  let visible, error, message
  const unsubscribe = toast.subscribe((values) => {
    ;({ visible, error, message } = values)
  })
</script>

<style>
  /* .error {
    background-color: var(--light2) !important;
  } */
  .toast {
    margin-top: var(--s0);
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    font-size: var(--s0);
    margin: 0 auto;
    z-index: 10;
    position: fixed;
    color: var(--dark1) !important;
    bottom: var(--s0);
  }
  button {
    cursor: pointer;
    right: 0;
    top: 0;
    font-size: var(--s0);
    flex: 0;
    padding: 0 var(--s-4);
    color: var(--light1);
    background-color: var(--dark2);
  }
  p {
    flex: 1;
    text-align: center;
    background-color: var(--light3);
    max-width: calc(var(--measure) / 2);
    padding: 0 var(--s0);
  }
</style>

{#if visible}
  <div
    class="toast row"
    class:error
    role="dialog"
    in:fly={{ delay: 0, duration: 300, x: 0, y: 50, opacity: 0.6, easing: backOut }}
    out:fade={{ duration: 300 }}>
    <p>{message}</p>
    <button on:click={toast.close}>ˣ</button>
  </div>
{/if}
