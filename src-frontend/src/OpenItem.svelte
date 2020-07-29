<script lang="ts">
  export let url, content, title
</script>

<style>
  h2 {
    line-height: var(--s2);
  }
  .stack {
    --space: var(--s1);
  }
</style>

<div class="stack main-content">
  <h2>{title}</h2>
  <a href={url}>view online</a>
  <div class="stack">
    {@html content}
  </div>
</div>
