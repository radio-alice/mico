<script lang="ts">
  export let url, content, title
</script>

<style>
  div.article {
    flex-basis: var(--measure);
    flex-grow: 999;
    min-width: 50%;
    overflow-y: scroll;
    max-height: 100vh;
    height: max-content;
    line-height: var(--s1);
    padding: var(--s1);
    max-width: var(--measure);
  }
</style>

<div class="stack article">
  <h1>{title}</h1>
  <a href={url}>view online</a>
  <div class="stack">
    {@html content}
  </div>
</div>
